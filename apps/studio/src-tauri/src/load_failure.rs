//! 受け手 (出力ウィンドウ・ブラウザソースのページ) が今のシーンのファイルを読めなかったことの報告
//! (T0825、`docs/studio.md#受け手が読めなかったとき`)。
//!
//! 受け手は、モデル・モジュール・背景の画像を取れない・読めないと、シーンの版と何を読めなかったかと理由のコードを返す
//! (出力ウィンドウは `report_load_failure`、ブラウザソースは WebSocket の `{"kind":"loadFailure",…}`)。
//! バックエンドは今のシーンの報告だけを受け、受け手ごとに 1 つ覚えて操作ウィンドウに配る (`Monitor::LoadFailures`)。
//! 次のシーンに移ったら、前のシーンの報告はすべて捨てる (次のシーンに移ったための失敗は知らせない)。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::stage::Asset;
use crate::{Monitor, Studio, lock};

/// 読めなかった理由。画面は `code` で文言を選ぶ (ADR 0029 の 4.)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadFailureCode {
    /// ファイルを受け取れなかった (バックエンドのコマンドや HTTP が失敗した)。
    Unavailable,
    /// 受け取ったが読めなかった (形式が違う・壊れている)。
    Unreadable,
    /// 受け手が描画を始められない (wasm・WebGPU・WebGL2 が使えない)。モデルを読まない。
    NoRenderer,
}

/// 受け手が報告する、読めなかったこと。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadFailure {
    /// シーンの版 (`scene` の `revision`)。今のシーンでなければ捨てる。
    pub revision: u32,
    pub asset: Asset,
    /// モジュールなら、その番号 (`scene` の `modules` の順。`scene_module` の `index`)。
    #[serde(default)]
    pub index: Option<usize>,
    pub code: LoadFailureCode,
}

/// 操作ウィンドウに出す、受け手ごとの読めなかったこと。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiverFailure {
    /// 受け手のラベル (負荷の表示と同じ。`output` は出力ウィンドウ、`browserSource`・`browserSource:2`… はブラウザソース)。
    pub label: String,
    #[serde(flatten)]
    pub failure: LoadFailure,
}

/// 受け手ごとの、今のシーンで読めなかったこと (ラベル → 報告)。
pub type LoadFailures = BTreeMap<String, LoadFailure>;

impl Studio {
    /// 受け手 `label` が読めなかったことを受ける。今のシーンの報告でなければ (次のシーンに移った) 捨てる。
    pub(crate) fn report_load_failure(&self, label: &str, failure: LoadFailure) {
        {
            let conductor = lock(&self.conductor);
            let Some(scene) = conductor.scene() else {
                return;
            };
            let valid = scene.revision == failure.revision
                && match failure.asset {
                    Asset::Module => failure.index.is_some_and(|i| i < scene.modules.len()),
                    Asset::Model | Asset::Background => failure.index.is_none(),
                };
            if !valid {
                return;
            }
            // 指揮役を握ったまま入れる (シーンの切り替えで捨てるのと入れ違わないように)。
            let mut failures = lock(&self.load_failures);
            if failures.get(label) == Some(&failure) {
                return;
            }
            failures.insert(label.to_owned(), failure);
        }
        self.send_load_failures();
    }

    /// 受け手 `label` が去った (ブラウザソースのページを閉じた)。その報告を捨てる。
    pub(crate) fn forget_load_failure(&self, label: &str) {
        if lock(&self.load_failures).remove(label).is_some() {
            self.send_load_failures();
        }
    }

    /// シーンが `revision` に変わった: 前のシーンの報告を捨てる。指揮役を握ったまま呼ぶ。
    pub(crate) fn scene_changed_load_failures(&self, revision: Option<u32>) {
        let changed = {
            let mut failures = lock(&self.load_failures);
            let before = failures.len();
            failures.retain(|_, f| Some(f.revision) == revision);
            failures.len() != before
        };
        if changed {
            self.send_load_failures();
        }
    }

    /// 操作ウィンドウに出す、受け手ごとの読めなかったこと (ラベルの順)。
    pub(crate) fn load_failures_view(&self) -> Vec<ReceiverFailure> {
        lock(&self.load_failures)
            .iter()
            .map(|(label, failure)| ReceiverFailure {
                label: label.clone(),
                failure: *failure,
            })
            .collect()
    }

    pub(crate) fn load_failures_message(&self) -> Option<tauri::ipc::InvokeResponseBody> {
        Self::json(&Monitor::LoadFailures {
            failures: self.load_failures_view(),
        })
    }

    fn send_load_failures(&self) {
        if let Some(body) = self.load_failures_message() {
            self.monitors.send_all(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_are_camel_case_json() {
        let failure: LoadFailure = serde_json::from_str(
            r#"{"revision":7,"asset":"module","index":1,"code":"unreadable"}"#,
        )
        .unwrap();
        assert_eq!(
            failure,
            LoadFailure {
                revision: 7,
                asset: Asset::Module,
                index: Some(1),
                code: LoadFailureCode::Unreadable,
            }
        );
        let model: LoadFailure =
            serde_json::from_str(r#"{"revision":7,"asset":"model","code":"noRenderer"}"#).unwrap();
        assert_eq!(model.index, None);
        assert_eq!(
            serde_json::to_string(&Monitor::LoadFailures {
                failures: vec![ReceiverFailure {
                    label: "output".into(),
                    failure: model,
                }],
            })
            .unwrap(),
            r#"{"kind":"loadFailures","failures":[{"label":"output","revision":7,"asset":"model","index":null,"code":"noRenderer"}]}"#
        );
        assert!(
            serde_json::from_str::<LoadFailure>(r#"{"revision":7,"asset":"model","code":"x"}"#)
                .is_err()
        );
    }
}
