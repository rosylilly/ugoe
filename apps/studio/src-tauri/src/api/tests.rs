//! 操作 API の接続と操作のテスト。
//!
//! 前半は WebSocket を通さずに、接続の名簿へ直接メッセージを入れて確かめる (承認・権限・操作・通知)。
//! 後半は本物の待ち受けにループバックで接続し、HTTP の検査と WebSocket の流れを確かめる。

#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};
use ugoe::format::Bundle;
use ugoe::model::fixtures;

use super::rpc::{
    ACCESS_DENIED, FORBIDDEN, INVALID_PARAMS, METHOD_NOT_FOUND, NOT_AUTHENTICATED, OPERATION_FAILED,
};
use super::store::{self, Permission};
use super::ws;
use super::*;
use crate::hotkeys::{Hotkey, HotkeyAction};
use crate::load_failure::{LoadFailure, LoadFailureCode};
use crate::lock;
use crate::stage::{Asset, LOADING, Loading};

/// メッセージの JSON Schema (`docs/studio-api.schema.json`)。
const SCHEMA: &str = include_str!("../../../../../docs/studio-api.schema.json");

fn schema() -> &'static Value {
    static SCHEMA_VALUE: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    SCHEMA_VALUE.get_or_init(|| serde_json::from_str(SCHEMA).unwrap())
}

/// JSON Schema のうち、この Schema が使う語 (`$ref`・`type`・`const`・`enum`・`properties`・`required`・`items`・
/// `oneOf`・`additionalProperties: false`) だけを見る小さな検査。合わなければ理由。
fn conforms(value: &Value, rule: &Value) -> Result<(), String> {
    if let Some(target) = rule.get("$ref").and_then(Value::as_str) {
        let name = target.strip_prefix("#/$defs/").unwrap();
        return conforms(value, &schema()["$defs"][name]).map_err(|e| format!("{name}: {e}"));
    }
    if let Some(types) = rule.get("type") {
        let ok = |t: &str| match t {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.is_i64() || value.is_u64(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => false,
        };
        let matched = match types {
            Value::String(t) => ok(t),
            Value::Array(ts) => ts.iter().any(|t| ok(t.as_str().unwrap())),
            _ => false,
        };
        if !matched {
            return Err(format!("{value} is not {types}"));
        }
    }
    if let Some(expected) = rule.get("const")
        && expected != value
    {
        return Err(format!("{value} is not {expected}"));
    }
    if let Some(options) = rule.get("enum").and_then(Value::as_array)
        && !options.contains(value)
    {
        return Err(format!("{value} is not one of {options:?}"));
    }
    if let Some(object) = value.as_object() {
        for key in rule
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if !object.contains_key(key.as_str().unwrap()) {
                return Err(format!("missing `{key}` in {value}"));
            }
        }
        let properties = rule.get("properties").and_then(Value::as_object);
        for (key, item) in object {
            match properties.and_then(|p| p.get(key)) {
                Some(property) => conforms(item, property).map_err(|e| format!("{key}: {e}"))?,
                None if rule.get("additionalProperties") == Some(&Value::Bool(false)) => {
                    return Err(format!("unexpected `{key}`"));
                }
                None => {}
            }
        }
    }
    if let (Some(items), Some(array)) = (rule.get("items"), value.as_array()) {
        for item in array {
            conforms(item, items)?;
        }
    }
    if let Some(variants) = rule.get("oneOf").and_then(Value::as_array) {
        let results: Vec<_> = variants.iter().map(|v| conforms(value, v)).collect();
        let matched = results.iter().filter(|r| r.is_ok()).count();
        if matched != 1 {
            return Err(format!("{value} matches {matched} variants: {results:?}"));
        }
    }
    Ok(())
}

/// メッセージが Schema に合うか (応答の `result` は、メソッドの `<method>.result` にも合うか)。
fn assert_conforms(message: &Value, method: Option<&str>) {
    conforms(message, schema()).unwrap_or_else(|e| panic!("{message}: {e}"));
    if let (Some(method), Some(result)) = (method, message.get("result")) {
        conforms(
            result,
            &json!({ "$ref": format!("#/$defs/{method}.result") }),
        )
        .unwrap_or_else(|e| panic!("{method}: {e}"));
    }
}

const TOKEN: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// テスト用のファイルを置く場所 (テストごとの一時ディレクトリ。破棄時に消す)。
struct Dir(tempfile::TempDir);

impl Dir {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    /// 着せ替えのできるモデル (`.ugoe`) に、モーション `Wave` (`BodySway`。始まりと終わりにイベント) と表情 `Smile`・`Angry` (`Secret`) を入れて書く。
    fn model(&self) -> PathBuf {
        let mut bundle = Bundle::new(fixtures::dress_up_base());
        bundle.motions = vec![
            ugoe::format::motion_from_json(
                r#"{ "name": "Wave", "duration": 0.5, "looping": false, "fade_in": 0.0, "fade_out": 0.0,
                     "curves": [{ "target": { "kind": "parameter", "id": "BodySway" },
                                  "segments": [{ "type": "start", "p": { "t": 0.0, "v": 1.0 } },
                                               { "type": "linear", "to": { "t": 0.5, "v": 1.0 } }] }],
                     "events": [{ "t": 0.0, "value": "wave-start" }, { "t": 0.5, "value": "wave-end" }] }"#,
            )
            .unwrap(),
            // 替えられた後もフェードアウトしながらイベントを発火する (T0821)。
            ugoe::format::motion_from_json(
                r#"{ "name": "Bow", "duration": 1.0, "looping": false, "fade_in": 0.0, "fade_out": 0.5,
                     "curves": [], "events": [{ "t": 0.25, "value": "bow-late" }] }"#,
            )
            .unwrap(),
        ];
        bundle.expressions = ["Smile", "Angry"]
            .iter()
            .map(|name| {
                ugoe::format::expression_from_json(&format!(
                    r#"{{ "name": "{name}", "fade_in": 0.0, "fade_out": 0.0,
                         "parameters": [{{ "parameter": "Secret", "value": 1.0, "op": "add" }}] }}"#
                ))
                .unwrap()
            })
            .collect();
        // 書き出しはテクスチャの数と先頭のシグネチャだけを確かめる (中身はデコードしない)。
        bundle.textures = vec![
            ugoe::format::TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
            };
            bundle.model.textures.len()
        ];
        let path = self.0.path().join("avatar.ugoe");
        std::fs::write(&path, ugoe::format::write_bundle(&bundle).unwrap()).unwrap();
        path
    }

    /// ポーズのあるモデル (`.ugoe`。グループ `ArmA` / `ArmB`) を書く (T0843)。モーション `wave` は 0.5 秒 `ArmB` を表示させる (T0848)。
    fn pose_model(&self) -> PathBuf {
        let mut bundle = Bundle::new(fixtures::pose_and_effects());
        bundle.motions.push(wave_motion());
        bundle.textures = vec![
            ugoe::format::TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
            };
            bundle.model.textures.len()
        ];
        let path = self.0.path().join("pose.ugoe");
        std::fs::write(&path, ugoe::format::write_bundle(&bundle).unwrap()).unwrap();
        path
    }

    fn module(&self, name: &str, module: ugoe::model::module::Module) -> PathBuf {
        let path = self.0.path().join(name);
        let mut file = ugoe::format::ModuleFile::new(module);
        file.textures = vec![
            ugoe::format::TextureData {
                format: ugoe::format::TextureFormat::Png,
                bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
            };
            file.module.textures.len()
        ];
        std::fs::write(&path, ugoe::format::write_module(&file).unwrap()).unwrap();
        path
    }
}

fn studio() -> Arc<Studio> {
    let studio = Arc::new(Studio::default());
    lock(&studio.api.0).settings.token = TOKEN.to_owned();
    studio
}

/// 名簿に入れた接続 (WebSocket を通さない)。
struct Client<'a> {
    studio: &'a Studio,
    id: u64,
    rx: Receiver<Outgoing>,
    next: u64,
    /// 応答を待つ間に届いた通知 (操作が起こした通知は応答より先に届くことがある)。
    notifications: std::collections::VecDeque<Value>,
    /// 送った要求のメソッド (`id` ごと。応答を Schema と比べる)。
    methods: std::collections::HashMap<u64, String>,
}

impl<'a> Client<'a> {
    fn connect(studio: &'a Studio) -> Self {
        let (tx, rx) = mpsc::channel();
        let id = studio.api_connect(tx).unwrap();
        Self {
            studio,
            id,
            rx,
            next: 1,
            notifications: std::collections::VecDeque::new(),
            methods: std::collections::HashMap::new(),
        }
    }

    fn send(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next;
        self.next += 1;
        let text = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.methods.insert(id, method.to_owned());
        self.studio.api_receive(self.id, &text.to_string());
        id
    }

    /// 次に届いたもの (無ければ `None`)。
    fn take(&self) -> Option<Outgoing> {
        self.rx.try_recv().ok()
    }

    fn take_json(&self) -> Value {
        match self.take() {
            Some(Outgoing::Text(text)) => {
                let message: Value = serde_json::from_str(&text).unwrap();
                let method = message["id"]
                    .as_u64()
                    .and_then(|id| self.methods.get(&id))
                    .map(String::as_str);
                assert_conforms(&message, method);
                message
            }
            other => panic!("expected a message, got {other:?}"),
        }
    }

    /// 応答 (通知は `notifications` に除けておく)。
    fn response(&mut self) -> Value {
        loop {
            let message = self.take_json();
            if message.get("id").is_some() {
                return message;
            }
            self.notifications.push_back(message);
        }
    }

    /// 次の通知。
    fn notification(&mut self) -> Option<Value> {
        self.notifications
            .pop_front()
            .or_else(|| match self.take() {
                Some(Outgoing::Text(text)) => {
                    let message: Value = serde_json::from_str(&text).unwrap();
                    assert_conforms(&message, None);
                    Some(message)
                }
                _ => None,
            })
    }

    /// 呼んで、応答の `result` を返す (誤りなら落とす)。
    fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.send(method, params);
        let response = self.response();
        assert_eq!(response["id"], id, "{response}");
        assert!(response.get("error").is_none(), "{method}: {response}");
        response["result"].clone()
    }

    /// 呼んで、誤りの `code` を返す。
    fn fail(&mut self, method: &str, params: Value) -> i64 {
        let id = self.send(method, params);
        let response = self.response();
        assert_eq!(response["id"], id, "{response}");
        response["error"]["code"]
            .as_i64()
            .unwrap_or_else(|| panic!("{method}: {response}"))
    }

    /// トークンで名乗り、利用者が `permission` で承認する。鍵を返す。
    fn approved(&mut self, permission: Permission) -> String {
        let id = self.send(
            "session.hello",
            json!({ "name": "Deck", "token": TOKEN, "permission": permission }),
        );
        assert!(self.take().is_none(), "the response waits for the user");
        let pending = self.studio.api_view().pending[0].id.clone();
        self.studio
            .approve_api_client(&pending, permission)
            .unwrap();
        let response = self.take_json();
        assert_eq!(response["id"], id);
        assert_eq!(response["result"]["permission"], json!(permission));
        response["result"]["key"].as_str().unwrap().to_owned()
    }
}

/// 舞台にシーンを置いて (モデルとモジュール)、最初のシーンを出す。シーンの ID を返す。
fn stage_scenes(studio: &Studio, model: &Path, modules: &[PathBuf]) -> (String, String) {
    studio.with_stage(|stage, conductor| {
        let a = stage.profile().scenes[0].id.clone();
        let b = stage.add_scene("Game");
        for (id, modules) in [(&a, Vec::new()), (&b, modules.to_vec())] {
            let mut scene = stage.profile().scene(id).unwrap().clone();
            scene.name = if *id == a {
                "Chat".into()
            } else {
                "Game".into()
            };
            scene.model = Some(model.to_owned());
            scene.modules = modules;
            stage.update_scene(scene, conductor).unwrap();
        }
        stage.activate(&a, conductor).unwrap();
        (a, b)
    })
}

#[test]
fn first_contact_needs_the_token_and_the_users_approval() {
    let studio = studio();
    // トークンが違えば断って閉じる。
    let mut stranger = Client::connect(&studio);
    let id = stranger.send("session.hello", json!({ "name": "x", "token": "nope" }));
    let response = stranger.take_json();
    assert_eq!(
        (response["id"].as_u64(), response["error"]["code"].as_i64()),
        (Some(id), Some(ACCESS_DENIED))
    );
    assert_eq!(
        stranger.take(),
        Some(Outgoing::Close(CLOSE_DENIED, "access denied"))
    );
    assert_eq!(studio.api_view().connections, 0);

    // 名乗る前は何もできない。
    let mut client = Client::connect(&studio);
    assert_eq!(client.fail("scene.list", json!({})), NOT_AUTHENTICATED);
    assert_eq!(
        client.fail("session.hello", json!({ "name": " ", "token": TOKEN })),
        INVALID_PARAMS
    );
    // 名乗れなかった接続は、名乗るまでの期限 (`server.rs` の `HelloDeadline`) で閉じられる側のまま。
    assert!(!studio.api_introduced(client.id));
    // トークンを示すと承認を待つ (応答は承認まで返らない)。承認待ちは期限で閉じない。
    client.send(
        "session.hello",
        json!({ "name": "Stream Deck", "token": TOKEN, "permission": "control" }),
    );
    assert!(client.take().is_none());
    assert!(studio.api_introduced(client.id));
    let view = studio.api_view();
    assert_eq!(view.pending.len(), 1);
    assert_eq!(view.pending[0].name, "Stream Deck");
    assert_eq!(view.pending[0].permission, Permission::Control);
    assert_eq!(client.fail("scene.list", json!({})), NOT_AUTHENTICATED);
    // 利用者は読むだけで承認する (求めた権限より狭くできる)。
    let approved = studio
        .approve_api_client(&view.pending[0].id, Permission::Read)
        .unwrap();
    let hello = client.take_json();
    assert_eq!(hello["result"]["permission"], "read");
    assert_eq!(hello["result"]["apiVersion"], API_VERSION);
    let key = hello["result"]["key"].as_str().unwrap().to_owned();
    assert_eq!(key.len(), 64);
    let view = studio.api_view();
    assert!(view.pending.is_empty());
    assert_eq!(view.clients.len(), 1);
    // 返す ID は承認したクライアントの ID (承認を待つ間の ID とは違う。画面の「取り消す」が使う。T0919)
    assert_eq!(view.clients[0].id, approved);
    assert_eq!(view.clients[0].connections, 1);
    assert!(client.call("scene.list", json!({}))["scenes"].is_array());
    assert_eq!(
        client.fail("scene.activate", json!({ "scene": "x" })),
        FORBIDDEN
    );
    assert_eq!(client.fail("nope.nope", json!({})), METHOD_NOT_FOUND);
    assert_eq!(
        client.fail("session.hello", json!({ "name": "again", "token": TOKEN })),
        rpc::INVALID_REQUEST
    );

    // 次からは鍵で (トークン無しで) すぐに使える。鍵は応答に入れない。
    let mut again = Client::connect(&studio);
    let result = again.call(
        "session.hello",
        json!({ "name": "Stream Deck", "key": key }),
    );
    assert_eq!(result["permission"], "read");
    assert!(result.get("key").is_none());
    assert_eq!(studio.api_view().clients[0].connections, 2);

    // 取り消すと、つないでいる接続を閉じ、鍵は使えなくなる。
    let client_id = studio.api_view().clients[0].id.clone();
    studio.revoke_api_client(&client_id);
    assert_eq!(
        client.take(),
        Some(Outgoing::Close(CLOSE_REVOKED, "revoked"))
    );
    assert_eq!(
        again.take(),
        Some(Outgoing::Close(CLOSE_REVOKED, "revoked"))
    );
    let mut late = Client::connect(&studio);
    late.send(
        "session.hello",
        json!({ "name": "Stream Deck", "key": key }),
    );
    assert_eq!(late.take_json()["error"]["code"], ACCESS_DENIED);
}

#[test]
fn the_user_can_deny_disconnect_and_rotate_the_token() {
    let studio = studio();
    let mut client = Client::connect(&studio);
    let id = client.send("session.hello", json!({ "name": "Bot", "token": TOKEN }));
    let pending = studio.api_view().pending[0].clone();
    assert_eq!(
        pending.permission,
        Permission::Read,
        "read is the default request"
    );
    studio.deny_api_client(&pending.id);
    let response = client.take_json();
    assert_eq!(
        (response["id"].as_u64(), response["error"]["code"].as_i64()),
        (Some(id), Some(ACCESS_DENIED))
    );
    assert_eq!(
        client.take(),
        Some(Outgoing::Close(CLOSE_DENIED, "not approved"))
    );
    assert!(studio.api_view().pending.is_empty());

    // 承認を待つ間に接続が切れたら、待ちから外す。
    let mut gone = Client::connect(&studio);
    gone.send("session.hello", json!({ "name": "Gone", "token": TOKEN }));
    assert_eq!(studio.api_view().pending.len(), 1);
    studio.api_disconnect(gone.id);
    assert!(studio.api_view().pending.is_empty());

    // 切断は鍵を残す。
    let mut client = Client::connect(&studio);
    let key = client.approved(Permission::Control);
    let client_id = studio.api_view().clients[0].id.clone();
    studio.disconnect_api_client(&client_id);
    assert_eq!(
        client.take(),
        Some(Outgoing::Close(
            CLOSE_GOING_AWAY,
            "disconnected by the user"
        ))
    );
    let mut back = Client::connect(&studio);
    assert_eq!(
        back.call("session.hello", json!({ "name": "Bot", "key": key }))["permission"],
        "control"
    );

    // トークンを作り直すと、古いトークンでは初めての接続ができない (鍵はそのまま)。
    let fresh = studio.regenerate_api_token().unwrap();
    assert_ne!(fresh, TOKEN);
    assert_eq!(studio.api_token(), fresh);
    let mut old = Client::connect(&studio);
    old.send("session.hello", json!({ "name": "Old", "token": TOKEN }));
    assert_eq!(old.take_json()["error"]["code"], ACCESS_DENIED);
    let mut keyed = Client::connect(&studio);
    assert_eq!(
        keyed.call("session.hello", json!({ "name": "Bot", "key": key }))["permission"],
        "control"
    );
}

#[test]
fn operates_scenes_expressions_motions_and_parameters() {
    let dir = Dir::new();
    let model = dir.model();
    let studio = studio();
    let (chat, game) = stage_scenes(&studio, &model, &[]);
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);

    let list = client.call("scene.list", json!({}));
    assert_eq!(list["live"], chat.as_str());
    assert_eq!(
        list["scenes"],
        json!([{ "id": chat, "name": "Chat", "model": "avatar.ugoe" }, { "id": game, "name": "Game", "model": "avatar.ugoe" }])
    );
    client.call("scene.activate", json!({ "scene": game }));
    assert_eq!(lock(&studio.stage).live(), Some(game.as_str()));
    let error = {
        let id = client.send("scene.activate", json!({ "scene": "nope" }));
        let response = client.take_json();
        assert_eq!(response["id"], id);
        response["error"].clone()
    };
    assert_eq!(error["code"], OPERATION_FAILED);
    assert_eq!(error["data"]["code"], "unknownScene");
    assert_eq!(error["data"]["id"], "nope");
    assert_eq!(error["data"]["messageCode"], "scene.unknown");
    assert_eq!(error["data"]["args"], json!({ "id": "nope" }));

    // 表情
    assert_eq!(
        client.call("expression.list", json!({}))["expressions"],
        json!(["Smile", "Angry"])
    );
    client.call("expression.set", json!({ "expression": "Smile" }));
    client.call("expression.add", json!({ "expression": "Angry" }));
    client.call("expression.toggle", json!({ "expression": "Smile" }));
    assert_eq!(
        client.call("expression.list", json!({}))["active"],
        json!(["Angry"])
    );
    client.call("expression.remove", json!({ "expression": "Angry" }));
    client.call("expression.add", json!({ "expression": "Smile" }));
    client.call("expression.clear", json!({}));
    assert_eq!(
        client.call("expression.list", json!({}))["active"],
        json!([])
    );
    let id = client.send("expression.set", json!({ "expression": "Nope" }));
    let response = client.take_json();
    assert_eq!(response["id"], id);
    assert_eq!(response["error"]["data"]["code"], "unknownExpression");
    assert_eq!(client.fail("expression.set", json!({})), INVALID_PARAMS);

    // モーション
    let motions = client.call("motion.list", json!({}));
    assert_eq!(motions["motions"][0]["name"], "Wave");
    assert_eq!(
        client.call(
            "motion.play",
            json!({ "motion": "Wave", "priority": "force" })
        )["started"],
        true
    );
    assert_eq!(
        client.call("studio.state", json!({}))["motion"]["name"],
        "Wave"
    );
    client.call("motion.stop", json!({}));
    assert_eq!(
        client.fail(
            "motion.play",
            json!({ "motion": "Wave", "priority": "loud" })
        ),
        INVALID_PARAMS
    );

    // パラメータの上書き: 期限 (秒) の間だけ、刻みで配る値が上書きされる。
    let parameters = client.call("parameter.list", json!({}))["parameters"].clone();
    let index = parameters
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["id"] == "AngleX")
        .unwrap();
    let result = client.call(
        "parameter.set",
        json!({ "values": [{ "id": "AngleX", "value": 99.0 }, { "id": "Nope", "value": 1.0 }], "duration": 0.05 }),
    );
    assert_eq!(result["unknown"], json!(["Nope"]));
    let frame = lock(&studio.conductor).tick(0.02).unwrap();
    assert_eq!(frame.values[index], 30.0, "clamped to the range");
    assert_eq!(
        client.call("parameter.list", json!({}))["parameters"][index]["value"],
        30.0
    );
    client.call(
        "parameter.set",
        json!({ "values": [{ "id": "AngleX", "value": 20.0, "weight": 0.5 }] }),
    );
    let frame = lock(&studio.conductor).tick(0.02).unwrap();
    assert!(
        (frame.values[index] - 10.0).abs() < 1e-4,
        "{}",
        frame.values[index]
    );
    for _ in 0..60 {
        lock(&studio.conductor).tick(0.02);
    }
    let frame = lock(&studio.conductor).tick(0.02).unwrap();
    assert_eq!(frame.values[index], 0.0, "the override expires");
    for bad in [
        json!({ "values": [{ "id": "AngleX", "value": 1.0 }], "duration": 0 }),
        json!({ "values": [{ "id": "AngleX", "value": 1.0 }], "duration": 11 }),
        json!({ "values": [{ "id": "AngleX", "value": 1.0, "weight": 2 }] }),
        json!({ "values": [{ "id": "AngleX" }] }),
    ] {
        assert_eq!(
            client.fail("parameter.set", bad.clone()),
            INVALID_PARAMS,
            "{bad}"
        );
    }
    // 1 つでも誤りがあれば、どの値も上書きしない (途中まで当てない)。
    assert_eq!(
        client.fail(
            "parameter.set",
            json!({ "values": [{ "id": "AngleX", "value": 20.0 }, { "id": "AngleY", "value": 1.0, "weight": 2 }] }),
        ),
        INVALID_PARAMS
    );
    let frame = lock(&studio.conductor).tick(0.02).unwrap();
    assert_eq!(frame.values[index], 0.0, "nothing was overridden");

    // ホットキーと同じ操作: トラッキングと口パクの一時停止 (省けば切り替え)。
    assert_eq!(
        client.call("tracking.pause", json!({}))["trackingPaused"],
        true
    );
    assert_eq!(
        client.call("tracking.pause", json!({ "paused": true }))["trackingPaused"],
        true
    );
    assert_eq!(
        client.call("lipsync.pause", json!({}))["lipSyncPaused"],
        true
    );
    client.call("tracking.calibrate", json!({}));
    let state = client.call("studio.state", json!({}));
    assert_eq!(state["scene"]["id"], game.as_str());
    assert_eq!(state["model"], "avatar.ugoe");
    assert_eq!(state["tracking"]["trackingPaused"], true);
}

#[test]
fn dresses_up_only_with_modules_from_the_profile() {
    let dir = Dir::new();
    let model = dir.model();
    let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
    let dress = dir.module("dress.ugoemod", fixtures::dress_module());
    let stray = dir.module("stray.ugoemod", fixtures::dress_module());
    let studio = studio();
    let (chat, _) = stage_scenes(&studio, &model, std::slice::from_ref(&hair));
    // ホットキーに割り当てたモジュールも着せ替えに使える。
    studio.with_stage(|stage, _| {
        stage.set_hotkeys(vec![Hotkey {
            keys: "F9".into(),
            action: HotkeyAction::DressUp {
                slot: "Outfit".into(),
                module: Some(dress.clone()),
            },
        }]);
    });
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);
    client.call("events.subscribe", json!({ "events": ["dressup.changed"] }));

    let list = client.call("dressup.list", json!({}));
    assert_eq!(
        list["slots"],
        json!([{ "slot": "Hair", "module": null }, { "slot": "Outfit", "module": null }])
    );
    let ids: Vec<&str> = list["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [hair.to_str().unwrap(), dress.to_str().unwrap()]);
    assert_eq!(list["modules"][1]["slot"], "Outfit");

    client.call(
        "dressup.set",
        json!({ "slot": "Outfit", "module": dress.to_str() }),
    );
    let notification = client.notification().unwrap();
    assert_eq!(notification["method"], "dressup.changed");
    assert_eq!(notification["params"]["modules"][0]["slot"], "Outfit");
    let list = client.call("dressup.list", json!({}));
    assert_eq!(list["slots"][1]["module"], dress.to_str().unwrap());
    assert_eq!(lock(&studio.stage).live(), Some(chat.as_str()));
    // プロファイルに無いパスは読ませない。
    assert_eq!(
        client.fail(
            "dressup.set",
            json!({ "slot": "Outfit", "module": stray.to_str() })
        ),
        INVALID_PARAMS
    );
    client.call("dressup.set", json!({ "slot": "Outfit", "module": null }));
    assert_eq!(
        client.notification().unwrap()["params"]["modules"],
        json!([])
    );
}

/// `pose.list` は出しているモデルのポーズのグループと表示している要素を返し、`pose.set` は操作ウィンドウの `show_pose` と同じく
/// 要素を選んでシーンに覚える (T0843)。
#[test]
fn lists_and_sets_the_pose_of_the_scene_on_air() {
    let dir = Dir::new();
    let model = dir.pose_model();
    let studio = studio();
    let mut reader = Client::connect(&studio);
    reader.approved(Permission::Read);
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);

    // 出しているシーンが無い。
    assert_eq!(client.call("pose.list", json!({})), json!({ "groups": [] }));
    let id = client.send("pose.set", json!({ "part": "ArmB" }));
    let response = client.response();
    assert_eq!(response["id"], id);
    assert_eq!(response["error"]["code"], OPERATION_FAILED);
    assert_eq!(response["error"]["data"]["code"], "noModel");

    let (chat, _) = stage_scenes(&studio, &model, &[]);
    assert_eq!(
        reader.call("pose.list", json!({})),
        json!({ "groups": [{ "parts": ["ArmA", "ArmB"], "module": null, "shown": "ArmA", "selected": "ArmA" }] })
    );
    assert_eq!(
        reader.fail("pose.set", json!({ "part": "ArmB" })),
        FORBIDDEN
    );

    assert_eq!(
        client.call("pose.set", json!({ "part": "ArmB" })),
        json!({})
    );
    let group = &client.call("pose.list", json!({}))["groups"][0];
    assert_eq!(
        (&group["shown"], &group["selected"]),
        (&json!("ArmB"), &json!("ArmB"))
    );
    assert_eq!(
        lock(&studio.stage).profile().scene(&chat).unwrap().pose,
        ["ArmB"]
    );

    let id = client.send("pose.set", json!({ "part": "Nope" }));
    let response = client.response();
    assert_eq!(response["id"], id);
    assert_eq!(response["error"]["code"], OPERATION_FAILED);
    assert_eq!(
        response["error"]["data"],
        json!({ "code": "unknownPose", "part": "Nope", "messageCode": "scene.unknown_pose", "args": { "part": "Nope" } })
    );
    assert_eq!(client.fail("pose.set", json!({})), INVALID_PARAMS);
}

/// `ArmB` を表示させるモーション `wave` (0.5 秒。`pose_and_effects` の `ArmA` と `ArmB` のパーツの不透明度のカーブ。T0848)。
fn wave_motion() -> ugoe::model::motion::Motion {
    use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
    let curve = |part: &str, v: f32| Curve {
        target: CurveTarget::PartOpacity(part.into()),
        segments: vec![
            Segment::Start {
                p: Point { t: 0.0, v },
            },
            Segment::Linear {
                to: Point { t: 0.5, v },
            },
        ],
        fade_in: None,
        fade_out: None,
    };
    Motion {
        name: "wave".into(),
        duration: 0.5,
        looping: false,
        fade_in: 0.0,
        fade_out: 0.0,
        curves: vec![curve("ArmA", 0.0), curve("ArmB", 1.0)],
        events: vec![],
    }
}

/// `pose.changed` は表示中の要素か選択が変わったときに送る: `pose.set`・モーションの切り替え (選択は変わらない)・
/// シーンの出し直し (選択に戻る)。中身は `pose.list` と同じ (T0848)。
#[test]
fn pose_changes_are_reported_with_the_shown_and_selected_parts() {
    let dir = Dir::new();
    let model = dir.pose_model();
    let studio = studio();
    let (_, game) = stage_scenes(&studio, &model, &[]);
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);
    client.call(
        "events.subscribe",
        json!({ "events": ["scene.changed", "pose.changed"] }),
    );
    let group = |shown: &str, selected: &str| json!({ "groups": [{ "parts": ["ArmA", "ArmB"], "module": null, "shown": shown, "selected": selected }] });

    // 刻んでも変わらなければ送らない。
    studio.tick(0.02, false);
    assert!(drain(&mut client).is_empty());

    client.call("motion.play", json!({ "motion": "wave" }));
    for _ in 0..40 {
        studio.tick(0.02, false);
    }
    let changes = drain(&mut client);
    assert_eq!(
        changes,
        [("pose.changed".to_owned(), group("ArmB", "ArmA"))],
        "the motion switched the arm once and it stays"
    );
    assert_eq!(client.call("pose.list", json!({})), group("ArmB", "ArmA"));

    // 表示している要素を選ぶ: 選択だけが変わっても送る。
    client.call("pose.set", json!({ "part": "ArmB" }));
    assert_eq!(
        drain(&mut client),
        [("pose.changed".to_owned(), group("ArmB", "ArmB"))]
    );
    client.call("pose.set", json!({ "part": "ArmA" }));
    assert_eq!(
        drain(&mut client),
        [("pose.changed".to_owned(), group("ArmA", "ArmA"))]
    );

    // モーションの後にシーンを切り替えると、`scene.changed` の後に新しいシーンの選択に戻ったことを送る。
    client.call("motion.play", json!({ "motion": "wave" }));
    for _ in 0..40 {
        studio.tick(0.02, false);
    }
    drain(&mut client);
    client.call("scene.activate", json!({ "scene": game }));
    let methods: Vec<String> = drain(&mut client).into_iter().map(|(m, _)| m).collect();
    assert_eq!(methods, ["scene.changed", "pose.changed"]);
    assert_eq!(client.call("pose.list", json!({})), group("ArmA", "ArmA"));
}

/// `dressup.list` は付けていないモジュールを、舞台と指揮役のロックを放してから読む (シーンの切り替えを待たせない。T0819)。
#[test]
fn lists_the_wardrobe_without_holding_the_stage() {
    let dir = Dir::new();
    let model = dir.model();
    let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
    let studio = studio();
    // 出しているのは Chat (モジュール無し) なので、Game の髪は読まないと分からない。
    stage_scenes(&studio, &model, std::slice::from_ref(&hair));
    let mut read = Vec::new();
    let list = methods::dressup_list_with(&studio, |path| {
        assert!(studio.stage.try_lock().is_ok(), "the stage is locked");
        assert!(
            studio.conductor.try_lock().is_ok(),
            "the conductor is locked"
        );
        assert!(
            studio.module_summaries.try_lock().is_ok(),
            "the summaries are locked"
        );
        read.push(path.to_owned());
        methods::read_module_value(path)
    });
    assert_eq!(read, std::slice::from_ref(&hair));
    assert_eq!(list["modules"][0]["id"], hair.to_str().unwrap());
    assert_eq!(list["modules"][0]["slot"], "Hair");
}

/// シーンの切り替えはファイルを舞台のロックの外で読み、読んでいる間も外部 API の要求に答える。
/// 読んでいる間に次の切り替えが来たら、先の切り替えは捨てる (T0834)。
#[test]
fn answers_while_a_scene_loads_and_keeps_the_later_switch() {
    let dir = Dir::new();
    let model = dir.model();
    let other = dir.0.path().join("other.ugoe");
    std::fs::copy(&model, &other).unwrap();
    let studio = studio();
    let (chat, game) = stage_scenes(&studio, &model, &[]);
    let far = studio.with_stage(|stage, conductor| {
        let id = stage.add_scene("Far");
        let mut scene = stage.profile().scene(&id).unwrap().clone();
        scene.model = Some(other.clone());
        stage.update_scene(scene, conductor).unwrap();
        id
    });
    let client = std::cell::RefCell::new(Client::connect(&studio));
    client.borrow_mut().approved(Permission::Control);

    let reads = std::cell::Cell::new(0);
    let read = |asset: Asset, path: &Path, extensions: &[&str], max: u64| {
        reads.set(reads.get() + 1);
        assert!(studio.stage.try_lock().is_ok(), "the stage is locked");
        assert!(
            studio.conductor.try_lock().is_ok(),
            "the conductor is locked"
        );
        let mut client = client.borrow_mut();
        // 読むだけの要求に答える (まだ前のシーン)。
        let list = client.call("scene.list", json!({}));
        assert_eq!(list["live"], chat.as_str());
        client.call("studio.state", json!({}));
        // 読んでいる間に次の切り替え (同じモデルなので読まずに出す)。
        client.call("scene.activate", json!({ "scene": game }));
        assert_eq!(lock(&studio.stage).live(), Some(game.as_str()));
        crate::stage::read_asset(asset, path, extensions, max)
    };
    studio
        .switch_scene_with(
            |stage, conductor| stage.begin_activate(&far, conductor).map(Some),
            Loading {
                read: &read,
                ..LOADING
            },
        )
        .unwrap();
    assert_eq!(reads.get(), 1);
    // 先に始めた Far は読み終えても出さない。
    assert_eq!(lock(&studio.stage).live(), Some(game.as_str()));
    assert_eq!(
        lock(&studio.conductor).scene().unwrap().model_name,
        "avatar.ugoe"
    );

    // 次の切り替えが来なければ、読み終えて出す。
    studio
        .switch_scene_with(
            |stage, conductor| stage.begin_activate(&far, conductor).map(Some),
            Loading {
                read: &|asset: Asset, path: &Path, extensions: &[&str], max: u64| {
                    assert!(studio.stage.try_lock().is_ok(), "the stage is locked");
                    crate::stage::read_asset(asset, path, extensions, max)
                },
                ..LOADING
            },
        )
        .unwrap();
    assert_eq!(lock(&studio.stage).live(), Some(far.as_str()));
    assert_eq!(
        lock(&studio.conductor).scene().unwrap().model_name,
        "other.ugoe"
    );
}

/// パペットの組み立ても舞台のロックの外で行い、組み立てている間も外部 API の要求に答える。
/// 組み立てている間に次の切り替えが来たら、先の組み立ての結果は差し込まない (T0837)。
#[test]
fn answers_while_a_puppet_is_assembled_and_drops_the_stale_one() {
    let dir = Dir::new();
    let model = dir.model();
    let other = dir.0.path().join("other.ugoe");
    std::fs::copy(&model, &other).unwrap();
    let studio = studio();
    let (chat, game) = stage_scenes(&studio, &model, &[]);
    let far = studio.with_stage(|stage, conductor| {
        let id = stage.add_scene("Far");
        let mut scene = stage.profile().scene(&id).unwrap().clone();
        scene.model = Some(other.clone());
        stage.update_scene(scene, conductor).unwrap();
        id
    });
    let revision = lock(&studio.conductor).scene().unwrap().revision;
    let client = std::cell::RefCell::new(Client::connect(&studio));
    client.borrow_mut().approved(Permission::Control);

    let assembled = std::cell::Cell::new(0);
    let assemble = |model: crate::conductor::NewModel,
                    modules: &[Arc<ugoe::model::module::Module>]| {
        assembled.set(assembled.get() + 1);
        assert_eq!(model.name, "other.ugoe");
        assert!(studio.stage.try_lock().is_ok(), "the stage is locked");
        assert!(
            studio.conductor.try_lock().is_ok(),
            "the conductor is locked"
        );
        let mut client = client.borrow_mut();
        // 読むだけの要求に答える (まだ前のシーン)。
        let list = client.call("scene.list", json!({}));
        assert_eq!(list["live"], chat.as_str());
        client.call("studio.state", json!({}));
        // 組み立てている間に次の切り替え (同じモデルなので読まずに出す)。
        client.call("scene.activate", json!({ "scene": game }));
        assert_eq!(lock(&studio.stage).live(), Some(game.as_str()));
        crate::conductor::assemble(model, modules)
    };
    studio
        .switch_scene_with(
            |stage, conductor| stage.begin_activate(&far, conductor).map(Some),
            Loading {
                assemble: &assemble,
                ..LOADING
            },
        )
        .unwrap();
    assert_eq!(assembled.get(), 1);
    // 先に始めた Far の組み立ての結果は差し込まない (指揮役は Game の 1 回だけ進む)。
    assert_eq!(lock(&studio.stage).live(), Some(game.as_str()));
    let conductor = lock(&studio.conductor);
    let scene = conductor.scene().unwrap();
    assert_eq!(
        (scene.model_name.as_str(), scene.revision),
        ("avatar.ugoe", revision + 1)
    );
    drop(conductor);

    // 次の切り替えが来なければ、組み立てた結果を差し込む。
    studio
        .switch_scene_with(
            |stage, conductor| stage.begin_activate(&far, conductor).map(Some),
            Loading {
                assemble: &|model, modules| {
                    assert!(studio.stage.try_lock().is_ok(), "the stage is locked");
                    crate::conductor::assemble(model, modules)
                },
                ..LOADING
            },
        )
        .unwrap();
    assert_eq!(lock(&studio.stage).live(), Some(far.as_str()));
    assert_eq!(
        lock(&studio.conductor).scene().unwrap().model_name,
        "other.ugoe"
    );
}

/// 操作ウィンドウが受けた `sceneLoading` の通知の `scene` を、届いた順に取り出す。
fn scene_loading_notices(studio: &Studio) -> Receiver<Value> {
    let (tx, rx) = mpsc::channel();
    let channel = tauri::ipc::Channel::new(move |body| {
        if let tauri::ipc::InvokeResponseBody::Json(json) = body {
            let value: Value = serde_json::from_str(&json).unwrap();
            if value["kind"] == "sceneLoading" {
                let _ = tx.send(value["scene"].clone());
            }
        }
        Ok(())
    });
    studio.monitors.insert("main", channel);
    rx
}

/// ホットキーと外部 API が始めた切り替えも、読み始めと終わり (失敗・捨てたものも) を操作ウィンドウに知らせる (T0841)。
#[test]
fn tells_the_control_window_while_a_switch_is_loading() {
    let dir = Dir::new();
    let model = dir.model();
    let other = dir.0.path().join("other.ugoe");
    std::fs::copy(&model, &other).unwrap();
    let studio = studio();
    let (chat, game) = stage_scenes(&studio, &model, &[]);
    let add = |name: &str, path: &Path| {
        studio.with_stage(|stage, conductor| {
            let id = stage.add_scene(name);
            let mut scene = stage.profile().scene(&id).unwrap().clone();
            scene.model = Some(path.to_owned());
            stage.update_scene(scene, conductor).unwrap();
            id
        })
    };
    let far = add("Far", &other);
    let gone = add("Gone", &dir.0.path().join("gone.ugoe"));
    let notices = scene_loading_notices(&studio);
    let drain = || notices.try_iter().collect::<Vec<_>>();
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);

    // 読むファイルが無い (同じモデル) なら、握ったまま差し込むので知らせない。
    client.call("scene.activate", json!({ "scene": game }));
    assert_eq!(drain(), Vec::<Value>::new());
    // 外部 API の切り替え: 読み始めと終わり。
    client.call("scene.activate", json!({ "scene": far }));
    assert_eq!(drain(), [json!(far), Value::Null]);

    // ホットキーの切り替え: 読めなくても終わりを知らせる。
    let error = studio.perform_hotkey(&HotkeyAction::Scene {
        scene: gone.clone(),
    });
    assert!(error.is_some());
    assert_eq!(drain(), [json!(gone), Value::Null]);
    assert!(
        studio
            .perform_hotkey(&HotkeyAction::Scene {
                scene: chat.clone()
            })
            .is_none()
    );
    assert_eq!(drain(), [json!(chat), Value::Null]);

    // 読んでいる間に後から始めた切り替えが済んだら、そこで終わりを知らせ、先の切り替えの終わりでは知らせない。
    let read = |asset: Asset, path: &Path, extensions: &[&str], max: u64| {
        assert_eq!(notices.try_recv().unwrap(), json!(far));
        assert!(
            studio
                .perform_hotkey(&HotkeyAction::Scene {
                    scene: chat.clone()
                })
                .is_none()
        );
        assert_eq!(notices.try_recv().unwrap(), Value::Null);
        crate::stage::read_asset(asset, path, extensions, max)
    };
    studio
        .switch_scene_with(
            |stage, conductor| stage.begin_activate(&far, conductor).map(Some),
            Loading {
                read: &read,
                ..LOADING
            },
        )
        .unwrap();
    assert_eq!(lock(&studio.stage).live(), Some(chat.as_str()));
    assert_eq!(drain(), Vec::<Value>::new());
}

/// `dressup.list` は付けていないモジュールの概要を覚え、ファイルの時刻と大きさが変わらなければ読み直さない (T0833)。
#[test]
fn remembers_the_wardrobe_until_the_file_changes() {
    let dir = Dir::new();
    let model = dir.model();
    let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
    let studio = studio();
    stage_scenes(&studio, &model, std::slice::from_ref(&hair));
    // 読んだ回数を数える。
    let list = |studio: &Studio| {
        let mut reads = 0;
        let value = methods::dressup_list_with(studio, |path| {
            reads += 1;
            methods::read_module_value(path)
        });
        (value, reads)
    };
    let (first, reads) = list(&studio);
    assert_eq!(reads, 1);
    assert_eq!(first["modules"][0]["id"], hair.to_str().unwrap());
    assert_eq!(list(&studio), (first.clone(), 0));
    assert_eq!(list(&studio), (first, 0));

    // 書き換えた (時刻が変わった) ら読み直す。
    let mut module = fixtures::long_hair_module();
    module.name = "Braid".into();
    dir.module("hair.ugoemod", module);
    let file = std::fs::File::options().write(true).open(&hair).unwrap();
    file.set_modified(std::time::SystemTime::now() + Duration::from_secs(60))
        .unwrap();
    drop(file);
    let (changed, reads) = list(&studio);
    assert_eq!(reads, 1, "read again");
    assert_eq!(changed["modules"][0]["name"], "Braid");
    assert_eq!(list(&studio).1, 0);

    // 消えたら一覧から外し、忘れる。
    std::fs::remove_file(&hair).unwrap();
    let gone = methods::dressup_list_with(&studio, |_| panic!("read a missing file"));
    assert_eq!(gone["modules"], json!([]));
    assert!(lock(&studio.module_summaries).is_empty());
}

/// 覚える概要の数には上限があり、着せ替えに使えなくなったモジュールは忘れる (T0833)。
#[test]
fn remembers_a_bounded_wardrobe() {
    let dir = Dir::new();
    let model = dir.model();
    let studio = studio();
    let count = MAX_MODULE_SUMMARIES + 8;
    let paths: Vec<PathBuf> = (0..count)
        .map(|i| {
            let path = dir.0.path().join(format!("m{i}.ugoemod"));
            std::fs::write(&path, b"x").unwrap();
            path
        })
        .collect();
    // Game のシーンが全部を付ける (出しているのは Chat なので読まれない)。
    let (_, game) = stage_scenes(&studio, &model, &paths);
    let fake =
        |path: &Path| Some(json!({ "id": path.to_string_lossy(), "name": "m", "slot": "Hair" }));
    let list = methods::dressup_list_with(&studio, fake);
    assert_eq!(list["modules"].as_array().unwrap().len(), count);
    assert_eq!(lock(&studio.module_summaries).len(), MAX_MODULE_SUMMARIES);

    studio.with_stage(|stage, conductor| {
        let mut scene = stage.profile().scene(&game).unwrap().clone();
        scene.modules.truncate(2);
        stage.update_scene(scene, conductor).unwrap();
    });
    methods::dressup_list_with(&studio, fake);
    assert_eq!(lock(&studio.module_summaries).len(), 2);
}

#[test]
fn notifies_subscribed_events_only() {
    let dir = Dir::new();
    let model = dir.model();
    let studio = studio();
    let (chat, game) = stage_scenes(&studio, &model, &[]);
    let mut watcher = Client::connect(&studio);
    watcher.approved(Permission::Read);
    let mut quiet = Client::connect(&studio);
    quiet.approved(Permission::Control);
    assert_eq!(
        watcher.fail("events.subscribe", json!({ "events": ["nope"] })),
        INVALID_PARAMS
    );
    let subscribed = watcher.call(
        "events.subscribe",
        json!({ "events": ["scene.changed", "scene.listChanged", "expression.changed", "motion.started", "motion.finished", "motion.event", "tracking.changed"] }),
    );
    assert_eq!(subscribed["events"].as_array().unwrap().len(), 7);

    quiet.call("scene.activate", json!({ "scene": game }));
    let n = watcher.notification().unwrap();
    assert_eq!(
        (n["method"].as_str(), &n["params"]["scene"]["id"]),
        (Some("scene.changed"), &json!(game))
    );
    assert!(quiet.take().is_none(), "not subscribed");
    // シーンの一覧の変化 (名前を変える)。
    studio.with_stage(|stage, conductor| {
        let mut scene = stage.profile().scene(&chat).unwrap().clone();
        scene.name = "Talk".into();
        stage.update_scene(scene, conductor).unwrap();
    });
    let n = watcher.notification().unwrap();
    assert_eq!(n["method"], "scene.listChanged");
    assert_eq!(n["params"]["scenes"][0]["name"], "Talk");

    // 演技の変化は刻みの後に通知する。
    quiet.call("expression.set", json!({ "expression": "Smile" }));
    quiet.call(
        "motion.play",
        json!({ "motion": "Wave", "priority": "force" }),
    );
    quiet.call("tracking.pause", json!({ "paused": true }));
    studio.tick(0.02, false);
    let mut methods: Vec<String> = Vec::new();
    while let Some(n) = watcher.notification() {
        methods.push(n["method"].as_str().unwrap().to_owned());
        if n["method"] == "motion.event" {
            assert_eq!(
                n["params"],
                json!({ "motion": "Wave", "value": "wave-start", "time": 0.0 })
            );
        }
    }
    methods.sort();
    assert_eq!(
        methods,
        [
            "expression.changed",
            "motion.event",
            "motion.started",
            "tracking.changed"
        ]
    );
    // 終わったら motion.finished。
    for _ in 0..40 {
        studio.tick(0.02, false);
    }
    let mut finished = false;
    while let Some(n) = watcher.notification() {
        finished |= n["method"] == "motion.finished";
    }
    assert!(finished);
    watcher.call("events.unsubscribe", json!({ "events": ["scene.changed"] }));
    quiet.call("scene.activate", json!({ "scene": chat }));
    assert!(watcher.notification().is_none());
}

/// 届いている通知を、届いた順に (メソッドと引数)。
fn drain(client: &mut Client) -> Vec<(String, Value)> {
    std::iter::from_fn(|| client.notification())
        .map(|n| {
            (
                n["method"].as_str().unwrap().to_owned(),
                n["params"].clone(),
            )
        })
        .collect()
}

/// モーションの通知を取りこぼさず、順に送る: 同じモーションを始め直しても終わりと始まりを知らせる。
/// 終わりで発火したイベントは、そのモーションの名前で、終わりの知らせより先に届く。
#[test]
fn motion_notifications_cover_restarts_and_events_at_the_end() {
    let dir = Dir::new();
    let model = dir.model();
    let studio = studio();
    stage_scenes(&studio, &model, &[]);
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);
    client.call(
        "events.subscribe",
        json!({ "events": ["motion.started", "motion.finished", "motion.event"] }),
    );
    let started = |priority: &str| {
        (
            "motion.started".to_owned(),
            json!({ "motion": "Wave", "priority": priority }),
        )
    };
    let event = |value: &str, time: f64| {
        (
            "motion.event".to_owned(),
            json!({ "motion": "Wave", "value": value, "time": time }),
        )
    };
    let finished = ("motion.finished".to_owned(), json!({ "motion": "Wave" }));

    client.call(
        "motion.play",
        json!({ "motion": "Wave", "priority": "force" }),
    );
    studio.tick(0.02, false);
    assert_eq!(
        drain(&mut client),
        [started("force"), event("wave-start", 0.0)]
    );
    // 同じ名前と優先度で始め直す。
    client.call(
        "motion.play",
        json!({ "motion": "Wave", "priority": "force" }),
    );
    studio.tick(0.02, false);
    assert_eq!(
        drain(&mut client),
        [finished.clone(), started("force"), event("wave-start", 0.0)]
    );
    // 終わるまで進める。
    let mut tail = Vec::new();
    for _ in 0..40 {
        studio.tick(0.02, false);
        tail.extend(drain(&mut client));
    }
    assert_eq!(tail, [event("wave-end", 0.5), finished]);
}

/// 新しいモーションに替えた後、フェードアウト中のモーションが発火したイベントは、そのモーションの名前で送る (T0821)。
#[test]
fn events_of_a_motion_fading_out_carry_its_name() {
    let dir = Dir::new();
    let model = dir.model();
    let studio = studio();
    stage_scenes(&studio, &model, &[]);
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);
    client.call(
        "events.subscribe",
        json!({ "events": ["motion.started", "motion.finished", "motion.event"] }),
    );
    let note = |method: &str, params: Value| (method.to_owned(), params);
    client.call(
        "motion.play",
        json!({ "motion": "Bow", "priority": "force" }),
    );
    studio.tick(0.02, false);
    client.call(
        "motion.play",
        json!({ "motion": "Wave", "priority": "force" }),
    );
    let mut notes = drain(&mut client);
    for _ in 0..40 {
        studio.tick(0.02, false);
        notes.extend(drain(&mut client));
    }
    assert_eq!(
        notes,
        [
            note(
                "motion.started",
                json!({ "motion": "Bow", "priority": "force" })
            ),
            note("motion.finished", json!({ "motion": "Bow" })),
            note(
                "motion.started",
                json!({ "motion": "Wave", "priority": "force" })
            ),
            note(
                "motion.event",
                json!({ "motion": "Wave", "value": "wave-start", "time": 0.0 })
            ),
            note(
                "motion.event",
                json!({ "motion": "Bow", "value": "bow-late", "time": 0.25 })
            ),
            note(
                "motion.event",
                json!({ "motion": "Wave", "value": "wave-end", "time": 0.5 })
            ),
            note("motion.finished", json!({ "motion": "Wave" })),
        ]
    );
}

/// シーンを切り替えて付けているモジュールが変わったら、`scene.changed` に加えて `dressup.changed` も送る。
#[test]
fn switching_to_a_scene_with_other_modules_reports_the_dress_up() {
    let dir = Dir::new();
    let model = dir.model();
    let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
    let studio = studio();
    let (chat, game) = stage_scenes(&studio, &model, std::slice::from_ref(&hair));
    let mut client = Client::connect(&studio);
    client.approved(Permission::Control);
    client.call(
        "events.subscribe",
        json!({ "events": ["scene.changed", "dressup.changed"] }),
    );
    client.call("scene.activate", json!({ "scene": game }));
    let methods: Vec<String> = drain(&mut client).into_iter().map(|(m, _)| m).collect();
    assert_eq!(methods, ["scene.changed", "dressup.changed"]);
    // 同じモジュールの組のシーンどうし (どちらも無し) なら `scene.changed` だけ。
    client.call("scene.activate", json!({ "scene": chat }));
    let notifications = drain(&mut client);
    assert_eq!(notifications[1].1["modules"], json!([]));
    studio.with_stage(|stage, conductor| {
        let mut scene = stage.profile().scene(&game).unwrap().clone();
        scene.modules.clear();
        stage.update_scene(scene, conductor).unwrap();
    });
    client.call("scene.activate", json!({ "scene": game }));
    let methods: Vec<String> = drain(&mut client).into_iter().map(|(m, _)| m).collect();
    assert_eq!(methods, ["scene.changed"]);
}

// --- 本物の待ち受け ---

/// 待ち受けへ生の HTTP の要求を送り、応答の状態を返す。
fn http_status(port: u16, request: &str) -> u16 {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    line.split(' ').nth(1).unwrap().parse().unwrap()
}

fn upgrade(port: u16, extra: &str) -> String {
    upgrade_at(port, "/api", extra)
}

fn upgrade_at(port: u16, path: &str, extra: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n{extra}\r\n"
    )
}

/// フレームの種類 (RFC 6455 5.2)。
mod opcode {
    pub const TEXT: u8 = 0x1;
    pub const BINARY: u8 = 0x2;
    pub const CLOSE: u8 = 0x8;
    pub const PING: u8 = 0x9;
    pub const PONG: u8 = 0xA;
}

/// クライアントのフレーム (マスク付き) を作る。
fn client_frame(fin: bool, op: u8, payload: &[u8]) -> Vec<u8> {
    let mask = [0x12, 0x34, 0x56, 0x78];
    let mut out = vec![if fin { 0x80 } else { 0 } | op];
    match payload.len() {
        n if n < 126 => out.push(0x80 | n as u8),
        n if n <= 0xFFFF => {
            out.push(0x80 | 126);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            out.push(0x80 | 127);
            out.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    out.extend_from_slice(&mask);
    out.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    out
}

/// 最小の WebSocket のクライアント (フレームをバイト列で書き、サーバの返すフレームをそのまま読む)。
struct Socket(BufReader<TcpStream>);

impl Socket {
    fn open(port: u16) -> Self {
        Self::open_at(port, "/api", "")
    }

    fn open_at(port: u16, path: &str, extra: &str) -> Self {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .write_all(upgrade_at(port, path, extra).as_bytes())
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            head.push_str(&line);
        }
        assert!(head.starts_with("HTTP/1.1 101"), "{head}");
        assert!(head.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
        Self(reader)
    }

    fn send(&mut self, op: u8, payload: &[u8]) {
        self.0
            .get_mut()
            .write_all(&client_frame(true, op, payload))
            .unwrap();
    }

    fn call(&mut self, value: Value) {
        self.send(opcode::TEXT, value.to_string().as_bytes());
    }

    /// サーバのフレーム (種類と中身)。
    fn read(&mut self) -> (u8, Vec<u8>) {
        let mut head = [0u8; 2];
        self.0.read_exact(&mut head).unwrap();
        assert_eq!(head[1] & 0x80, 0, "server frames are not masked");
        let len = match head[1] & 0x7F {
            126 => {
                let mut b = [0u8; 2];
                self.0.read_exact(&mut b).unwrap();
                usize::from(u16::from_be_bytes(b))
            }
            127 => {
                let mut b = [0u8; 8];
                self.0.read_exact(&mut b).unwrap();
                usize::try_from(u64::from_be_bytes(b)).unwrap()
            }
            n => usize::from(n),
        };
        let mut payload = vec![0u8; len];
        self.0.read_exact(&mut payload).unwrap();
        (head[0] & 0x0F, payload)
    }

    fn read_json(&mut self) -> Value {
        let (op, payload) = self.read();
        assert_eq!(op, opcode::TEXT);
        serde_json::from_slice(&payload).unwrap()
    }
}

#[test]
fn the_listener_checks_requests_and_speaks_websocket() {
    let studio = studio();
    lock(&studio.api.0).settings.enabled = true;
    let server = server::Server::start(Arc::clone(&studio), 0).unwrap();
    let port = server.addrs()[0].port();
    assert!(server.addrs().iter().all(|a| a.ip().is_loopback()));

    // HTTP の検査
    assert_eq!(
        http_status(
            port,
            &format!("GET /nope HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
        ),
        404
    );
    assert_eq!(
        http_status(
            port,
            &format!("GET /api HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
        ),
        426
    );
    assert_eq!(
        http_status(
            port,
            &upgrade(port, "").replace("127.0.0.1", "attacker.example")
        ),
        403
    );
    assert_eq!(
        http_status(port, &upgrade(port, "Origin: https://attacker.example\r\n")),
        403
    );
    assert_eq!(
        http_status(port, &upgrade(port, "X-Forwarded-For: 203.0.113.1\r\n")),
        403
    );
    assert_eq!(
        http_status(
            port,
            &upgrade(port, &format!("Origin: http://localhost:{port}\r\n"))
        ),
        101,
        "the studio's own pages may connect"
    );

    // WebSocket の流れ: 名乗る → 承認 → 操作 → ping → 閉じる。
    let mut socket = Socket::open(port);
    socket.call(json!({ "jsonrpc": "2.0", "id": 1, "method": "session.hello", "params": { "name": "Deck", "token": TOKEN } }));
    let pending = (0..100)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(10));
            studio.api_view().pending.first().cloned()
        })
        .unwrap();
    studio
        .approve_api_client(&pending.id, Permission::Read)
        .unwrap();
    let hello = socket.read_json();
    assert_eq!(hello["id"], 1);
    assert_eq!(hello["result"]["permission"], "read");
    socket.call(json!({ "jsonrpc": "2.0", "id": 2, "method": "studio.state" }));
    let state = socket.read_json();
    assert_eq!(state["id"], 2);
    assert_eq!(state["result"]["scene"], Value::Null);
    socket.send(opcode::TEXT, b"{");
    assert_eq!(socket.read_json()["error"]["code"], rpc::PARSE_ERROR);
    socket.send(opcode::PING, b"hi");
    assert_eq!(socket.read(), (opcode::PONG, b"hi".to_vec()));
    // 通知 (`id` の無い要求) には応答しない。
    socket.call(json!({ "jsonrpc": "2.0", "method": "studio.state" }));
    socket.call(json!({ "jsonrpc": "2.0", "id": 3, "method": "scene.list" }));
    assert_eq!(socket.read_json()["id"], 3);
    socket.send(opcode::BINARY, b"x");
    let (op, payload) = socket.read();
    assert_eq!(
        (op, &payload[..2]),
        (
            opcode::CLOSE,
            &ws::close::UNSUPPORTED_DATA.to_be_bytes()[..]
        )
    );

    // 大きすぎるフレーム・マスクの無いフレーム・UTF-8 でないテキストは、理由を付けて閉じる。
    for (frame, code) in [
        (
            client_frame(true, opcode::TEXT, &vec![b'a'; ws::MAX_FRAME_BYTES + 1]),
            ws::close::TOO_BIG,
        ),
        (vec![0x81, 0x01, b'a'], ws::close::PROTOCOL_ERROR),
        (
            client_frame(true, opcode::TEXT, &[0xff]),
            ws::close::INVALID_DATA,
        ),
    ] {
        let mut socket = Socket::open(port);
        socket.0.get_mut().write_all(&frame).unwrap();
        let (op, payload) = socket.read();
        assert_eq!(
            (op, &payload[..2]),
            (opcode::CLOSE, &code.to_be_bytes()[..]),
            "{code}"
        );
    }
    // 分割したメッセージは組み立てる。
    let mut socket = Socket::open(port);
    let text = json!({ "jsonrpc": "2.0", "id": 9, "method": "studio.state" }).to_string();
    let (head, tail) = text.as_bytes().split_at(10);
    let mut bytes = client_frame(false, opcode::TEXT, head);
    bytes.extend(client_frame(true, 0x0, tail));
    socket.0.get_mut().write_all(&bytes).unwrap();
    assert_eq!(socket.read_json()["error"]["code"], NOT_AUTHENTICATED);

    // 止めると待ち受けをやめ、接続も閉じる。
    let mut socket = Socket::open(port);
    let key = {
        let client = &studio.api_view().clients[0];
        lock(&studio.api.0)
            .settings
            .clients
            .iter()
            .find(|c| c.id == client.id)
            .unwrap()
            .key
            .clone()
    };
    socket.call(json!({ "jsonrpc": "2.0", "id": 1, "method": "session.hello", "params": { "name": "Deck", "key": key } }));
    assert_eq!(socket.read_json()["result"]["permission"], "read");
    lock(&studio.api.0).server = Some(server);
    lock(&studio.api.0).settings.enabled = true;
    studio.set_api_enabled(false);
    let (op, payload) = socket.read();
    assert_eq!(
        (op, &payload[..2]),
        (opcode::CLOSE, &CLOSE_GOING_AWAY.to_be_bytes()[..])
    );
    assert!(!studio.api_view().listening);
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "no longer listening"
    );
}

/// 止めた待ち受けは、`stop` から戻ったときにはポートを手放している (すぐに同じポートで待ち受け直せる。T0961)。
#[test]
fn a_stopped_listener_has_released_its_port() {
    let studio = studio();
    for _ in 0..50 {
        let server = server::Server::start(Arc::clone(&studio), 0).unwrap();
        let addrs = server.addrs().to_vec();
        server.stop();
        for addr in addrs {
            if let Err(e) = std::net::TcpListener::bind(addr) {
                panic!("{addr} is still held after stop: {e}");
            }
        }
    }
}

#[test]
fn turning_the_api_on_saves_the_choice_and_listens() {
    let dir = Dir::new();
    let path = dir.0.path().join(SETTINGS_FILE);
    let studio = Arc::new(Studio::default());
    studio.load_api(Some(path.clone()));
    let view = studio.api_view();
    assert!(!view.enabled && !view.listening, "off by default");
    let token = studio.api_token();
    assert_eq!(token.len(), 64);
    // 使われていないポートを選ぶ。
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    studio.set_api_port(port).unwrap();
    assert!(studio.set_api_port(80).is_err());
    studio.set_api_enabled(true);
    let view = studio.api_view();
    assert!(view.enabled && view.listening, "{view:?}");
    assert_eq!(
        http_status(
            port,
            &format!("GET / HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n")
        ),
        404
    );
    let (saved, _) = store::ApiSettings::read(&path);
    assert!(saved.enabled);
    assert_eq!((saved.port, saved.token.as_str()), (port, token.as_str()));
    // 読み直すと、覚えた設定で待ち受ける。
    studio.set_api_enabled(false);
    let again = Arc::new(Studio::default());
    lock(&again.api.0).settings.enabled = true;
    again.load_api(Some(path.clone()));
    assert!(!again.api_view().listening, "the saved choice (off) wins");
    again.set_api_enabled(true);
    let reloaded = Arc::new(Studio::default());
    again.set_api_enabled(false);
    std::fs::write(
        &path,
        std::fs::read_to_string(&path)
            .unwrap()
            .replace("\"enabled\": false", "\"enabled\": true"),
    )
    .unwrap();
    reloaded.load_api(Some(path));
    assert!(reloaded.api_view().listening);
    assert_eq!(reloaded.api_token(), token);
    reloaded.set_api_enabled(false);
}

/// 読めない設定 (壊れたもの・新しい版のスタジオが書いたもの) は、起動しただけでは書き換えない
/// (承認したクライアントの鍵を黙って失わない。利用者が設定を変えたときに初めて書く)。
#[test]
fn an_unreadable_settings_file_is_not_overwritten_at_startup() {
    let dir = Dir::new();
    let path = dir.0.path().join(SETTINGS_FILE);
    for text in [
        "{",
        r#"{"format":"ugoe-studio-api","version":99,"clients":[]}"#,
    ] {
        std::fs::write(&path, text).unwrap();
        let studio = Arc::new(Studio::default());
        studio.load_api(Some(path.clone()));
        let view = studio.api_view();
        assert!(view.error.is_some(), "{text}");
        assert!(!view.enabled && !view.listening, "{text}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        // 設定を変えたら書く。
        studio.set_api_port(30001).unwrap();
        let (saved, problem) = store::ApiSettings::read(&path);
        assert!(problem.is_none(), "{text}");
        assert_eq!(saved.port, 30001);
        assert!(studio.api_view().error.is_none(), "{text}");
    }
}

/// Schema はすべてのメソッド (権限も) と通知を定め、送る要求の例も Schema に合う。
#[test]
fn the_schema_covers_every_method_and_event() {
    let defs = schema()["$defs"].as_object().unwrap();
    let requests = schema()["$defs"]["request"]["oneOf"].as_array().unwrap();
    let documented: Vec<&str> = requests
        .iter()
        .map(|r| r["properties"]["method"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(
        documented, SCHEMA_METHODS,
        "the schema lists every method once"
    );
    for (request, method) in requests.iter().zip(&documented) {
        assert!(defs.contains_key(&format!("{method}.params")), "{method}");
        assert!(defs.contains_key(&format!("{method}.result")), "{method}");
        let expected = match methods::permission(method) {
            Some(Permission::Read) => "Permission: read.",
            Some(Permission::Control) => "Permission: control.",
            None => {
                assert_eq!(
                    *method, "session.hello",
                    "methods.rs does not know {method}"
                );
                "Permission: none."
            }
        };
        assert_eq!(request["description"], expected, "{method}");
    }
    for event in EVENTS {
        assert!(defs.contains_key(&format!("{event}.event")), "{event}");
    }
    assert_eq!(schema()["$defs"]["eventName"]["enum"], json!(EVENTS));
    for request in [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "session.hello", "params": { "name": "Deck", "token": TOKEN, "permission": "control" } }),
        json!({ "jsonrpc": "2.0", "id": "a", "method": "parameter.set", "params": { "values": [{ "id": "AngleX", "value": 10, "weight": 0.5 }], "duration": 2 } }),
        json!({ "jsonrpc": "2.0", "method": "motion.stop" }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "dressup.set", "params": { "slot": "Hair", "module": null } }),
    ] {
        conforms(&request, schema()).unwrap_or_else(|e| panic!("{request}: {e}"));
    }
    assert!(
        conforms(
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "nope" }),
            schema()
        )
        .is_err()
    );
    assert!(
        conforms(
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "dressup.set", "params": { "slot": 1 } }),
            schema()
        )
        .is_err()
    );
}

/// Schema に並べるメソッド (`session.hello` と、`methods::permission` が知るもの)。
const SCHEMA_METHODS: &[&str] = &[
    "session.hello",
    "studio.state",
    "scene.list",
    "scene.activate",
    "expression.list",
    "expression.set",
    "expression.add",
    "expression.remove",
    "expression.toggle",
    "expression.clear",
    "motion.list",
    "motion.play",
    "motion.stop",
    "parameter.list",
    "parameter.set",
    "dressup.list",
    "dressup.set",
    "pose.list",
    "pose.set",
    "tracking.pause",
    "lipsync.pause",
    "tracking.calibrate",
    "events.subscribe",
    "events.unsubscribe",
];

// --- ブラウザソース (T0808) ---

/// 待ち受けへ GET を送り、応答の状態・頭・本文を返す (応答は `Connection: close`)。
fn http_get(port: u16, path: &str, extra: &str) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}\r\n").as_bytes(),
        )
        .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(response[..split].to_vec()).unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head, response[split + 4..].to_vec())
}

/// ブラウザソースを出しているスタジオ (操作 API は切。ページのファイルは偽物)。待ち受けのポートとトークンを返す。
fn source_studio() -> (Arc<Studio>, u16, String) {
    let studio = studio();
    studio.sources.set_assets(|path| match path {
        "source.html" => Some(b"<!doctype html><title>source</title>".to_vec()),
        "assets/source-1.js" => Some(b"export {};".to_vec()),
        _ => None,
    });
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    lock(&studio.api.0).settings.source.token = store::generate_secret().unwrap();
    studio.set_api_port(port).unwrap();
    studio.set_source_enabled(true);
    let token = lock(&studio.api.0).settings.source.token.clone();
    assert_eq!(token.len(), 64);
    (studio, port, token)
}

#[test]
fn the_browser_source_has_its_own_switch_and_token() {
    let studio = studio();
    let view = studio.api_view();
    assert!(
        !view.source.enabled && !view.source.listening,
        "off by default"
    );
    let (studio, port, token) = source_studio();
    let view = studio.api_view();
    assert!(view.source.enabled && view.source.listening, "{view:?}");
    assert!(!view.enabled, "the API stays off");
    assert_eq!(
        studio.source_url(),
        format!("http://127.0.0.1:{port}/source/{token}/")
    );
    // 操作 API は切ってあるので答えない。API のトークンではページを出さない。
    assert_eq!(http_status(port, &upgrade(port, "")), 404);
    let api_token = studio.api_token();
    assert_eq!(http_get(port, &format!("/source/{api_token}/"), "").0, 404);
    assert_eq!(http_get(port, "/source/", "").0, 404);
    assert_eq!(
        http_get(port, &format!("/source/{}/", &token[..63]), "").0,
        404
    );
    // 正しいトークン: ページ・ファイル・末尾の `/` を付ける転送。
    let (status, head, body) = http_get(port, &format!("/source/{token}/"), "");
    assert_eq!(status, 200, "{head}");
    assert!(head.contains("Content-Type: text/html; charset=utf-8"));
    assert!(head.contains("Referrer-Policy: no-referrer"));
    assert!(head.contains("Content-Security-Policy: default-src 'none'"));
    assert!(body.starts_with(b"<!doctype html>"));
    let (status, head, _) = http_get(port, &format!("/source/{token}"), "");
    assert_eq!(status, 301);
    assert!(
        head.contains(&format!("Location: /source/{token}/\r\n")),
        "{head}"
    );
    let (status, head, body) = http_get(port, &format!("/source/{token}/assets/source-1.js"), "");
    assert_eq!((status, body.as_slice()), (200, &b"export {};"[..]));
    assert!(head.contains("Content-Type: text/javascript"));
    for missing in [
        "assets/nope.js",
        "assets/../source.html",
        "output.html",
        "index.html",
        "scene/1/model",
    ] {
        assert_eq!(
            http_get(port, &format!("/source/{token}/{missing}"), "").0,
            404,
            "{missing}"
        );
    }
    // 開いた Web ページ (別の Origin) と、ループバックでない Host からは断る。
    assert_eq!(
        http_get(
            port,
            &format!("/source/{token}/"),
            "Origin: https://evil.example\r\n"
        )
        .0,
        403
    );
    assert_eq!(
        http_status(
            port,
            &format!("GET /source/{token}/ HTTP/1.1\r\nHost: evil.example:{port}\r\n\r\n")
        ),
        403
    );
    assert_eq!(
        http_status(
            port,
            &format!("POST /source/{token}/ HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
        ),
        405
    );

    // API を入れて切っても、ブラウザソースは出し続ける。
    studio.set_api_enabled(true);
    assert_eq!(http_status(port, &upgrade(port, "Origin: x\r\n")), 403);
    studio.set_api_enabled(false);
    assert_eq!(http_get(port, &format!("/source/{token}/"), "").0, 200);
    assert_eq!(http_status(port, &upgrade(port, "")), 404);

    // トークンを作り直すと古い URL は使えない。
    let url = studio.regenerate_source_token().unwrap();
    let fresh = lock(&studio.api.0).settings.source.token.clone();
    assert_ne!(fresh, token);
    assert!(url.ends_with(&format!("/source/{fresh}/")));
    assert_eq!(http_get(port, &format!("/source/{token}/"), "").0, 404);
    assert_eq!(http_get(port, &format!("/source/{fresh}/"), "").0, 200);

    // どちらも切れば待ち受けない。
    studio.set_source_enabled(false);
    assert!(!studio.api_view().source.listening);
    std::thread::sleep(Duration::from_millis(100));
    assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
}

#[test]
fn the_browser_source_serves_the_scene_and_streams_frames() {
    let dir = Dir::new();
    let model = dir.model();
    let (studio, port, token) = source_studio();
    let (_, game) = stage_scenes(&studio, &model, &[]);
    let scene = lock(&studio.conductor).scene().unwrap().clone();

    // 今のシーンのファイルだけを出す。
    let base = format!("/source/{token}/scene");
    let (status, head, body) = http_get(port, &format!("{base}/{}/model", scene.revision), "");
    assert_eq!(status, 200);
    assert!(head.contains("Content-Type: application/octet-stream"));
    assert_eq!(body, std::fs::read(&model).unwrap());
    assert_eq!(
        http_get(port, &format!("{base}/{}/model", scene.revision + 1), "").0,
        404
    );
    assert_eq!(
        http_get(port, &format!("{base}/{}/module/0", scene.revision), "").0,
        404
    );
    assert_eq!(
        http_get(port, &format!("{base}/{}/background", scene.revision), "").0,
        404
    );
    assert_eq!(http_get(port, &format!("{base}/x/model"), "").0, 404);

    // ページの WebSocket (ページ自身の Origin)。最初にシーン、刻むとフレーム。
    let origin = format!("Origin: http://127.0.0.1:{port}\r\n");
    let mut socket = Socket::open_at(port, &format!("/source/{token}/frames"), &origin);
    let first = socket.read_json();
    assert_eq!(first["kind"], "scene");
    assert_eq!(first["revision"], scene.revision);
    assert_eq!(studio.api_view().source.connections, 1);
    studio.tick(1.0 / 60.0, false);
    studio.tick(1.0 / 30.0, false);
    let frames: Vec<_> = (0..2)
        .map(|_| {
            let (op, payload) = socket.read();
            assert_eq!(op, opcode::BINARY);
            crate::stream::Frame::decode(&payload).unwrap()
        })
        .collect();
    assert_eq!(frames[1].seq, frames[0].seq + 1);
    assert_eq!(frames[0].revision, scene.revision);
    assert!((frames[1].dt - 1.0 / 30.0).abs() < 1e-6);

    // シーンを切り替えると、次のフレームより先にシーンが届く。
    studio.with_stage(|stage, conductor| stage.activate(&game, conductor).unwrap());
    studio.tick(1.0 / 60.0, false);
    let next = socket.read_json();
    assert_eq!(next["kind"], "scene");
    let (op, payload) = socket.read();
    assert_eq!(op, opcode::BINARY);
    assert_eq!(
        crate::stream::Frame::decode(&payload).unwrap().revision,
        next["revision"].as_u64().unwrap() as u32
    );

    // ページは描画の負荷を返す (操作ウィンドウの負荷の表示に出る)。
    socket.send(
        opcode::TEXT,
        br#"{"kind":"renderStats","fps":59.5,"renderMs":1.25,"renderMaxMs":3.0,"dropped":0}"#,
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !lock(&studio.renders).contains_key(source::LABEL) {
        assert!(std::time::Instant::now() < deadline, "the stats arrive");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(lock(&studio.renders)[source::LABEL].stats.fps, 59.5);
    // 読めなかったことも返す (今のシーンのものだけ。操作ウィンドウに出る。T0825)。
    let revision = next["revision"].as_u64().unwrap();
    socket.send(
        opcode::TEXT,
        format!(
            r#"{{"kind":"loadFailure","revision":{revision},"asset":"model","code":"unreadable"}}"#
        )
        .as_bytes(),
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while studio.load_failures_view().is_empty() {
        assert!(std::time::Instant::now() < deadline, "the failure arrives");
        std::thread::sleep(Duration::from_millis(10));
    }
    let failures = studio.load_failures_view();
    assert_eq!(failures[0].label, source::LABEL);
    assert_eq!(failures[0].failure.code, LoadFailureCode::Unreadable);
    // 2 つ目のページは別のラベル。
    let mut second = Socket::open_at(port, &format!("/source/{token}/frames"), &origin);
    assert_eq!(second.read_json()["kind"], "scene");
    assert_eq!(studio.api_view().source.connections, 2);

    // 読むだけ: バイナリは受けずに閉じる。
    second.send(opcode::BINARY, b"\x01");
    let (op, payload) = second.read();
    assert_eq!(
        (op, &payload[..2]),
        (
            opcode::CLOSE,
            &ws::close::UNSUPPORTED_DATA.to_be_bytes()[..]
        )
    );

    // 切ると閉じ、負荷の表示からも外す。
    studio.set_source_enabled(false);
    let (op, payload) = loop {
        let (op, payload) = socket.read();
        if op != opcode::BINARY {
            break (op, payload);
        }
    };
    assert_eq!(
        (op, &payload[..2]),
        (opcode::CLOSE, &source::CLOSE_GOING_AWAY.to_be_bytes()[..])
    );
    assert!(!lock(&studio.renders).contains_key(source::LABEL));
    assert!(studio.load_failures_view().is_empty());
    assert_eq!(studio.api_view().source.connections, 0);
}

/// 受け手が読めなかったことは、今のシーンのものだけを受け、次のシーンに移ったら捨てる (T0825)。
#[test]
fn load_failures_are_kept_for_the_current_scene_only() {
    let dir = Dir::new();
    let model = dir.model();
    let studio = studio();
    let (_, game) = stage_scenes(&studio, &model, &[]);
    let revision = lock(&studio.conductor).scene().unwrap().revision;
    let failure = |revision, asset, index, code| LoadFailure {
        revision,
        asset,
        index,
        code,
    };
    // 前のシーンのもの (受け手が読んでいる間に次のシーンになった) は知らせない。
    studio.report_load_failure(
        "output",
        failure(
            revision - 1,
            Asset::Model,
            None,
            LoadFailureCode::Unavailable,
        ),
    );
    // シーンに無いモジュールの番号・モデルに番号を付けたものは捨てる。
    studio.report_load_failure(
        "output",
        failure(
            revision,
            Asset::Module,
            Some(0),
            LoadFailureCode::Unreadable,
        ),
    );
    studio.report_load_failure(
        "output",
        failure(revision, Asset::Model, Some(0), LoadFailureCode::Unreadable),
    );
    assert!(studio.load_failures_view().is_empty());

    studio.report_load_failure(
        "output",
        failure(revision, Asset::Model, None, LoadFailureCode::Unreadable),
    );
    studio.report_load_failure(
        "browserSource",
        failure(
            revision,
            Asset::Background,
            None,
            LoadFailureCode::Unavailable,
        ),
    );
    let failures = studio.load_failures_view();
    assert_eq!(
        failures
            .iter()
            .map(|f| (f.label.as_str(), f.failure.asset, f.failure.code))
            .collect::<Vec<_>>(),
        [
            (
                "browserSource",
                Asset::Background,
                LoadFailureCode::Unavailable
            ),
            ("output", Asset::Model, LoadFailureCode::Unreadable),
        ]
    );
    // 受け手が去れば、その報告を捨てる。
    studio.forget_load_failure("browserSource");
    assert_eq!(studio.load_failures_view().len(), 1);

    // シーンを切り替えると、前のシーンの報告はすべて捨てる。遅れて届いた前のシーンの報告も受けない。
    studio.with_stage(|stage, conductor| stage.activate(&game, conductor).unwrap());
    assert!(studio.load_failures_view().is_empty());
    studio.report_load_failure(
        "output",
        failure(revision, Asset::Model, None, LoadFailureCode::Unreadable),
    );
    assert!(studio.load_failures_view().is_empty());
}

/// 1 つの受け手で読めなかったものは、1 つでなくすべて覚えて出す。同じものの報告は置き換え、並びはモデル・モジュール
/// (番号の順)・背景の画像 (T0840)。
#[test]
fn every_load_failure_of_a_receiver_is_kept() {
    use crate::virtual_camera::{Event, LABEL};
    let dir = Dir::new();
    let model = dir.model();
    let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
    let dress = dir.module("dress.ugoemod", fixtures::dress_module());
    let studio = studio();
    let (chat, game) = stage_scenes(&studio, &model, &[hair, dress]);
    studio.with_stage(|stage, conductor| stage.activate(&game, conductor).unwrap());
    let revision = lock(&studio.conductor).scene().unwrap().revision;
    let failure = |asset, index, code| LoadFailure {
        revision,
        asset,
        index,
        code,
    };
    let view = |studio: &Studio| {
        studio
            .load_failures_view()
            .into_iter()
            .map(|f| (f.label, f.failure.asset, f.failure.index, f.failure.code))
            .collect::<Vec<_>>()
    };
    let output = || "output".to_owned();
    let camera = || LABEL.to_owned();

    // 出力ウィンドウは 1 つずつ返す。前に返したものは残る。
    studio.report_load_failure(
        "output",
        failure(Asset::Background, None, LoadFailureCode::Unreadable),
    );
    studio.report_load_failure(
        "output",
        failure(Asset::Module, Some(1), LoadFailureCode::Unreadable),
    );
    studio.report_load_failure(
        "output",
        failure(Asset::Module, Some(0), LoadFailureCode::Unavailable),
    );
    // 同じものは置き換える (同じ報告が重ねて届いても 1 つ)。
    studio.report_load_failure(
        "output",
        failure(Asset::Module, Some(1), LoadFailureCode::Unavailable),
    );
    studio.report_load_failure(
        "output",
        failure(Asset::Module, Some(1), LoadFailureCode::Unavailable),
    );
    // 仮想カメラは 1 つのシーンの分をまとめて返す。捨てるもの (シーンに無い番号) が混じっても、残りは受ける。
    studio.camera_event(
        Event::LoadFailures(vec![
            failure(Asset::Module, Some(0), LoadFailureCode::Unreadable),
            failure(Asset::Module, Some(2), LoadFailureCode::Unreadable),
            failure(Asset::Background, None, LoadFailureCode::Unavailable),
        ]),
        30,
    );
    assert_eq!(
        view(&studio),
        [
            (
                output(),
                Asset::Module,
                Some(0),
                LoadFailureCode::Unavailable
            ),
            (
                output(),
                Asset::Module,
                Some(1),
                LoadFailureCode::Unavailable
            ),
            (
                output(),
                Asset::Background,
                None,
                LoadFailureCode::Unreadable
            ),
            (
                camera(),
                Asset::Module,
                Some(0),
                LoadFailureCode::Unreadable
            ),
            (
                camera(),
                Asset::Background,
                None,
                LoadFailureCode::Unavailable
            ),
        ]
    );
    // 受け手が去れば、その報告をすべて捨てる。
    studio.forget_load_failure(LABEL);
    assert_eq!(view(&studio).len(), 3);
    // シーンを切り替えると、すべて捨てる。
    studio.with_stage(|stage, conductor| stage.activate(&chat, conductor).unwrap());
    assert!(view(&studio).is_empty());
}

/// 仮想カメラの受け手が読めなかったことも、同じ報告として操作ウィンドウに出す。止まったら捨てる (T0836)。
#[test]
fn virtual_camera_load_failures_are_reported_like_the_output_window() {
    use crate::virtual_camera::{Event, LABEL, Settings, Status};
    let dir = Dir::new();
    let model = dir.model();
    let studio = studio();
    stage_scenes(&studio, &model, &[]);
    let revision = lock(&studio.conductor).scene().unwrap().revision;
    let failure = LoadFailure {
        revision,
        asset: Asset::Module,
        index: None,
        code: LoadFailureCode::Unreadable,
    };
    // シーンに無いモジュールの番号は、出力ウィンドウと同じく捨てる。
    studio.camera_event(Event::LoadFailures(vec![failure]), 30);
    assert!(studio.load_failures_view().is_empty());
    let failure = LoadFailure {
        asset: Asset::Model,
        ..failure
    };
    studio.camera_event(Event::LoadFailures(vec![failure]), 30);
    let failures = studio.load_failures_view();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].label, LABEL);
    assert_eq!(failures[0].failure, failure);
    // 動いている間の状態の知らせでは捨てない。止まったら (失敗も) 捨てる。
    let settings = Settings::default();
    studio.camera_event(
        Event::Status(Status::Running {
            settings,
            device: "/dev/video10".into(),
            consumer: true,
            late: false,
        }),
        30,
    );
    assert_eq!(studio.load_failures_view().len(), 1);
    studio.camera_event(
        Event::Status(Status::Failed {
            settings,
            code: "gpuError".into(),
            detail: String::new(),
        }),
        30,
    );
    assert!(studio.load_failures_view().is_empty());
}

/// 仮想カメラに渡すモジュールは、シーンの番号のまま並べる (受け取れないものは `None`。T0836)。
#[test]
fn the_camera_scene_keeps_module_indices() {
    let dir = Dir::new();
    let model = dir.model();
    let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
    let studio = studio();
    let (_, game) = stage_scenes(&studio, &model, &[hair]);
    studio.with_stage(|stage, conductor| stage.activate(&game, conductor).unwrap());
    let stage = lock(&studio.stage);
    let conductor = lock(&studio.conductor);
    let files = Studio::camera_scene(&stage, &conductor).unwrap();
    assert_eq!(files.modules.len(), 1);
    assert_eq!(files.scene.modules.len(), 1);
    assert!(files.modules.iter().all(Option::is_some));
    assert!(files.model.is_some());
}

#[test]
fn a_page_that_stops_reading_is_dropped() {
    let (studio, _port, _token) = source_studio();
    let (tx, rx) = mpsc::sync_channel(source::QUEUE);
    let id = studio.source_connect(tx).unwrap();
    assert_eq!(studio.api_view().source.connections, 1);
    for _ in 0..source::QUEUE {
        studio.source_frame(vec![1]);
    }
    assert_eq!(studio.api_view().source.connections, 1);
    studio.source_frame(vec![1]);
    assert_eq!(studio.api_view().source.connections, 0, "dropped when full");
    // 溜まった分は送れる。
    assert_eq!(rx.try_iter().count(), source::QUEUE);
    let _ = id;
    studio.set_source_enabled(false);
}
