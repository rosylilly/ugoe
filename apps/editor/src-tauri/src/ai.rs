//! AI のプロバイダの選択・API キーの保存と、外部の AI サービスに送る前の同意の IPC コマンド。
//! 方針: `docs/ai-integration.md#セキュリティとプライバシー`・`docs/editor.md#ai-の設定と送信の同意`・ADR 0022。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。
//!
//! - プロバイダ (Claude / OpenAI 互換) とモデル・ベースの URL は [`crate::ai_provider`] が持つ。
//! - API キーはプロバイダごとに OS の資格情報の保存先 (keyring) に置き ([`KeySlot`])、プロジェクトや
//!   普通のファイルには書かない。読む順は保存先 → 環境変数 (Claude は `ANTHROPIC_API_KEY`、OpenAI は
//!   `OPENAI_API_KEY`)。保存先が使えない環境 (WSL・ヘッドレスの Linux) では環境変数だけを使う。
//! - キーはログ・エラーの文言・IPC の応答に出さない。フロントエンドが知るのは [`KeyStatus`] だけ。
//! - AI に送る IPC コマンドは、プロバイダに関わらず [`prepare_requests`] を通す。外部に出るなら、
//!   プロジェクトが「外部に送らない」なら断り、同意がなければ送らずに [`AiReply::NeedsConsent`] を返す
//!   (フロントエンドが確認のダイアログを出して、[`ConsentAnswer`] を付けて呼び直す)。
//!   送り先が手元 (ループバック) なら外部に出ないので、同意を求めない。

use std::fmt;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use ts_rs::TS;
use ugoe_ai::claude::ApiKey;
use ugoe_ai::{AiError, Destination, Disclosure, Provider, ProviderRequest};

use crate::AppState;
pub use crate::ai_provider::KeySlot;
use crate::ai_provider::{ProviderSettings, Purpose};
use crate::commands::blocking;
use crate::error::{EditorError, IpcResult};

/// 資格情報の保存先での名前 (サービス名)。アカウント名はプロバイダごと ([`KeySlot::account`])。
const KEYRING_SERVICE: &str = "ugoe-editor";

// ---- API キー ----

/// API キーを置く保存先。`account` はプロバイダごとの名前 ([`KeySlot::account`])。
/// テストではメモリ上のものに差し替える。
pub trait SecretStore: Send + Sync {
    /// 保存したキー。なければ `None`。
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, secret: &str) -> Result<(), String>;
    /// 消す。もともとなければ何もしない。
    fn delete(&self, account: &str) -> Result<(), String>;
}

/// OS の資格情報の保存先 (Linux は Secret Service、macOS はキーチェーン、Windows は資格情報マネージャー)。
struct KeyringStore {
    store: Arc<keyring_core::CredentialStore>,
}

impl KeyringStore {
    fn open() -> Result<Self, String> {
        let store = platform_store().map_err(|e| describe(&e))?;
        Ok(Self { store })
    }

    fn entry(&self, account: &str) -> Result<keyring_core::Entry, String> {
        self.store
            .build(KEYRING_SERVICE, account, None)
            .map_err(|e| describe(&e))
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        match self.entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(describe(&e)),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        self.entry(account)?
            .set_password(secret)
            .map_err(|e| describe(&e))
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match self.entry(account)?.delete_credential() {
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
/// 環境変数を読む (テストでは差し替える)。
type EnvReader = fn(&str) -> Option<String>;

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
            |name| std::env::var(name).ok(),
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

    pub fn status(&self, slot: &KeySlot) -> KeyStatus {
        match self.store().and_then(|s| s.get(&slot.account)) {
            Ok(Some(_)) => KeyStatus::Stored,
            Ok(None) => KeyStatus::NotStored,
            Err(_) => KeyStatus::Unavailable,
        }
    }

    /// `slot` の環境変数にキーがあるか。
    pub fn env_available(&self, slot: &KeySlot) -> bool {
        self.env_value(slot).is_some()
    }

    fn env_value(&self, slot: &KeySlot) -> Option<String> {
        slot.env.and_then(self.env).filter(|v| !v.trim().is_empty())
    }

    /// 保存先に保存する (前後の空白は除く)。
    pub fn save(&self, slot: &KeySlot, key: &str) -> Result<(), EditorError> {
        let key = key.trim();
        if key.is_empty() {
            return Err(EditorError::EmptyApiKey);
        }
        self.store()
            .and_then(|s| s.set(&slot.account, key))
            .map_err(EditorError::CredentialStore)
    }

    pub fn delete(&self, slot: &KeySlot) -> Result<(), EditorError> {
        self.store()
            .and_then(|s| s.delete(&slot.account))
            .map_err(EditorError::CredentialStore)
    }

    /// 使うキー: 保存先 → 環境変数の順。保存先が使えなければ環境変数だけを見る。どちらにもなければ `None`。
    pub fn find(&self, slot: &KeySlot) -> Option<ApiKey> {
        if let Ok(Some(key)) = self.store().and_then(|s| s.get(&slot.account))
            && let Ok(key) = ApiKey::new(key)
        {
            return Some(key);
        }
        self.env_value(slot).and_then(|k| ApiKey::new(k).ok())
    }

    /// [`Self::find`] の、キーがなければエラーにする版。
    pub fn resolve(&self, slot: &KeySlot) -> Result<ApiKey, AiError> {
        self.find(slot).ok_or_else(|| {
            AiError::MissingApiKey(match slot.env {
                Some(env) => format!("save a key in Settings or set {env}"),
                None => "save a key in Settings".into(),
            })
        })
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
    /// 答えが検証を通らなかったときに、同じ画像と前の答え・理由を添えて送り直すことがある回数の上限
    /// (上の数は 1 回分。`ugoe_ai::rig::propose_rig_with_ai` の問い直し)。
    pub retries: usize,
    /// テキストに含まれる、利用者の注文の文字数 (`ugoe_ai::instructions::Instructions`)。注文がなければ 0。
    /// 分かれたリクエストには、どれにも同じ注文が入る。
    pub instruction_chars: usize,
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
            retries: 0,
            instruction_chars: 0,
        })
    }

    /// 複数のリクエストをまとめたもの (枚数・バイト数は合計)。送り先の違うものは混ぜない。
    /// すべて外部に送らないなら `None`。
    pub fn combined(disclosures: &[Disclosure]) -> Result<Option<Self>, EditorError> {
        let mut total: Option<Self> = None;
        for view in disclosures.iter().filter_map(Self::new) {
            match &mut total {
                None => total = Some(view),
                Some(t) if t.destination() == view.destination() => {
                    t.image_count += view.image_count;
                    t.image_bytes += view.image_bytes;
                    t.text_bytes += view.text_bytes;
                }
                Some(_) => return Err(EditorError::MixedDestinations),
            }
        }
        Ok(total)
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

/// 送る前の確認。`allowed` はプロジェクトの「外部に送ってよい」
/// (`ProjectMeta::ai_send_externally`)。送ってよくなければ、外部に出る送信はすべて断る。
/// `view` は送る内容 ([`DisclosureView::new`]。外部に出なければ `None`)。
pub fn check_consent(
    allowed: bool,
    memory: &mut ConsentMemory,
    view: Option<DisclosureView>,
    answer: Option<&ConsentAnswer>,
) -> Result<Gate, EditorError> {
    let Some(view) = view else {
        return Ok(Gate::Proceed);
    };
    if !allowed {
        return Err(EditorError::SendDisabled);
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

/// [`prepare_requests`] の結果。
pub enum Prepared {
    /// 送ってよい。このプロバイダで送る。
    Ready(Box<dyn Provider>),
    NeedsConsent(DisclosureView),
}

impl fmt::Debug for Prepared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready(_) => f.write_str("Ready"),
            Self::NeedsConsent(view) => f.debug_tuple("NeedsConsent").field(view).finish(),
        }
    }
}

/// 選んだプロバイダで AI に `requests` を送る前の準備。プロジェクトの設定・API キー・同意を確かめる。
/// 送る IPC コマンドは必ずこれを通し、[`Prepared::NeedsConsent`] なら送らずに [`AiReply::NeedsConsent`] を
/// 返す。複数のリクエストは 1 回の確認で合計を示す (レイヤーの分類を分けて送るなど)。
/// `retries` は答えが検証を通らなかったときに呼び出し側が送り直すことがある回数の上限。確認に示す回数
/// ([`DisclosureView::retries`]) には、プロバイダ自身の問い直し (OpenAI 互換の API で構造化出力を
/// 使わないとき) も含める。ロックは返る前に外す (送信中に編集を止めないため)。
pub fn prepare_requests(
    state: &AppState,
    purpose: Purpose,
    requests: &[ProviderRequest],
    retries: usize,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<Prepared> {
    prepare_instructed(state, purpose, requests, retries, 0, consent)
}

/// [`prepare_requests`] の、利用者の注文を含むリクエストの版。確認に注文の文字数
/// ([`DisclosureView::instruction_chars`]) を示す。
pub fn prepare_instructed(
    state: &AppState,
    purpose: Purpose,
    requests: &[ProviderRequest],
    retries: usize,
    instruction_chars: usize,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<Prepared> {
    let settings = state.ai_provider.get();
    prepare_with(
        state,
        &settings,
        purpose,
        requests,
        retries,
        instruction_chars,
        consent,
    )
}

fn prepare_with(
    state: &AppState,
    settings: &ProviderSettings,
    purpose: Purpose,
    requests: &[ProviderRequest],
    retries: usize,
    instruction_chars: usize,
    consent: Option<&ConsentAnswer>,
) -> IpcResult<Prepared> {
    if !settings.is_local() && !lock_project(state)?.ai_send_externally() {
        return Err(EditorError::SendDisabled.into());
    }
    let slot = settings.key_slot();
    let key = if slot.required {
        Some(state.api_keys.resolve(&slot).map_err(EditorError::Ai)?)
    } else {
        state.api_keys.find(&slot)
    };
    let provider = settings.build(key, purpose)?;
    let json_retries = settings.json_retries();
    let retries = (retries + 1) * (json_retries + 1) - 1;
    let disclosures: Vec<Disclosure> = requests.iter().map(|r| provider.disclosure(r)).collect();
    let view = DisclosureView::combined(&disclosures)?.map(|v| DisclosureView {
        retries,
        instruction_chars,
        ..v
    });
    let mut session = lock_project(state)?;
    let allowed = session.ai_send_externally();
    match check_consent(allowed, session.ai_consent_mut(), view, consent)? {
        Gate::Proceed => Ok(Prepared::Ready(provider)),
        Gate::Ask(view) => Ok(Prepared::NeedsConsent(view)),
    }
}

// ---- IPC ----

/// 設定の画面に出す AI の状態。キーの中身は含めない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct AiSettings {
    /// 選んだプロバイダ・モデル・ベースの URL (エディタ全体の設定)。
    pub provider: ProviderSettings,
    /// 利用者に示す送り先のサービスの名前。
    pub service: String,
    /// 送り先がこの計算機 (ループバック) か。外部に出ないので同意を求めず、「外部に送らない」でも使える。
    pub local: bool,
    /// 選んだプロバイダの API キーの、資格情報の保存先での状態。
    pub key: KeyStatus,
    /// 選んだプロバイダがキーなしでは送れないか (手元のサーバなどはキーが要らない)。
    pub key_required: bool,
    /// 保存先にキーがないときに読む環境変数 (例: `ANTHROPIC_API_KEY`)。なければ `null`。
    pub key_env: Option<String>,
    /// その環境変数にキーがあるか (保存先にキーがなければこちらを使う)。
    pub env_key: bool,
    /// 開いているプロジェクトのデータを外部の AI サービスに送ってよいか。
    pub send_externally: bool,
    /// このプロジェクトで「今後は確かめない」を選んだか (エディタを終えるかプロジェクトを替えると消える)。
    pub consent_remembered: bool,
}

pub(crate) fn lock_project(
    state: &AppState,
) -> IpcResult<std::sync::MutexGuard<'_, crate::project::ProjectSession>> {
    state
        .project
        .lock()
        .map_err(|_| EditorError::LockPoisoned { what: "project" }.into())
}

fn settings(state: &AppState) -> IpcResult<AiSettings> {
    let provider = state.ai_provider.get();
    let slot = provider.key_slot();
    let key = state.api_keys.status(&slot);
    let env_key = state.api_keys.env_available(&slot);
    let session = lock_project(state)?;
    Ok(AiSettings {
        service: provider.service(),
        local: provider.is_local(),
        key,
        key_required: slot.required,
        key_env: slot.env.map(str::to_owned),
        env_key,
        provider,
        send_externally: session.ai_send_externally(),
        consent_remembered: session.ai_consent().is_remembered(),
    })
}

#[tauri::command]
pub async fn ai_settings(app: AppHandle) -> IpcResult<AiSettings> {
    blocking(app, settings).await
}

/// プロバイダ・モデル・ベースの URL を変え、アプリの設定のディレクトリに保存する。
/// 送り先が変わるので、「今後は確かめない」は次の送信で確かめ直す (覚えた送り先と違うため)。
#[tauri::command]
pub async fn set_ai_provider(app: AppHandle, provider: ProviderSettings) -> IpcResult<AiSettings> {
    blocking(app, move |state| {
        state.ai_provider.set(provider)?;
        settings(state)
    })
    .await
}

/// 選んでいるプロバイダの API キーを資格情報の保存先に保存する。キーは応答に含めない。
#[tauri::command]
pub async fn store_api_key(app: AppHandle, key: String) -> IpcResult<AiSettings> {
    blocking(app, move |state| {
        state
            .api_keys
            .save(&state.ai_provider.get().key_slot(), &key)?;
        settings(state)
    })
    .await
}

/// 選んでいるプロバイダの API キーを消す。
#[tauri::command]
pub async fn delete_api_key(app: AppHandle) -> IpcResult<AiSettings> {
    blocking(app, |state| {
        state.api_keys.delete(&state.ai_provider.get().key_slot())?;
        settings(state)
    })
    .await
}

/// 開いているプロジェクトの「外部の AI に送ってよいか」を変える (プロジェクトを保存すると残る)。
/// 送らないにすると「今後は確かめない」も忘れる。
#[tauri::command]
pub async fn set_ai_send_externally(app: AppHandle, allowed: bool) -> IpcResult<AiSettings> {
    blocking(app, move |state| {
        lock_project(state)?.set_ai_send_externally(allowed);
        settings(state)
    })
    .await
}

/// 「今後は確かめない」を取り消す (次に送るときにまた確かめる)。
#[tauri::command]
pub async fn forget_ai_consent(app: AppHandle) -> IpcResult<AiSettings> {
    blocking(app, |state| {
        lock_project(state)?.ai_consent_mut().forget();
        settings(state)
    })
    .await
}

/// 接続の確認で送る小さなリクエスト (画像なし)。
fn test_request() -> (ProviderRequest, serde_json::Value) {
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
    (request, schema)
}

/// 選んでいるプロバイダの API キーと接続を確かめる (画像を含まない小さなリクエストを送る)。
/// 外部に送るなら、送る前に同意を求める。
#[tauri::command]
pub async fn test_ai_connection(
    app: AppHandle,
    consent: Option<ConsentAnswer>,
) -> IpcResult<AiReply<()>> {
    blocking(app, move |state| {
        let (request, schema) = test_request();
        let requests = std::slice::from_ref(&request);
        match prepare_requests(
            state,
            Purpose::ConnectionTest,
            requests,
            0,
            consent.as_ref(),
        )? {
            Prepared::NeedsConsent(disclosure) => Ok(AiReply::NeedsConsent { disclosure }),
            Prepared::Ready(provider) => {
                provider
                    .complete_json(&request, &schema)
                    .map_err(EditorError::Ai)?;
                Ok(AiReply::Done { value: () })
            }
        }
    })
    .await
}

#[cfg(test)]
#[path = "ai_tests.rs"]
pub(crate) mod tests;
