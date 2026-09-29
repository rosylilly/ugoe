use std::sync::Mutex;

use ugoe_ai::FakeProvider;
use ugoe_editor_core::Project;

use super::*;

/// メモリ上の保存先。`fail` なら使えない保存先として振る舞う。
#[derive(Default)]
struct MemoryStore {
    secret: Arc<Mutex<Option<String>>>,
    fail: bool,
}

impl SecretStore for MemoryStore {
    fn get(&self) -> Result<Option<String>, String> {
        if self.fail {
            return Err("no secret service".into());
        }
        Ok(self.secret.lock().unwrap().clone())
    }

    fn set(&self, secret: &str) -> Result<(), String> {
        if self.fail {
            return Err("no secret service".into());
        }
        *self.secret.lock().unwrap() = Some(secret.into());
        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        if self.fail {
            return Err("no secret service".into());
        }
        *self.secret.lock().unwrap() = None;
        Ok(())
    }
}

fn keys(secret: Arc<Mutex<Option<String>>>, env: EnvReader) -> ApiKeys {
    ApiKeys::new(
        Box::new(move || {
            Ok(Box::new(MemoryStore {
                secret: secret.clone(),
                fail: false,
            }) as Box<dyn SecretStore>)
        }),
        env,
    )
}

fn no_env() -> Option<String> {
    None
}

fn env_key() -> Option<String> {
    Some("sk-env".into())
}

#[test]
fn keys_are_saved_trimmed_and_deleted() {
    let secret = Arc::new(Mutex::new(None));
    let keys = keys(secret.clone(), no_env);
    assert_eq!(keys.status(), KeyStatus::NotStored);
    assert!(keys.resolve().is_err());
    assert!(keys.save("  ").is_err());

    keys.save("  sk-stored \n").unwrap();
    assert_eq!(secret.lock().unwrap().as_deref(), Some("sk-stored"));
    assert_eq!(keys.status(), KeyStatus::Stored);
    assert!(keys.resolve().is_ok());

    keys.delete().unwrap();
    keys.delete().unwrap();
    assert_eq!(keys.status(), KeyStatus::NotStored);
}

/// 保存先が使えなければ「使えない」になり、環境変数のキーを使う。
#[test]
fn unavailable_store_falls_back_to_env() {
    let keys = ApiKeys::new(Box::new(|| Err("no D-Bus session".into())), env_key);
    assert_eq!(keys.status(), KeyStatus::Unavailable);
    assert!(keys.env_available());
    assert!(keys.resolve().is_ok());
    assert!(keys.save("sk-x").unwrap_err().contains("D-Bus"));

    let failing = ApiKeys::new(
        Box::new(|| {
            Ok(Box::new(MemoryStore {
                fail: true,
                ..MemoryStore::default()
            }) as Box<dyn SecretStore>)
        }),
        no_env,
    );
    assert_eq!(failing.status(), KeyStatus::Unavailable);
    assert!(!failing.env_available());
    let err = failing.resolve().unwrap_err().to_string();
    assert!(err.contains(API_KEY_ENV), "{err}");
}

/// キーは `Debug` にもエラーにも出ない。空白だけの保存値は使わない。
#[test]
fn keys_are_never_shown() {
    let keys = keys(
        Arc::new(Mutex::new(Some("sk-stored-secret".into()))),
        env_key,
    );
    let key = keys.resolve().unwrap();
    let shown = format!("{key:?} {keys:?}");
    assert!(!shown.contains("sk-"), "{shown}");
    let blank = self::keys(Arc::new(Mutex::new(Some("   ".into()))), no_env);
    assert!(blank.resolve().is_err());
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
    let Gate::Ask(view) = check_consent(true, &mut memory, &disclosure, None).unwrap() else {
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
        check_consent(true, &mut memory, &disclosure, Some(&once)).unwrap(),
        Gate::Proceed
    );
    assert!(!memory.is_remembered());
    // 1 回きりの同意は次の送信には使わない。
    assert!(matches!(
        check_consent(true, &mut memory, &disclosure, None).unwrap(),
        Gate::Ask(_)
    ));
    // 同意した内容と違う (画像が増えた) なら確かめ直す。
    assert!(matches!(
        check_consent(true, &mut memory, &external(3), Some(&once)).unwrap(),
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
    check_consent(true, &mut memory, &disclosure, Some(&remember)).unwrap();
    assert!(memory.is_remembered());
    // 同じ送り先なら、画像の数が違っても確かめない。
    assert_eq!(
        check_consent(true, &mut memory, &external(5), None).unwrap(),
        Gate::Proceed
    );
    // 別のモデルなら確かめる。
    let mut other = external(1);
    if let Destination::External { model, .. } = &mut other.destination {
        *model = "claude-sonnet-5-5".into();
    }
    assert!(matches!(
        check_consent(true, &mut memory, &other, None).unwrap(),
        Gate::Ask(_)
    ));
    memory.forget();
    assert!(matches!(
        check_consent(true, &mut memory, &disclosure, None).unwrap(),
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
    let err = check_consent(false, &mut memory, &disclosure, Some(&answer)).unwrap_err();
    assert_eq!(err, SEND_DISABLED);
    assert!(!memory.is_remembered());

    let local = FakeProvider::new().disclosure(&ProviderRequest::default());
    assert_eq!(
        check_consent(false, &mut memory, &local, None).unwrap(),
        Gate::Proceed
    );
}

fn state(secret: Option<&str>) -> AppState {
    let (session, document) = crate::project::initial_state();
    AppState {
        document: Mutex::new(document),
        project: Mutex::new(session),
        api_keys: keys(Arc::new(Mutex::new(secret.map(Into::into))), no_env),
    }
}

/// 送る IPC コマンドの入口: 設定・キー・同意の順に確かめ、同意がなければ送らない。
#[test]
fn prepare_claude_checks_settings_key_and_consent() {
    let (config, request, _) = test_request();

    let no_key = state(None);
    let err = prepare_claude(&no_key, config.clone(), &request, None).unwrap_err();
    assert!(err.contains("API key"), "{err}");

    let state = state(Some("sk-test"));
    let Prepared::NeedsConsent(view) =
        prepare_claude(&state, config.clone(), &request, None).unwrap()
    else {
        panic!("expected to ask for consent");
    };
    assert_eq!(view.model, config.model);
    assert_eq!(view.image_count, 0);
    let answer = ConsentAnswer {
        disclosure: view,
        remember: false,
    };
    assert!(matches!(
        prepare_claude(&state, config.clone(), &request, Some(&answer)).unwrap(),
        Prepared::Ready(_)
    ));

    // 「外部に送らない」にすると、同意があっても断る。
    state.project.lock().unwrap().set_ai_send_externally(false);
    let err = prepare_claude(&state, config, &request, Some(&answer)).unwrap_err();
    assert_eq!(err, SEND_DISABLED);
    let settings = settings(&state).unwrap();
    assert!(!settings.send_externally);
    assert_eq!(settings.key, KeyStatus::Stored);
}

/// 設定の応答はキーの中身を含まない。
#[test]
fn settings_never_contain_the_key() {
    let state = state(Some("sk-very-secret"));
    let json = serde_json::to_string(&settings(&state).unwrap()).unwrap();
    assert!(!json.contains("sk-"), "{json}");
    assert_eq!(
        json,
        r#"{"key":"stored","env_key":false,"send_externally":true,"consent_remembered":false}"#
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
    let dir = std::env::temp_dir().join(format!("ugoe-editor-ai-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    project.save(&dir).unwrap();
    let loaded = Project::load(&dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!loaded.meta.ai_send_externally());
}
