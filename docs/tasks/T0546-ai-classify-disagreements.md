---
id: T0546
title: AI: 名前の規則と AI の分類の食い違いを利用者に示す
status: done
milestone: M9
crates: [ugoe-ai, ugoe-mcp, apps/editor, docs]
depends_on: [T0528]
assignee: claude-worker-T0546
---

# T0546: AI: 名前の規則と AI の分類の食い違いを利用者に示す

## 目的

T0528 のレビューで見つけた。`classify_layers_guided` は、モデルが答えたレイヤーのラベルと左右を答えでそのまま置き換え、名前の規則
(`classify_by_name`) の結果は答えのないレイヤーの補いにだけ使う。名前の規則が確からしさ 0.9 で当たっていても (`eyewhite-r` など)、
AI が別のラベルや逆の左右を答えれば黙って上書きされ、確認に回るか (`LayerLabel::needs_review`) は AI の確からしさだけで決まる。
左右の取り違えは目の開閉のパラメータを入れ替えるので、食い違いは利用者に見せたい。

## 受け入れ条件

- [x] 名前の規則が当たった (確からしさが高い) レイヤーで、AI のラベルか左右が違うものを結果から分かるようにする (確からしさを下げる・
      食い違いの印を `LayerLabel` に足す、などから選び、公開 API の変更なら `docs/ai-integration.md` を同じ PR で直す)
- [x] エディタの分類の確認と MCP の `get_psd_layers` で、食い違いを示す (文言は i18n)
- [x] 食い違いのあるとき・ないときのテスト (`FakeProvider`)
- [x] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/classify.rs` (`classify_layers_guided`・`merge_answer`)
- `docs/ai-integration.md#レイヤーの意味分類`
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ

- **やったこと:**
  - `LayerLabel` に `name_disagreement: Option<NameDisagreement>` (`{ label, side }`。食い違う項目にだけ名前の規則の答え。
    serde は省略可・欠けは `None`) を足し、`needs_review` は確からしさが低いか食い違うときに真にした。答えの確からしさは変えない。
  - 公開関数 `classify::mark_name_disagreements(tree, labels)`: 毎回すべてのレイヤーを `classify_by_name` と比べ直す (作り直しで直れば消える)。
    ラベルは名前の規則の確からしさが 0.8 以上 (レイヤー自身の名前で具体的な規則。受け継ぎ 0.63・あいまい 0.5 は除く) のとき。
    左右は名前 (レイヤーかグループ) に左右が書いてあるときと、確からしい左右の対の部位でラベルが同じなのに位置から決めた左右が違うとき。
    `classify_layers_guided` は自分で呼び、`apply_answer` (木を受け取らない) の後は呼び出し側が呼ぶ (MCP の `set_layer_labels`)。
  - MCP: `get_psd_layers` の各レイヤーに `name_disagreement`、`set_layer_labels` の結果に `name_disagreements` (`index`・`path`・`label`・`side`・`by_name`)。ツールの説明も。
  - エディタ: `LayerLabelView.name_disagreement` (`NameDisagreementView`)。分類の確認の一覧の見出しを `toReview` にし、各行に左右と、食い違いには
    「名前では eye_white / キャラクターの右」を出す (日英の翻訳を足し、使わなくなった `lowConfidence` を消した)。提案の理由の末尾の注記にも名前の規則の答えを添える。
  - テスト: `tests/classify.rs` (`FakeProvider` で食い違うとき・同じとき・作り直しと `apply_answer`・長さ違い)、MCP の stdio、エディタの `proposals_tests.rs`。
  - 文書: `docs/ai-integration.md#レイヤーの意味分類`・`docs/editor.md`・`crates/ugoe-ai/README.md`・`crates/ugoe-mcp/README.md`。
- **やらなかったこと:** 食い違いを利用者が「これでよい」と認めて印を消す操作 (エディタには分類を画面で直す機能がまだない)。食い違いの文言はエラーではないので message_codes は足していない。
- **起票の候補:** エディタで分類の結果 (ラベル・左右) を直し、食い違いを認める機能 (分類の確認の一覧から)。
