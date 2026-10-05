//! グローバルなホットキーの割り当て (`docs/studio.md#ホットキー`、ADR 0028 の 9.)。
//!
//! 割り当てはプロファイルに持つ ([`Hotkey`] の列)。キーは `tauri-plugin-global-shortcut` の書き方
//! (`Ctrl+Shift+KeyA`・`Alt+F1` など。修飾キーは `Ctrl`・`Alt`・`Shift`・`Super`、キーは `KeyboardEvent.code` の名前) で持つ。
//! ここは割り当ての形と、登録してよいかの判断 (キーを読めるか・ほかの割り当てとぶつかるか) だけを持ち、
//! OS への登録と操作の実行は `lib.rs` が行う (Tauri に依存しない)。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri_plugin_global_shortcut::Shortcut;

/// 割り当ての数の上限 (プロファイルを手で書き換えたとき)。
pub const MAX_HOTKEYS: usize = 200;

/// 1 つの割り当て。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotkey {
    /// キー (`Ctrl+Shift+KeyA` など)。空ならまだ決めていない (登録しない)。
    #[serde(default)]
    pub keys: String,
    pub action: HotkeyAction,
}

/// 表情の操作の仕方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExpressionMode {
    /// その表情だけにする (ほかの表情は外す)。
    #[default]
    Set,
    /// 付けていれば外し、外していれば重ねる。
    Toggle,
}

/// モーションの優先度 (`docs/spec/motion.md#モーションマネージャ`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MotionPriority {
    Idle,
    #[default]
    Normal,
    Force,
}

impl From<MotionPriority> for ugoe::motion::Priority {
    fn from(p: MotionPriority) -> Self {
        match p {
            MotionPriority::Idle => Self::Idle,
            MotionPriority::Normal => Self::Normal,
            MotionPriority::Force => Self::Force,
        }
    }
}

impl From<ugoe::motion::Priority> for MotionPriority {
    fn from(p: ugoe::motion::Priority) -> Self {
        match p {
            ugoe::motion::Priority::Idle => Self::Idle,
            ugoe::motion::Priority::Normal => Self::Normal,
            ugoe::motion::Priority::Force => Self::Force,
        }
    }
}

/// ホットキーで行う操作。`kind` で分ける。新しい版のスタジオが足した操作は [`HotkeyAction::Unknown`] として読み飛ばす
/// (保存し直すと消える。`docs/studio.md#プロファイルの形式` の「足すときの規則」)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum HotkeyAction {
    /// シーンに切り替える。
    #[serde(rename_all = "camelCase")]
    Scene { scene: String },
    /// 表情 (名前) を付ける・付け外す。
    #[serde(rename_all = "camelCase")]
    Expression {
        expression: String,
        #[serde(default)]
        mode: ExpressionMode,
    },
    /// 表情をすべて外す。
    ClearExpressions,
    /// モーション (名前) を優先度付きで再生する。
    #[serde(rename_all = "camelCase")]
    Motion {
        motion: String,
        #[serde(default)]
        priority: MotionPriority,
    },
    /// 再生中のモーションを止める (待機モーションに戻る)。
    StopMotions,
    /// 出しているシーンの着せ替え: スロット `slot` のモジュールを `module` にする (`None` は外す)。
    #[serde(rename_all = "camelCase")]
    DressUp {
        slot: String,
        #[serde(default)]
        module: Option<PathBuf>,
    },
    /// 出しているシーンのポーズで、要素 `part` (パーツの ID) をそのグループの表示する要素にする (T0842)。
    #[serde(rename_all = "camelCase")]
    Pose { part: String },
    /// トラッキングを一時停止する・再開する (押すたびに切り替える)。
    TrackingPause,
    /// トラッキングのキャリブレーション (今の顔を中立にする)。
    TrackingCalibrate,
    /// 口パクを一時停止する・再開する (押すたびに切り替える)。
    LipSyncPause,
    /// 知らない操作 (新しい版のプロファイル)。何もしない。
    #[serde(other)]
    Unknown,
}

/// 割り当てを登録できない理由。画面は `code` で文言を選ぶ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum HotkeyProblem {
    /// キーをまだ決めていない。
    #[serde(rename = "unassigned")]
    Unassigned,
    /// キーを読めない (修飾キーだけ・知らないキーの名前)。
    #[serde(rename = "invalidKeys")]
    InvalidKeys,
    /// 前の割り当てと同じキー (`other` はその割り当ての位置)。後の方は登録しない。
    #[serde(rename = "duplicate")]
    Duplicate { other: usize },
    /// OS に登録できなかった (ほかのアプリや OS が使っているなど)。
    #[serde(rename = "registerFailed")]
    RegisterFailed { message: String },
    /// 知らない操作。
    #[serde(rename = "unknownAction")]
    UnknownAction,
}

/// キーを読む。修飾キーの無いキー (`KeyA` だけ) も読めるが、文字の入力を奪うので画面では修飾キーを勧める。
pub fn parse_keys(keys: &str) -> Option<Shortcut> {
    let keys = keys.trim();
    if keys.is_empty() {
        return None;
    }
    keys.parse::<Shortcut>().ok()
}

/// 登録の計画: 登録するもの (割り当ての位置とキー) と、登録しないものの理由 (位置ごと)。
#[derive(Debug, Default)]
pub struct Plan {
    pub register: Vec<(usize, Shortcut)>,
    pub problems: Vec<(usize, HotkeyProblem)>,
}

/// 割り当ての列から、登録するものを選ぶ。同じキーは前のものを登録し、後のものはぶつかりとして知らせる。
pub fn plan(hotkeys: &[Hotkey]) -> Plan {
    let mut plan = Plan::default();
    for (index, hotkey) in hotkeys.iter().enumerate() {
        if hotkey.action == HotkeyAction::Unknown {
            plan.problems.push((index, HotkeyProblem::UnknownAction));
            continue;
        }
        if hotkey.keys.trim().is_empty() {
            plan.problems.push((index, HotkeyProblem::Unassigned));
            continue;
        }
        let Some(shortcut) = parse_keys(&hotkey.keys) else {
            plan.problems.push((index, HotkeyProblem::InvalidKeys));
            continue;
        };
        if let Some((other, _)) = plan.register.iter().find(|(_, s)| s.id() == shortcut.id()) {
            plan.problems
                .push((index, HotkeyProblem::Duplicate { other: *other }));
            continue;
        }
        plan.register.push((index, shortcut));
    }
    plan
}

/// 割り当ての列を収め直す (数の上限・キーの前後の空白)。
pub fn sanitize(hotkeys: &mut Vec<Hotkey>) {
    hotkeys.truncate(MAX_HOTKEYS);
    for hotkey in hotkeys {
        let trimmed = hotkey.keys.trim();
        if trimmed.len() != hotkey.keys.len() {
            hotkey.keys = trimmed.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hotkey(keys: &str, action: HotkeyAction) -> Hotkey {
        Hotkey {
            keys: keys.into(),
            action,
        }
    }

    #[test]
    fn actions_are_tagged_and_unknown_ones_are_kept_as_unknown() {
        let list: Vec<Hotkey> = serde_json::from_str(
            r#"[
              { "keys": "Ctrl+Shift+Digit1", "action": { "kind": "scene", "scene": "scene-1" } },
              { "keys": "Ctrl+KeyE", "action": { "kind": "expression", "expression": "Smile", "mode": "toggle" } },
              { "keys": "Ctrl+KeyM", "action": { "kind": "motion", "motion": "Wave" } },
              { "keys": "Ctrl+KeyH", "action": { "kind": "dressUp", "slot": "Hair", "module": "/m/long.ugoemod" } },
              { "keys": "Ctrl+KeyP", "action": { "kind": "trackingPause" } },
              { "keys": "Ctrl+KeyX", "action": { "kind": "somethingNew", "value": 1 } },
              { "keys": "Ctrl+KeyW", "action": { "kind": "pose", "part": "Wave" } }
            ]"#,
        )
        .unwrap();
        assert_eq!(
            list[1].action,
            HotkeyAction::Expression {
                expression: "Smile".into(),
                mode: ExpressionMode::Toggle
            }
        );
        assert_eq!(
            list[2].action,
            HotkeyAction::Motion {
                motion: "Wave".into(),
                priority: MotionPriority::Normal
            }
        );
        assert_eq!(list[5].action, HotkeyAction::Unknown);
        let json = serde_json::to_value(&list[3]).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "keys": "Ctrl+KeyH",
                "action": { "kind": "dressUp", "slot": "Hair", "module": "/m/long.ugoemod" }
            })
        );
        assert_eq!(
            serde_json::to_value(&list[4].action).unwrap(),
            serde_json::json!({ "kind": "trackingPause" })
        );
        assert_eq!(
            list[6].action,
            HotkeyAction::Pose {
                part: "Wave".into()
            }
        );
    }

    #[test]
    fn keys_use_the_plugin_syntax() {
        assert!(parse_keys("Ctrl+Shift+KeyA").is_some());
        assert!(parse_keys("Alt+F1").is_some());
        assert!(parse_keys("Super+Digit3").is_some());
        assert!(parse_keys("Ctrl+Numpad1").is_some());
        assert!(parse_keys("").is_none());
        assert!(parse_keys("Ctrl+Shift").is_none());
        assert!(parse_keys("Ctrl+NoSuchKey").is_none());
        // 書き方が違っても同じキーなら同じ ID (ぶつかりを見分ける)。
        assert_eq!(
            parse_keys("ctrl+shift+keya").unwrap().id(),
            parse_keys("Shift+Ctrl+A").unwrap().id()
        );
    }

    #[test]
    fn the_plan_registers_the_first_of_each_key() {
        let list = vec![
            hotkey(
                "Ctrl+KeyA",
                HotkeyAction::Scene {
                    scene: "scene-1".into(),
                },
            ),
            hotkey("", HotkeyAction::ClearExpressions),
            hotkey("Ctrl+Nope", HotkeyAction::StopMotions),
            hotkey("ctrl+a", HotkeyAction::TrackingCalibrate),
            hotkey("Ctrl+KeyB", HotkeyAction::Unknown),
            hotkey("Ctrl+KeyB", HotkeyAction::LipSyncPause),
        ];
        let plan = plan(&list);
        let registered: Vec<usize> = plan.register.iter().map(|(i, _)| *i).collect();
        assert_eq!(registered, [0, 5]);
        assert_eq!(
            plan.problems,
            [
                (1, HotkeyProblem::Unassigned),
                (2, HotkeyProblem::InvalidKeys),
                (3, HotkeyProblem::Duplicate { other: 0 }),
                (4, HotkeyProblem::UnknownAction),
            ]
        );
        assert_eq!(
            serde_json::to_value(HotkeyProblem::Duplicate { other: 0 }).unwrap(),
            serde_json::json!({ "code": "duplicate", "other": 0 })
        );
    }

    #[test]
    fn sanitizing_trims_and_caps() {
        let mut list = vec![hotkey(" Ctrl+KeyA ", HotkeyAction::StopMotions); MAX_HOTKEYS + 5];
        sanitize(&mut list);
        assert_eq!(list.len(), MAX_HOTKEYS);
        assert_eq!(list[0].keys, "Ctrl+KeyA");
    }
}
