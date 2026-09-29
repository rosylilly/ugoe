//! AI の API キーの保存と、外部の AI サービスに送る前の同意の IPC コマンド。
//! 方針: `docs/ai-integration.md#セキュリティとプライバシー`・`docs/editor.md#ai-の設定と送信の同意`。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。
//!
//! - API キーは OS の資格情報の保存先 (keyring) に置き、プロジェクトや普通のファイルには書かない。
//!   読む順は保存先 → 環境変数 `ANTHROPIC_API_KEY`。保存先が使えない環境 (WSL・ヘッドレスの Linux) では
//!   環境変数だけを使う。
//! - キーはログ・エラーの文言・IPC の応答に出さない。フロントエンドが知るのは [`KeyStatus`] だけ。
//! - 外部に送る IPC コマンドは [`prepare_claude`] を通す。プロジェクトが「外部に送らない」なら断り、
//!   同意がなければ送らずに [`AiReply::NeedsConsent`] を返す (フロントエンドが確認のダイアログを出して、
//!   [`ConsentAnswer`] を付けて呼び直す)。

use std::fmt;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;
use ugoe_ai::claude::{API_KEY_ENV, ApiKey, ClaudeConfig, ClaudeProvider, Effort};
use ugoe_ai::{AiError, Destination, Disclosure, Provider, ProviderRequest};

use crate::AppState;

/// 資格情報の保存先での名前 (サービス名・アカウント名)。
const KEYRING_SERVICE: &str = "ugoe-editor";
const KEYRING_USER: &str = "anthropic-api-key";

// ---- API キー ----

/// API キーを置く保存先。テストではメモリ上のものに差し替える。
pub trait SecretStore: Send + Sync {
    /// 保存したキー。なければ `None`。
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, secret: &str) -> Result<(), String>;
    /// 消す。もともとなければ何もしない。
    fn delete(&self) -> Result<(), String>;
}

/// OS の資格情報の保存先 (Linux は Secret Service、macOS はキーチェーン、Windows は資格情報マネージャー)。
struct KeyringStore {
    entry: keyring_core::Entry,
}

impl KeyringStore {
    fn open() -> Result<Self, String> {
        let store = platform_store().map_err(|e| describe(&e))?;
        let entry = store
            .build(KEYRING_SERVICE, KEYRING_USER, None)
            .map_err(|e| describe(&e))?;
        Ok(Self { entry })
    }
}

impl SecretStore for KeyringStore {
    fn get(&self) -> Result<Option<String>, String> {
        match self.entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(describe(&e)),
        }
    }

    fn set(&self, secret: &str) -> Result<(), String> {
        self.entry.set_password(secret).map_err(|e| describe(&e))
    }

    fn delete(&self) -> Result<(), String> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(describe(&e)),
        }
    }
}

#[cfg(target_os = "linux")]
fn platform_store() -> keyring_core::Result<Arc<keyring_core::CredentialStore>> {
    let store: Arc<keyring_core::CredentialStore> =
        dbus_secret_service_keyring_store::Store::new()?;
    Ok(store)
}

#[cfg(target_os = "macos")]
fn platform_store() -> keyring_core::Result<Arc<keyring_core::CredentialStore>> {
    let store: Arc<keyring_core::CredentialStore> =
        apple_native_keyring_store::keychain::Store::new()?;
    Ok(store)
}

#[cfg(target_os = "windows")]
fn platform_store() -> keyring_core::Result<Arc<keyring_core::CredentialStore>> {
    let store: Arc<keyring_core::CredentialStore> = windows_native_keyring_store::Store::new()?;
    Ok(store)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_store() -> keyring_core::Result<Arc<keyring_core::CredentialStore>> {
    Err(keyring_core::Error::NotSupportedByStore(
        "no credential store on this platform".into(),
    ))
}

/// 保存先のエラーの文言。読み出したデータ (キーの断片かもしれない) は含めない。
fn describe(error: &keyring_core::Error) -> String {
    use keyring_core::Error as E;
    match error {
        E::PlatformFailure(e) => format!("the credential store failed: {e}"),
        E::NoStorageAccess(e) => format!("cannot access the credential store: {e}"),
        E::NoEntry => "no API key is stored".into(),
        E::BadEncoding(_) | E::BadDataFormat(..) => "the stored API key is unreadable".into(),
        E::BadStoreFormat(_) => "the credential store is malformed".into(),
        E::TooLong(..) => "the API key is too long for the credential store".into(),
        E::NotSupportedByStore(reason) => format!("the credential store is unsupported: {reason}"),
        _ => "the credential store failed".into(),
    }
}

/// 資格情報の保存先の状態 (フロントエンドに見せるのはこれだけ。キーの中身は見せない)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KeyStatus {
    /// 保存先にキーがある。
    Stored,
    /// 保存先は使えるが、キーはない。
    NotStored,
    /// 保存先が使えない (WSL・ヘッドレスの Linux など)。環境変数だけを使う。
    Unavailable,
}

type Opener = Box<dyn Fn() -> Result<Box<dyn SecretStore>, String> + Send + Sync>;
type EnvReader = fn() -> Option<String>;

/// API キーの読み書き。保存先は最初に使うときに開く (D-Bus への接続などで起動を遅らせないため)。
pub struct ApiKeys {
    open: Opener,
    store: OnceLock<Result<Box<dyn SecretStore>, String>>,
    env: EnvReader,
}

impl fmt::Debug for ApiKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeys").finish_non_exhaustive()
    }
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self::new(
            Box::new(|| KeyringStore::open().map(|s| Box::new(s) as Box<dyn SecretStore>)),
            || std::env::var(API_KEY_ENV).ok(),
        )
    }
}

impl ApiKeys {
    pub fn new(open: Opener, env: EnvReader) -> Self {
        Self {
            open,
            store: OnceLock::new(),
            env,
        }
    }

    fn store(&self) -> Result<&dyn SecretStore, String> {
        match self.store.get_or_init(|| (self.open)()) {
            Ok(store) => Ok(store.as_ref()),
            Err(e) => Err(e.clone()),
        }
    }

    pub fn status(&self) -> KeyStatus {
        match self.store().and_then(SecretStore::get) {
            Ok(Some(_)) => KeyStatus::Stored,
            Ok(None) => KeyStatus::NotStored,
            Err(_) => KeyStatus::Unavailable,
        }
    }

    /// 環境変数 `ANTHROPIC_API_KEY` にキーがあるか。
    pub fn env_available(&self) -> bool {
        (self.env)().is_some_and(|v| !v.trim().is_empty())
    }

    /// 保存先に保存する (前後の空白は除く)。
    pub fn save(&self, key: &str) -> Result<(), String> {
        let key = key.trim();
        if key.is_empty() {
            return Err("the API key is empty".into());
        }
        self.store()?.set(key)
    }

    pub fn delete(&self) -> Result<(), String> {
        self.store()?.delete()
    }

    /// 使うキー: 保存先 → 環境変数の順。保存先が使えなければ環境変数だけを見る。
    pub fn resolve(&self) -> Result<ApiKey, AiError> {
        if let Ok(Some(key)) = self.store().and_then(SecretStore::get)
            && let Ok(key) = ApiKey::new(key)
        {
            return Ok(key);
        }
        match (self.env)() {
            Some(key) => ApiKey::new(key),
            None => Err(AiError::MissingApiKey(format!(
                "save a key in Settings or set {API_KEY_ENV}"
            ))),
        }
    }
}

// ---- 送信の同意 ----

/// 外部に送る内容 (確認のダイアログに示す)。`ugoe_ai::Disclosure` の外部に送る場合の写し。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DisclosureView {
    /// サービスの名前 (例: `Anthropic Claude API`)。
    pub service: String,
    /// 送り先の URL。
    pub endpoint: String,
    /// 使うモデル。
    pub model: String,
    pub image_count: usize,
    /// 画像の合計バイト数 (エンコード前)。
    pub image_bytes: usize,
    /// テキスト (システムプロンプトとプロンプト) の合計バイト数。
    pub text_bytes: usize,
}

impl DisclosureView {
    /// 外部に送らない (`Destination::Local`) なら `None`。
    pub fn new(disclosure: &Disclosure) -> Option<Self> {
        let Destination::External {
            service,
            endpoint,
            model,
        } = &disclosure.destination
        else {
            return None;
        };
        Some(Self {
            service: service.clone(),
            endpoint: endpoint.clone(),
            model: model.clone(),
            image_count: disclosure.image_count,
            image_bytes: disclosure.image_bytes,
            text_bytes: disclosure.text_bytes,
        })
    }

    fn destination(&self) -> RememberedDestination {
        RememberedDestination {
            service: self.service.clone(),
            endpoint: self.endpoint.clone(),
            model: self.model.clone(),
        }
    }
}

/// 確認のダイアログでの利用者の答え。送る IPC コマンドに付けて呼び直す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ConsentAnswer {
    /// 同意した内容 (ダイアログに示したもの)。今送ろうとしている内容と違えば、もう一度確かめる。
    pub disclosure: DisclosureView,
    /// 「今後は確かめない」: このプロジェクトを閉じるかエディタを終えるまで、同じ送り先
    /// (サービス・URL・モデル) には確かめずに送る。ファイルには残さない。
    pub remember: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RememberedDestination {
    service: String,
    endpoint: String,
    model: String,
}

/// プロジェクトを開いている間だけ覚えておく同意 (`ProjectSession` が持つ。プロジェクトを替えると消える)。
#[derive(Debug, Default)]
pub struct ConsentMemory {
    remembered: Option<RememberedDestination>,
}

impl ConsentMemory {
    pub fn is_remembered(&self) -> bool {
        self.remembered.is_some()
    }

    pub fn forget(&mut self) {
        self.remembered = None;
    }
}

/// [`check_consent`] の結果。
#[derive(Debug, PartialEq, Eq)]
pub enum Gate {
    /// 送ってよい (外部に出ない・同意がある・「今後は確かめない」の送り先)。
    Proceed,
    /// 利用者に確かめる。
    Ask(DisclosureView),
}

/// プロジェクトが「外部に送らない」のときの断りの文言。
pub const SEND_DISABLED: &str =
    "this project is set not to send data to external AI services (see Settings)";

/// 送る前の確認。`allowed` はプロジェクトの「外部に送ってよい」
/// (`ProjectMeta::ai_send_externally`)。送ってよくなければ、外部に出る送信はすべて断る。
pub fn check_consent(
    allowed: bool,
    memory: &mut ConsentMemory,
    disclosure: &Disclosure,
    answer: Option<&ConsentAnswer>,
) -> Result<Gate, String> {
    let Some(view) = DisclosureView::new(disclosure) else {
        return Ok(Gate::Proceed);
    };
    if !allowed {
        return Err(SEND_DISABLED.into());
    }
    if let Some(answer) = answer
        && answer.disclosure == view
    {
        if answer.remember {
            memory.remembered = Some(view.destination());
        }
        return Ok(Gate::Proceed);
    }
    if memory.remembered.as_ref() == Some(&view.destination()) {
        return Ok(Gate::Proceed);
    }
    Ok(Gate::Ask(view))
}

/// 外部の AI に送る IPC コマンドの戻り値。
#[derive(Debug, Serialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AiReply<T> {
    /// 送って、結果を得た。
    Done { value: T },
    /// 送っていない。`disclosure` を示して同意を得てから、`ConsentAnswer` を付けて呼び直す。
    NeedsConsent { disclosure: DisclosureView },
}

/// [`prepare_claude`] の結果。
#[derive(Debug)]
pub enum Prepared {
    /// 送ってよい。このプロバイダで送る。
    Ready(ClaudeProvider),
    NeedsConsent(DisclosureView),
}

/// 外部の AI (Claude API) に `request` を送る前の準備。プロジェクトの設定・API キー・同意を確かめる。
/// 送る IPC コマンドは必ずこれを通し、[`Prepared::NeedsConsent`] なら送らずに
/// [`AiReply::NeedsConsent`] を返す。ロックは返る前に外す (送信中に編集を止めないため)。
pub fn prepare_claude(
    state: &AppState,
    config: ClaudeConfig,
    request: &ProviderRequest,
    consent: Option<&ConsentAnswer>,
) -> Result<Prepared, String> {
    if !lock_project(state)?.ai_send_externally() {
        return Err(SEND_DISABLED.into());
    }
    let key = state.api_keys.resolve().map_err(|e| e.to_string())?;
    let provider = ClaudeProvider::new(config, key);
    let disclosure = provider.disclosure(request);
    let mut session = lock_project(state)?;
    let allowed = session.ai_send_externally();
    match check_consent(allowed, session.ai_consent_mut(), &disclosure, consent)? {
        Gate::Proceed => Ok(Prepared::Ready(provider)),
        Gate::Ask(view) => Ok(Prepared::NeedsConsent(view)),
    }
}

// ---- IPC ----

/// 設定の画面に出す AI の状態。キーの中身は含めない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct AiSettings {
    /// 資格情報の保存先の状態。
    pub key: KeyStatus,
    /// 環境変数 `ANTHROPIC_API_KEY` にキーがあるか (保存先にキーがなければこちらを使う)。
    pub env_key: bool,
    /// 開いているプロジェクトのデータを外部の AI サービスに送ってよいか。
    pub send_externally: bool,
    /// このプロジェクトで「今後は確かめない」を選んだか (エディタを終えるかプロジェクトを替えると消える)。
    pub consent_remembered: bool,
}

fn lock_project(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, crate::project::ProjectSession>, String> {
    state
        .project
        .lock()
        .map_err(|_| "project lock poisoned".to_string())
}

fn settings(state: &AppState) -> Result<AiSettings, String> {
    let key = state.api_keys.status();
    let env_key = state.api_keys.env_available();
    let session = lock_project(state)?;
    Ok(AiSettings {
        key,
        env_key,
        send_externally: session.ai_send_externally(),
        consent_remembered: session.ai_consent().is_remembered(),
    })
}

/// 保存先の操作は時間がかかることがある (D-Bus の応答待ちなど) ので、画面を止めないよう別のスレッドで行う。
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    task: impl FnOnce(&AppState) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || task(&app.state::<AppState>()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ai_settings(app: AppHandle) -> Result<AiSettings, String> {
    blocking(app, settings).await
}

/// API キーを資格情報の保存先に保存する。キーは応答に含めない。
#[tauri::command]
pub async fn store_api_key(app: AppHandle, key: String) -> Result<AiSettings, String> {
    blocking(app, move |state| {
        state.api_keys.save(&key)?;
        settings(state)
    })
    .await
}

#[tauri::command]
pub async fn delete_api_key(app: AppHandle) -> Result<AiSettings, String> {
    blocking(app, |state| {
        state.api_keys.delete()?;
        settings(state)
    })
    .await
}

/// 開いているプロジェクトの「外部の AI に送ってよいか」を変える (プロジェクトを保存すると残る)。
/// 送らないにすると「今後は確かめない」も忘れる。
#[tauri::command]
pub async fn set_ai_send_externally(app: AppHandle, allowed: bool) -> Result<AiSettings, String> {
    blocking(app, move |state| {
        lock_project(state)?.set_ai_send_externally(allowed);
        settings(state)
    })
    .await
}

/// 「今後は確かめない」を取り消す (次に送るときにまた確かめる)。
#[tauri::command]
pub async fn forget_ai_consent(app: AppHandle) -> Result<AiSettings, String> {
    blocking(app, |state| {
        lock_project(state)?.ai_consent_mut().forget();
        settings(state)
    })
    .await
}

/// 接続の確認で送る小さなリクエスト (画像なし)。
fn test_request() -> (ClaudeConfig, ProviderRequest, serde_json::Value) {
    let config = ClaudeConfig {
        max_tokens: 1024,
        effort: Some(Effort::Low),
        ..ClaudeConfig::default()
    };
    let request = ProviderRequest {
        system: "You check that the API connection works.".into(),
        prompt: "Reply with {\"ok\": true}.".into(),
        images: Vec::new(),
    };
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "ok": { "type": "boolean" } },
        "required": ["ok"],
        "additionalProperties": false,
    });
    (config, request, schema)
}

/// API キーと接続を確かめる (画像を含まない小さなリクエストを送る)。送る前に同意を求める。
#[tauri::command]
pub async fn test_ai_connection(
    app: AppHandle,
    consent: Option<ConsentAnswer>,
) -> Result<AiReply<()>, String> {
    blocking(app, move |state| {
        let (config, request, schema) = test_request();
        match prepare_claude(state, config, &request, consent.as_ref())? {
            Prepared::NeedsConsent(disclosure) => Ok(AiReply::NeedsConsent { disclosure }),
            Prepared::Ready(provider) => {
                provider
                    .complete_json(&request, &schema)
                    .map_err(|e| e.to_string())?;
                Ok(AiReply::Done { value: () })
            }
        }
    })
    .await
}

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
