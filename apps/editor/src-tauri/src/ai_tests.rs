use std::collections::HashMap;
use std::sync::Mutex;

use ugoe_ai::FakeProvider;
use ugoe_editor_core::Project;

use super::*;
use crate::ai_provider::{OutputMode, ProviderKind, ProviderStore};

type Secrets = Arc<Mutex<HashMap<String, String>>>;

/// メモリ上の保存先 (アカウントごと)。`fail` なら使えない保存先として振る舞う。
#[derive(Default)]
struct MemoryStore {
    secrets: Secrets,
    fail: bool,
}

impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        if self.fail {
            return Err("no secret service".into());
        }
        Ok(self.secrets.lock().unwrap().get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        if self.fail {
            return Err("no secret service".into());
        }
        self.secrets
            .lock()
            .unwrap()
            .insert(account.into(), secret.into());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        if self.fail {
            return Err("no secret service".into());
        }
        self.secrets.lock().unwrap().remove(account);
        Ok(())
    }
}

fn keys(secrets: Secrets, env: EnvReader) -> ApiKeys {
    ApiKeys::new(
        Box::new(move || {
            Ok(Box::new(MemoryStore {
                secrets: secrets.clone(),
                fail: false,
            }) as Box<dyn SecretStore>)
        }),
        env,
    )
}

fn no_env(_: &str) -> Option<String> {
    None
}

fn env_key(name: &str) -> Option<String> {
    (name == "ANTHROPIC_API_KEY" || name == "OPENAI_API_KEY").then(|| "sk-env".into())
}

fn claude_slot() -> KeySlot {
    ProviderSettings::default().key_slot()
}

fn openai(base_url: &str) -> ProviderSettings {
    let mut settings = ProviderSettings {
        provider: ProviderKind::OpenaiCompatible,
        ..ProviderSettings::default()
    };
    settings.openai.base_url = base_url.into();
    settings
}

#[test]
fn keys_are_saved_trimmed_and_deleted() {
    let secrets = Secrets::default();
    let keys = keys(secrets.clone(), no_env);
    let slot = claude_slot();
    assert_eq!(keys.status(&slot), KeyStatus::NotStored);
    assert!(keys.resolve(&slot).is_err());
    assert!(keys.save(&slot, "  ").is_err());

    keys.save(&slot, "  sk-stored \n").unwrap();
    // 前からの保存先の名前のまま (T0638 で保存したキーをそのまま使う)
    assert_eq!(
        secrets
            .lock()
            .unwrap()
            .get("anthropic-api-key")
            .map(String::as_str),
        Some("sk-stored")
    );
    assert_eq!(keys.status(&slot), KeyStatus::Stored);
    assert!(keys.resolve(&slot).is_ok());

    keys.delete(&slot).unwrap();
    keys.delete(&slot).unwrap();
    assert_eq!(keys.status(&slot), KeyStatus::NotStored);
}

/// キーはプロバイダごと、OpenAI 互換の API は送り先のホストごとに分ける。
/// `OPENAI_API_KEY` は OpenAI に送るときだけ使う。
#[test]
fn keys_are_kept_per_provider_and_host() {
    let secrets = Secrets::default();
    let keys = keys(secrets.clone(), env_key);
    let openai_slot = openai("https://api.openai.com/v1").key_slot();
    let router_slot = openai("https://openrouter.ai/api/v1").key_slot();
    let local_slot = openai("http://localhost:11434/v1").key_slot();
    assert_eq!(openai_slot.account, "openai-compatible:api.openai.com");
    assert_eq!(router_slot.account, "openai-compatible:openrouter.ai");
    assert!(!openai_slot.required && !local_slot.required);
    assert!(claude_slot().required);

    keys.save(&router_slot, "sk-or-1").unwrap();
    assert_eq!(keys.status(&router_slot), KeyStatus::Stored);
    assert_eq!(keys.status(&openai_slot), KeyStatus::NotStored);
    assert_eq!(keys.status(&claude_slot()), KeyStatus::NotStored);
    // OpenAI には環境変数のキー、それ以外の送り先には環境変数を使わない
    assert!(keys.env_available(&openai_slot));
    assert!(!keys.env_available(&router_slot));
    assert!(keys.find(&local_slot).is_none());
    assert_eq!(secrets.lock().unwrap().len(), 1);
}

/// 保存先が使えなければ「使えない」になり、環境変数のキーを使う。
#[test]
fn unavailable_store_falls_back_to_env() {
    let slot = claude_slot();
    let keys = ApiKeys::new(Box::new(|| Err("no D-Bus session".into())), env_key);
    assert_eq!(keys.status(&slot), KeyStatus::Unavailable);
    assert!(keys.env_available(&slot));
    assert!(keys.resolve(&slot).is_ok());
    assert!(
        keys.save(&slot, "sk-x")
            .unwrap_err()
            .to_string()
            .contains("D-Bus")
    );

    let failing = ApiKeys::new(
        Box::new(|| {
            Ok(Box::new(MemoryStore {
                fail: true,
                ..MemoryStore::default()
            }) as Box<dyn SecretStore>)
        }),
        no_env,
    );
    assert_eq!(failing.status(&slot), KeyStatus::Unavailable);
    assert!(!failing.env_available(&slot));
    let err = failing.resolve(&slot).unwrap_err().to_string();
    assert!(err.to_string().contains("ANTHROPIC_API_KEY"), "{err}");
}

/// Secret Service の偽の保存先 (Linux)。`failures` を前から順に 1 つずつ失敗として返し、尽きたら普通に読み書きする。
#[cfg(target_os = "linux")]
mod flaky {
    use std::any::Any;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use keyring_core::api::{CredentialApi, CredentialStoreApi};
    use keyring_core::{Credential, Entry, Error, Result};
    pub(super) use ugoe_editor_core::Coded;

    use super::*;

    #[derive(Default)]
    pub(super) struct FlakyCred {
        failures: Mutex<VecDeque<Error>>,
        secret: Mutex<Option<Vec<u8>>>,
        calls: AtomicUsize,
    }

    impl FlakyCred {
        pub(super) fn fail_with(&self, errors: impl IntoIterator<Item = Error>) {
            self.failures.lock().unwrap().extend(errors);
        }

        /// 呼ばれた回数を返し、数え直す。
        pub(super) fn take_calls(&self) -> usize {
            self.calls.swap(0, Ordering::SeqCst)
        }

        fn next_failure(&self) -> Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.failures
                .lock()
                .unwrap()
                .pop_front()
                .map_or(Ok(()), Err)
        }
    }

    impl CredentialApi for FlakyCred {
        fn set_secret(&self, secret: &[u8]) -> Result<()> {
            self.next_failure()?;
            *self.secret.lock().unwrap() = Some(secret.to_vec());
            Ok(())
        }

        fn get_secret(&self) -> Result<Vec<u8>> {
            self.next_failure()?;
            self.secret.lock().unwrap().clone().ok_or(Error::NoEntry)
        }

        fn delete_credential(&self) -> Result<()> {
            self.next_failure()?;
            self.secret
                .lock()
                .unwrap()
                .take()
                .map(drop)
                .ok_or(Error::NoEntry)
        }

        fn get_credential(&self) -> Result<Option<Arc<Credential>>> {
            Ok(None)
        }

        fn get_specifiers(&self) -> Option<(String, String)> {
            None
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    pub(super) struct FlakyStore(pub(super) Arc<FlakyCred>);

    impl CredentialStoreApi for FlakyStore {
        fn vendor(&self) -> String {
            "flaky".into()
        }

        fn id(&self) -> String {
            "flaky".into()
        }

        fn build(
            &self,
            _service: &str,
            _user: &str,
            _modifiers: Option<&HashMap<&str, &str>>,
        ) -> Result<Entry> {
            Ok(Entry::new_with_credential(self.0.clone()))
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// dbus-secret-service-keyring-store が返す形の、D-Bus の失敗 (`name` の D-Bus のエラー)。
    pub(super) fn dbus_failure(name: &str) -> Error {
        Error::PlatformFailure(Box::new(dbus_secret_service::Error::Dbus(
            dbus::Error::new_custom(name, "Did not receive a reply"),
        )))
    }

    pub(super) fn no_reply() -> Error {
        dbus_failure("org.freedesktop.DBus.Error.NoReply")
    }

    /// 偽の Secret Service を [`KeyringStore`] で包んだ [`ApiKeys`]。
    pub(super) fn keys(cred: &Arc<FlakyCred>) -> ApiKeys {
        let cred = cred.clone();
        ApiKeys::new(
            Box::new(move || {
                let store: Arc<keyring_core::CredentialStore> = Arc::new(FlakyStore(cred.clone()));
                Ok(Box::new(KeyringStore { store }) as Box<dyn SecretStore>)
            }),
            no_env,
        )
    }
}

/// Secret Service の答えが時間内に来なかったときは、保存・読み出し・削除のどれもやり直す。
#[cfg(target_os = "linux")]
#[test]
fn keyring_calls_are_retried_when_the_secret_service_does_not_reply() {
    use flaky::*;
    let slot = claude_slot();
    let cred = Arc::new(FlakyCred::default());
    let keys = flaky::keys(&cred);

    cred.fail_with([no_reply()]);
    keys.save(&slot, "sk-a").unwrap();
    assert_eq!(cred.take_calls(), 2);

    cred.fail_with([no_reply(), no_reply()]);
    assert_eq!(keys.status(&slot), KeyStatus::Stored);
    assert_eq!(cred.take_calls(), 3);
    cred.fail_with([no_reply()]);
    assert!(keys.find(&slot).is_some());
    assert_eq!(cred.take_calls(), 2);

    cred.fail_with([no_reply()]);
    keys.delete(&slot).unwrap();
    assert_eq!(cred.take_calls(), 2);
    assert_eq!(keys.status(&slot), KeyStatus::NotStored);
    cred.take_calls();
}

/// 答えが来ないのが続けば、上限の回数で諦めて今のエラー (`ai.credential_store`) を返す。
#[cfg(target_os = "linux")]
#[test]
fn keyring_calls_give_up_after_repeated_no_replies() {
    use flaky::*;
    let slot = claude_slot();
    let cred = Arc::new(FlakyCred::default());
    let keys = flaky::keys(&cred);
    let too_many = || (0..KEYRING_ATTEMPTS).map(|_| no_reply());

    cred.fail_with(too_many());
    let err = keys.save(&slot, "sk-a").unwrap_err();
    assert!(matches!(err, EditorError::CredentialStore(_)), "{err:?}");
    assert_eq!(err.code(), "ai.credential_store");
    assert!(err.to_string().contains("Did not receive a reply"), "{err}");
    assert_eq!(cred.take_calls(), KEYRING_ATTEMPTS);

    cred.fail_with(too_many());
    assert_eq!(keys.status(&slot), KeyStatus::Unavailable);
    assert_eq!(cred.take_calls(), KEYRING_ATTEMPTS);

    cred.fail_with(too_many());
    let err = keys.delete(&slot).unwrap_err();
    assert_eq!(err.code(), "ai.credential_store");
    assert_eq!(cred.take_calls(), KEYRING_ATTEMPTS);
}

/// 答えが来ないほかの失敗 (サービスが無い・保管庫が施錠されている) はやり直さない。
#[cfg(target_os = "linux")]
#[test]
fn other_keyring_failures_are_not_retried() {
    use flaky::*;
    let slot = claude_slot();
    let cred = Arc::new(FlakyCred::default());
    let keys = flaky::keys(&cred);
    let others = [
        || dbus_failure("org.freedesktop.DBus.Error.ServiceUnknown"),
        || keyring_core::Error::NoStorageAccess(Box::new(dbus_secret_service::Error::Locked)),
        || keyring_core::Error::PlatformFailure(Box::new(dbus_secret_service::Error::Prompt)),
    ];
    for failure in others {
        cred.fail_with([failure()]);
        let err = keys.save(&slot, "sk-a").unwrap_err();
        assert_eq!(err.code(), "ai.credential_store");
        assert_eq!(cred.take_calls(), 1);

        cred.fail_with([failure()]);
        assert_eq!(keys.status(&slot), KeyStatus::Unavailable);
        assert_eq!(cred.take_calls(), 1);

        cred.fail_with([failure()]);
        assert!(keys.delete(&slot).is_err());
        assert_eq!(cred.take_calls(), 1);
    }
}

/// キーは `Debug` にもエラーにも出ない。空白だけの保存値は使わない。
#[test]
fn keys_are_never_shown() {
    let slot = claude_slot();
    let secrets = Secrets::default();
    secrets
        .lock()
        .unwrap()
        .insert(slot.account.clone(), "sk-stored-secret".into());
    let keys = keys(secrets, env_key);
    let key = keys.resolve(&slot).unwrap();
    let shown = format!("{key:?} {keys:?}");
    assert!(!shown.contains("sk-"), "{shown}");
    let blank = Secrets::default();
    blank
        .lock()
        .unwrap()
        .insert(slot.account.clone(), "   ".into());
    assert!(self::keys(blank, no_env).resolve(&slot).is_err());
}

fn external(images: usize) -> Disclosure {
    let request = ProviderRequest {
        system: "s".into(),
        prompt: "p".into(),
        images: vec![vec![0; 10]; images],
    };
    Disclosure::new(
        Destination::External {
            service: "Anthropic Claude API".into(),
            endpoint: "https://api.anthropic.com/v1/messages".into(),
            model: "claude-opus-5-5".into(),
        },
        &request,
    )
}

#[test]
fn consent_is_asked_for_each_external_request() {
    let mut memory = ConsentMemory::default();
    let disclosure = external(2);
    let Gate::Ask(view) =
        check_consent(true, &mut memory, DisclosureView::new(&disclosure), None).unwrap()
    else {
        panic!("expected to ask");
    };
    assert_eq!(
        (view.image_count, view.image_bytes, view.text_bytes),
        (2, 20, 2)
    );

    let once = ConsentAnswer {
        disclosure: view.clone(),
        remember: false,
    };
    assert_eq!(
        check_consent(
            true,
            &mut memory,
            DisclosureView::new(&disclosure),
            Some(&once)
        )
        .unwrap(),
        Gate::Proceed
    );
    assert!(!memory.is_remembered());
    // 1 回きりの同意は次の送信には使わない。
    assert!(matches!(
        check_consent(true, &mut memory, DisclosureView::new(&disclosure), None).unwrap(),
        Gate::Ask(_)
    ));
    // 同意した内容と違う (画像が増えた) なら確かめ直す。
    assert!(matches!(
        check_consent(
            true,
            &mut memory,
            DisclosureView::new(&external(3)),
            Some(&once)
        )
        .unwrap(),
        Gate::Ask(_)
    ));
}

#[test]
fn remembered_consent_covers_the_same_destination_only() {
    let mut memory = ConsentMemory::default();
    let disclosure = external(1);
    let view = DisclosureView::new(&disclosure).unwrap();
    let remember = ConsentAnswer {
        disclosure: view,
        remember: true,
    };
    check_consent(
        true,
        &mut memory,
        DisclosureView::new(&disclosure),
        Some(&remember),
    )
    .unwrap();
    assert!(memory.is_remembered());
    // 同じ送り先なら、画像の数が違っても確かめない。
    assert_eq!(
        check_consent(true, &mut memory, DisclosureView::new(&external(5)), None).unwrap(),
        Gate::Proceed
    );
    // 別のモデルなら確かめる。
    let mut other = external(1);
    if let Destination::External { model, .. } = &mut other.destination {
        *model = "claude-sonnet-5-5".into();
    }
    assert!(matches!(
        check_consent(true, &mut memory, DisclosureView::new(&other), None).unwrap(),
        Gate::Ask(_)
    ));
    memory.forget();
    assert!(matches!(
        check_consent(true, &mut memory, DisclosureView::new(&disclosure), None).unwrap(),
        Gate::Ask(_)
    ));
}

/// 「外部に送らない」プロジェクトでは、同意があっても外部には送らない。ローカルは送ってよい。
#[test]
fn disabled_projects_never_send_externally() {
    let mut memory = ConsentMemory::default();
    let disclosure = external(1);
    let answer = ConsentAnswer {
        disclosure: DisclosureView::new(&disclosure).unwrap(),
        remember: true,
    };
    let err = check_consent(
        false,
        &mut memory,
        DisclosureView::new(&disclosure),
        Some(&answer),
    )
    .unwrap_err();
    assert!(matches!(err, EditorError::SendDisabled), "{err}");
    assert!(!memory.is_remembered());

    let local = FakeProvider::new().disclosure(&ProviderRequest::default());
    assert_eq!(
        check_consent(false, &mut memory, DisclosureView::new(&local), None).unwrap(),
        Gate::Proceed
    );
}

pub(crate) fn state(secret: Option<&str>) -> AppState {
    let (session, document) = crate::project::initial_state();
    let secrets = Secrets::default();
    if let Some(secret) = secret {
        secrets
            .lock()
            .unwrap()
            .insert(claude_slot().account, secret.into());
    }
    AppState {
        document: Mutex::new(document),
        project: Mutex::new(session),
        saving: Mutex::new(()),
        api_keys: keys(secrets, no_env),
        ai_jobs: AiJobs::default(),
        ai_provider: ProviderStore::default(),
        mcp: crate::mcp::McpBridge::default(),
        preview_base: crate::module::PreviewBaseCache::default(),
        undo_limit: crate::history::UndoLimitStore::default(),
        files: crate::access::FileAccess::default(),
    }
}

/// 送る IPC コマンドの入口: 設定・キー・同意の順に確かめ、同意がなければ送らない。
#[test]
fn prepare_checks_settings_key_and_consent() {
    let (request, _) = test_request();
    let requests = std::slice::from_ref(&request);
    let test = Purpose::ConnectionTest;

    let no_key = state(None);
    let err = prepare_requests(&no_key, test, requests, 0, None).unwrap_err();
    assert!(err.to_string().contains("API key"), "{err}");

    let state = state(Some("sk-test"));
    let Prepared::NeedsConsent(view) = prepare_requests(&state, test, requests, 0, None).unwrap()
    else {
        panic!("expected to ask for consent");
    };
    assert_eq!(view.model, "claude-opus-5-5");
    assert_eq!(view.service, "Anthropic Claude API");
    assert_eq!(view.image_count, 0);
    let answer = ConsentAnswer {
        disclosure: view,
        remember: false,
    };
    assert!(matches!(
        prepare_requests(&state, test, requests, 0, Some(&answer)).unwrap(),
        Prepared::Ready(_)
    ));

    // 「外部に送らない」にすると、同意があっても断る。
    state.project.lock().unwrap().set_ai_send_externally(false);
    let err = prepare_requests(&state, test, requests, 0, Some(&answer)).unwrap_err();
    assert_eq!(err.code, "ai.send_disabled");
    let settings = settings(&state).unwrap();
    assert!(!settings.send_externally);
    assert_eq!(settings.key, KeyStatus::Stored);
}

/// OpenAI 互換の API: 外部の送り先は同じ同意の流れを通り、送り先・モデルを示す。キーがなくても送れる。
/// 構造化出力を使わないなら、プロバイダの問い直しも送り直す回数に含める。
#[test]
fn openai_compatible_goes_through_the_same_consent() {
    let state = state(None);
    let mut provider = openai("https://openrouter.ai/api/v1");
    provider.openai.model = "openai/gpt-6-sol".into();
    state.ai_provider.set(provider.clone()).unwrap();
    let (request, _) = test_request();
    let requests = std::slice::from_ref(&request);
    let Prepared::NeedsConsent(view) =
        prepare_requests(&state, Purpose::Proposal, requests, 2, None).unwrap()
    else {
        panic!("expected to ask for consent");
    };
    assert_eq!(view.service, "OpenRouter");
    assert_eq!(
        view.endpoint,
        "https://openrouter.ai/api/v1/chat/completions"
    );
    assert_eq!(view.model, "openai/gpt-6-sol");
    assert_eq!(view.retries, 2);

    provider.openai.output = OutputMode::Prompt;
    state.ai_provider.set(provider).unwrap();
    let Prepared::NeedsConsent(view) =
        prepare_requests(&state, Purpose::Proposal, requests, 2, None).unwrap()
    else {
        panic!("expected to ask for consent");
    };
    // 3 回の問い合わせそれぞれで、2 回まで問い直す
    assert_eq!(view.retries, 8);

    state.project.lock().unwrap().set_ai_send_externally(false);
    assert_eq!(
        prepare_requests(&state, Purpose::Proposal, requests, 0, None)
            .unwrap_err()
            .code,
        "ai.send_disabled"
    );
}

/// 手元のサーバ (ループバック) は外部に出ないので、同意を求めず、「外部に送らない」でも送れる。
#[test]
fn loopback_servers_need_no_consent() {
    let state = state(None);
    state.project.lock().unwrap().set_ai_send_externally(false);
    state
        .ai_provider
        .set(openai("http://localhost:11434/v1"))
        .unwrap();
    let (request, _) = test_request();
    assert!(matches!(
        prepare_requests(
            &state,
            Purpose::Proposal,
            std::slice::from_ref(&request),
            0,
            None
        )
        .unwrap(),
        Prepared::Ready(_)
    ));
    let settings = settings(&state).unwrap();
    assert!(settings.local);
    assert!(!settings.key_required);
    assert_eq!(settings.service, "Local OpenAI-compatible server");
}

/// キーを平文の http で外部に送らない。
#[test]
fn keys_are_not_sent_over_plain_http() {
    let settings = openai("http://llm.example.com/v1");
    let key = ApiKey::new("sk-x").unwrap();
    let Err(err) = settings.build(Some(key.clone()), Purpose::Proposal, CancelToken::new()) else {
        panic!("expected to refuse");
    };
    assert!(err.to_string().contains("https"), "{err}");
    assert!(
        settings
            .build(None, Purpose::Proposal, CancelToken::new())
            .is_ok()
    );
    assert!(
        openai("http://127.0.0.1:1234/v1")
            .build(Some(key), Purpose::Proposal, CancelToken::new())
            .is_ok()
    );
}

/// プロバイダの設定は検査してから保存し、ファイルから読み直せる。ファイルがなければ既定 (Claude)。
#[test]
fn provider_settings_are_checked_and_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("provider");
    let store = ProviderStore::default();
    store.load(dir.clone()).unwrap();
    assert_eq!(store.get(), ProviderSettings::default());
    assert_eq!(store.get().provider, ProviderKind::Claude);

    for bad in [openai("ftp://example.com"), openai("https://")] {
        assert!(store.set(bad).is_err());
    }
    let mut no_model = openai("http://localhost:1234/v1");
    no_model.openai.model = " ".into();
    assert!(store.set(no_model).is_err());
    assert_eq!(store.get(), ProviderSettings::default());

    let chosen = openai("http://localhost:1234/v1");
    store.set(chosen.clone()).unwrap();
    let text = std::fs::read_to_string(dir.join("ai-provider.json")).unwrap();
    assert!(!text.contains("sk-"), "{text}");
    let reloaded = ProviderStore::default();
    reloaded.load(dir.clone()).unwrap();
    assert_eq!(reloaded.get(), chosen);

    // 知らない項目・欠けた項目があっても読める
    std::fs::write(
        dir.join("ai-provider.json"),
        r#"{"provider":"openai_compatible","openai":{"model":"m"},"future":1}"#,
    )
    .unwrap();
    reloaded.load(dir.clone()).unwrap();
    let loaded = reloaded.get();
    assert_eq!(loaded.openai.model, "m");
    assert_eq!(loaded.openai.base_url, "https://api.openai.com/v1");
}

/// 読めない設定は、起動しただけでは書き換えない (利用者が設定を変えたときに初めて書く。T0011)。
#[test]
fn an_unreadable_provider_settings_file_is_not_overwritten_at_startup() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let path = dir.join("ai-provider.json");
    std::fs::write(&path, "{").unwrap();
    let store = ProviderStore::default();
    assert!(store.load(dir.to_path_buf()).is_err());
    assert_eq!(store.get(), ProviderSettings::default());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{");
}

/// 設定の応答はキーの中身を含まない。
#[test]
fn settings_never_contain_the_key() {
    let state = state(Some("sk-very-secret"));
    let json = serde_json::to_string(&settings(&state).unwrap()).unwrap();
    assert!(!json.contains("sk-"), "{json}");
    assert!(json.contains(r#""key":"stored","key_required":true,"key_env":"ANTHROPIC_API_KEY","env_key":false,"send_externally":true,"consent_remembered":false}"#), "{json}");
    assert!(
        json.starts_with(
            r#"{"provider":{"provider":"claude","claude":{"model":"claude-opus-5-5"}"#
        ),
        "{json}"
    );
}

/// 「外部に送らない」は project.json の `editor.ai` に保存され、開き直しても残る。
#[test]
fn send_externally_is_saved_with_the_project() {
    let state = state(None);
    state.project.lock().unwrap().set_ai_send_externally(false);
    let project = {
        let document = state.document.lock().unwrap();
        state.project.lock().unwrap().to_project(&document)
    };
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("project.ugoeproj");
    project.save(&dir).unwrap();
    let loaded = Project::load(&dir).unwrap();
    assert!(!loaded.meta.ai_send_externally());
}

/// 複数のリクエストをまとめた確認は合計を示す。外部に出ないものは数えず、送り先が違えば混ぜない。
#[test]
fn combined_disclosures_add_up() {
    let local = FakeProvider::new().disclosure(&ProviderRequest::default());
    let view = DisclosureView::combined(&[external(2), local.clone(), external(3)])
        .unwrap()
        .unwrap();
    let single = DisclosureView::new(&external(1)).unwrap();
    assert_eq!(view.image_count, 5);
    assert_eq!(view.image_bytes, 5 * single.image_bytes);
    assert_eq!(view.text_bytes, 2 * single.text_bytes);
    assert!(DisclosureView::combined(&[local]).unwrap().is_none());
    let mut other = external(1);
    if let Destination::External { model, .. } = &mut other.destination {
        *model = "claude-sonnet-5-5".into();
    }
    assert!(DisclosureView::combined(&[external(1), other]).is_err());
}

/// 取り消し: 実行中の送信は一覧にあり、`cancel_ai_requests` で取り消すと送らずに `ai.cancelled` で終わる。
/// 取り消した後に届いた結果も捨てる。終わった送信は一覧から外れる。
#[test]
fn running_requests_can_be_cancelled() {
    let state = state(None);
    state
        .ai_provider
        .set(openai("http://127.0.0.1:9/v1"))
        .unwrap();
    let (request, schema) = test_request();
    let requests = std::slice::from_ref(&request);
    let Prepared::Ready(job) =
        prepare_requests(&state, Purpose::ConnectionTest, requests, 0, None).unwrap()
    else {
        panic!("expected to be ready");
    };
    assert_eq!(state.ai_jobs.running(), 1);
    assert_eq!(state.ai_jobs.cancel_all(), 1);
    // 取り消しの後は送らない (送っていればつながらずに network のエラーになる)
    let err = job.provider().complete_json(&request, &schema).unwrap_err();
    assert!(matches!(err, AiError::Cancelled), "{err}");
    assert!(matches!(job.finish(Ok(())), Err(AiError::Cancelled)));
    assert_eq!(state.ai_jobs.running(), 0);
    assert_eq!(state.ai_jobs.cancel_all(), 0);
    assert_eq!(
        ugoe_editor_core::Coded::code(&EditorError::Ai(AiError::Cancelled)),
        "ai.cancelled"
    );

    // 取り消していない送信の結果はそのまま
    let Prepared::Ready(job) =
        prepare_requests(&state, Purpose::ConnectionTest, requests, 0, None).unwrap()
    else {
        panic!("expected to be ready");
    };
    assert!(matches!(job.finish(Ok(7)), Ok(7)));
}

/// 時間の上限: 既定は 15 分。前の版の設定のファイル (上限がない) も既定で読み、範囲の外は保存しない。
#[test]
fn time_limit_setting() {
    let settings = ProviderSettings::default();
    assert_eq!(settings.time_limit_minutes, 15);
    assert_eq!(settings.time_limit(), std::time::Duration::from_secs(900));
    let old: ProviderSettings = serde_json::from_str(r#"{"provider":"claude"}"#).unwrap();
    assert_eq!(old.time_limit_minutes, 15);
    for minutes in [0, 121] {
        let state = state(None);
        let err = state
            .ai_provider
            .set(ProviderSettings {
                time_limit_minutes: minutes,
                ..ProviderSettings::default()
            })
            .unwrap_err();
        assert_eq!(ugoe_editor_core::Coded::code(&err), "ai.invalid_time_limit");
    }
}
