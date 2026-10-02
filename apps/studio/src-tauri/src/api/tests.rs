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
use crate::lock;

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

/// テスト用のファイルを置く場所 (テストごとに分ける)。
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("ugoe-studio-api-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// 着せ替えのできるモデル (`.ugoe`) に、モーション `Wave` (`BodySway`) と表情 `Smile`・`Angry` (`Secret`) を入れて書く。
    fn model(&self) -> PathBuf {
        let mut bundle = Bundle::new(fixtures::dress_up_base());
        bundle.motions = vec![
            ugoe::format::motion_from_json(
                r#"{ "name": "Wave", "duration": 0.5, "looping": false, "fade_in": 0.0, "fade_out": 0.0,
                     "curves": [{ "target": { "kind": "parameter", "id": "BodySway" },
                                  "segments": [{ "type": "start", "p": { "t": 0.0, "v": 1.0 } },
                                               { "type": "linear", "to": { "t": 0.5, "v": 1.0 } }] }],
                     "events": [{ "t": 0.0, "value": "wave-start" }] }"#,
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
        let path = self.0.join("avatar.ugoe");
        std::fs::write(&path, ugoe::format::write_bundle(&bundle).unwrap()).unwrap();
        path
    }

    fn module(&self, name: &str, module: ugoe::model::module::Module) -> PathBuf {
        let path = self.0.join(name);
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

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
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
    // トークンを示すと承認を待つ (応答は承認まで返らない)。
    client.send(
        "session.hello",
        json!({ "name": "Stream Deck", "token": TOKEN, "permission": "control" }),
    );
    assert!(client.take().is_none());
    let view = studio.api_view();
    assert_eq!(view.pending.len(), 1);
    assert_eq!(view.pending[0].name, "Stream Deck");
    assert_eq!(view.pending[0].permission, Permission::Control);
    assert_eq!(client.fail("scene.list", json!({})), NOT_AUTHENTICATED);
    // 利用者は読むだけで承認する (求めた権限より狭くできる)。
    studio
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
    let dir = Dir::new("operate");
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
    let dir = Dir::new("dressup");
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

#[test]
fn notifies_subscribed_events_only() {
    let dir = Dir::new("events");
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
    format!(
        "GET /api HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
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
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(upgrade(port, "").as_bytes()).unwrap();
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

#[test]
fn turning_the_api_on_saves_the_choice_and_listens() {
    let dir = Dir::new("toggle");
    let path = dir.0.join(SETTINGS_FILE);
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
    "tracking.pause",
    "lipsync.pause",
    "tracking.calibrate",
    "events.subscribe",
    "events.unsubscribe",
];
