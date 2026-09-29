---
id: T0503
title: レイヤーの意味分類
status: done
milestone: M6
crates: [ugoe-ai, docs]
depends_on: [T0501]
assignee: claude-worker-T0503
---

# T0503: レイヤーの意味分類

## 目的

ラベルの語彙を定め、`Provider` を使ってレイヤーを分類する。`FakeProvider` でテストする。

## 受け入れ条件

- [x] ラベルの語彙が定数として定義されている
- [x] 出力の JSON Schema がある
- [x] 録画した応答を使うテストがある
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md`
- `assets/fixtures/psd-real/` (See-Through の PSD。英語のレイヤー名で、正解のラベルを付けやすい)

## メモ

- やったこと
  - 語彙 `ugoe_ai::classify::{Label, Side, LabelCategory, LayerLabel}`。ラベルは 40 個 (着手時に「38 個」と伝えた一覧は数え違いで、一覧そのものは同じ)。左右はキャラクター自身の左右。`Label::ALL`・`as_str`・`FromStr`・`category`・`is_paired`
  - 名前の規則による分類 `classify_by_name` (API を使わない、決定的)。英語のキーワード (区切り文字・camelCase・数字で語に分け、連続する 1〜3 語をつないで比べる) と日本語の部分文字列。グループの名前からの継承、位置による左右 (顔の中心と比べる)。See-Through の 2 つの PSD の全レイヤーで正解と一致する
  - AI による分類 `classify_layers` と出力の JSON Schema `output_schema` (`{ layers: [{ index, label, side, confidence }] }`、`additionalProperties: false`)。答えのないレイヤーは名前の規則で補い、範囲外・重複の番号は `InvalidResponse`。`confidence` は受け取ってから 0..=1 に丸める (構造化出力は数値の制約を書けない)
  - 送る画像 (`ClassifyOptions`): 合成画像は長い辺 1024 画素、サムネイルは 256 画素まで縮め 64 画素未満は整数倍に拡大、透明部分は明るい灰色。1 リクエスト 32 レイヤー (画像 33 枚) まで、超えれば分ける。送る前の確認用に `classification_requests`
  - 録画 `tests/cassettes/classify_layers_bustup.json` (Messages API の応答の形に合わせて手で作った。リクエストの本文は書かず、テストで送った本文を確かめる)
  - 範囲外の変更: `docs/ai-integration.md#レイヤーの意味分類` に語彙と 2 つの経路を書いた (`crates:` に `docs` を追加)
- やらなかったこと・後続の候補
  - 実際の API での精度の確認と録り直し (キーがない)。See-Through 以外の名前 (日本語の PSD・「レイヤー 1」だらけの PSD) での精度の評価
  - ラベルからメッシュのプリセット・標準パラメータ・テンプレートへの対応は T0504 などで行う
  - エディタでの確認の UI (低い `confidence` の表示) と、分類結果をプロジェクトに保存する形は未定
  - 合成画像はブレンドモード・クリッピングを無視する (目安の画像なので)
