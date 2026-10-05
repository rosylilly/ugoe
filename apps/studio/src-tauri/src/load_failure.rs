//! 受け手 (出力ウィンドウ・ブラウザソースのページ) が今のシーンのファイルを読めなかったことの報告
//! (T0825、`docs/studio.md#受け手が読めなかったとき`)。
//!
//! 受け手は、モデル・モジュール・背景の画像を取れない・読めないと、シーンの版と何を読めなかったかと理由のコードを返す
//! (出力ウィンドウは `report_load_failure`、ブラウザソースは WebSocket の `{"kind":"loadFailure",…}`)。
//! バックエンドは今のシーンの報告だけを受け、受け手ごとに読めなかったものをすべて覚えて (同じものは後の報告で置き換える。T0840)
//! 操作ウィンドウに配る (`Monitor::LoadFailures`)。次のシーンに移ったら、前のシーンの報告はすべて捨てる
//! (次のシーンに移ったための失敗は知らせない)。

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

impl LoadFailure {
    /// 何を読めなかったか (1 つの受け手で同じものの報告は 1 つだけ覚える)。モデル・モジュール (番号の順)・背景の画像の順に並ぶ。
    fn key(&self) -> (Asset, Option<usize>) {
        (self.asset, self.index)
    }
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

/// 受け手ごとの、今のシーンで読めなかったこと (ラベル → 報告。モデル・モジュール (番号の順)・背景の画像の順で、同じものは 1 つ。空の並びは置かない)。
pub type LoadFailures = BTreeMap<String, Vec<LoadFailure>>;

impl Studio {
    /// 受け手 `label` が読めなかったことを受ける。今のシーンの報告でなければ (次のシーンに移った) 捨てる。
    pub(crate) fn report_load_failure(&self, label: &str, failure: LoadFailure) {
        self.report_load_failures(label, std::slice::from_ref(&failure));
    }

    /// 受け手 `label` が読めなかったことをまとめて受ける (仮想カメラは 1 つのシーンの分をまとめて返す)。今のシーンの報告で
    /// なければ捨てる。前に受けたものは残し、同じもの (モデル・同じ番号のモジュール・背景の画像) は置き換える。
    pub(crate) fn report_load_failures(&self, label: &str, reported: &[LoadFailure]) {
        {
            let conductor = lock(&self.conductor);
            let Some(scene) = conductor.scene() else {
                return;
            };
            let valid = |failure: &LoadFailure| {
                scene.revision == failure.revision
                    && match failure.asset {
                        Asset::Module => failure.index.is_some_and(|i| i < scene.modules.len()),
                        Asset::Model | Asset::Background => failure.index.is_none(),
                    }
            };
            // 指揮役を握ったまま入れる (シーンの切り替えで捨てるのと入れ違わないように)。覚えているものはすべて今のシーンの版なので、
            // 1 つの受け手の並びは高々 2 + モジュールの数。
            let mut failures = lock(&self.load_failures);
            let mut changed = false;
            for failure in reported.iter().filter(|f| valid(f)) {
                let list = failures.entry(label.to_owned()).or_default();
                match list.binary_search_by_key(&failure.key(), LoadFailure::key) {
                    Ok(i) if list[i] == *failure => {}
                    Ok(i) => {
                        list[i] = *failure;
                        changed = true;
                    }
                    Err(i) => {
                        list.insert(i, *failure);
                        changed = true;
                    }
                }
            }
            if !changed {
                return;
            }
        }
        self.send_load_failures();
    }

    /// 受け手 `label` が去った (ブラウザソースのページを閉じた・仮想カメラが止まった)。その報告をすべて捨てる。
    pub(crate) fn forget_load_failure(&self, label: &str) {
        if lock(&self.load_failures).remove(label).is_some() {
            self.send_load_failures();
        }
    }

    /// シーンが `revision` に変わった: 前のシーンの報告を捨てる。指揮役を握ったまま呼ぶ。
    pub(crate) fn scene_changed_load_failures(&self, revision: Option<u32>) {
        let changed = {
            let mut failures = lock(&self.load_failures);
            let mut changed = false;
            failures.retain(|_, list| {
                let before = list.len();
                list.retain(|f| Some(f.revision) == revision);
                changed |= list.len() != before;
                !list.is_empty()
            });
            changed
        };
        if changed {
            self.send_load_failures();
        }
    }

    /// 操作ウィンドウに出す、受け手ごとの読めなかったこと (ラベルの順。1 つの受け手ではモデル・モジュール (番号の順)・背景の画像の順)。
    pub(crate) fn load_failures_view(&self) -> Vec<ReceiverFailure> {
        lock(&self.load_failures)
            .iter()
            .flat_map(|(label, list)| {
                list.iter().map(|failure| ReceiverFailure {
                    label: label.clone(),
                    failure: *failure,
                })
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
