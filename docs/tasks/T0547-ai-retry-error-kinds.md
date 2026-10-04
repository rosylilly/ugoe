---
id: T0547
title: AI: リグの値の問い直しで、AI の値の誤りとテンプレートの失敗を分ける
status: done
milestone: M9
crates: [ugoe-ai, ugoe-mcp, docs]
depends_on: [T0528]
assignee: claude-worker-T0547
---

# T0547: AI: リグの値の問い直しで、AI の値の誤りとテンプレートの失敗を分ける

## 目的

T0528 のレビューで見つけた。`propose_rig_with_ai`・`propose_module_rig_with_ai` (共通の `rig::ask_until_valid`) は、組むときの
`AiError::InvalidProposal` をすべて「AI の値が悪い」とみなして問い直す。ところがテンプレートの内部の失敗 (座標変換が解けない・
軸のキーがないなど、`rig::template` が作るエラー) も同じ `InvalidProposal` なので、値を変えても直らない失敗で上限まで問い合わせ
(費用がかかる)、内部のエラー文を AI に送る。逆に、モジュールのリグで AI が範囲の中の `sway_scale = 0` を答え、ほかに組むものが
ないと `InvalidRequest` ("nothing to rig") になり、問い直さずに失敗する。`max_attempts = 0` は問い合わせずに
`InvalidProposal` ("after 0 attempts") を返す。

## 受け入れ条件

- [x] テンプレートの内部の失敗を、問い直さないエラー (新しい種類か `InvalidRequest`) で返す。AI の値が原因の失敗 (範囲の外・
      `validate` を通らない・組むものがなくなる値) だけを問い直す
- [x] `max_attempts = 0` の扱いを決める (`InvalidRequest` にする、など)
- [x] それぞれのテスト (`FakeProvider`)。エラーの種類を見ている呼び出し側 (エディタ・MCP・CLI) を確かめる
- [x] `docs/ai-integration.md` の「AI が決める値」の問い直しの説明を直す
- [x] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/rig.rs` (`ask_until_valid`・`template`)、`crates/ugoe-ai/src/module_rig.rs` (`build`)
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ

### やったこと

- `rig::template` (テンプレートの内部の失敗) を `AiError::InvalidRequest` ("the template cannot rig this model: …") にした。
  `ask_until_valid` は `InvalidRequest` を問い直さずに返すので、値を変えても直らない失敗で上限まで問い合わせず、内部のエラー文も
  AI に送らない。新しいエラーの種類は足さなかった (足すとエディタの `error.rs` の網羅の照合と翻訳の文言が要り、範囲の外になる。
  `Layout::new` の「`face` がない」も既に `InvalidRequest` で、入力が原因で組めないものとしてそろう)。`AiError` の
  `InvalidProposal`・`InvalidRequest` の doc を「AI の値が原因か」で書き分けた。
- 問い直すのは AI の値が原因のもの: スキーマに合わない・範囲の外 (`check`)・適用や `validate` を通らない (`Builder::apply`・
  `validate`)・形が有限でない・モジュールとして分けられない、と、モジュールで揺れるラベルの部品があるのに `sway_scale = 0` で
  組むものがなくなった場合 (`module_rig::build` で `InvalidProposal` "sway_scale is 0 … choose a sway_scale above 0")。
  揺れるものがなく組むものがない場合は今までどおり `InvalidRequest` ("nothing to rig")。
- `max_attempts = 0` は問い合わせずに `InvalidRequest` ("max_attempts is 0 …")。
- テスト: `rig.rs` の単体テスト (`FakeProvider` で `ask_until_valid` を直接: 値の誤りは問い直す・テンプレートの失敗は 1 回で止まる・
  上限 0)、`tests/module_rig.rs` (`sway_scale = 0` の答えを問い直して 2 回目を使う・上限 0)。`nothing_to_rig_is_reported` の
  `sway_scale = 0` の期待を `InvalidProposal` に直した。
- 呼び出し側の確認: エディタ (`apps/editor/src-tauri/src/proposals.rs`) はエラーをそのまま返し、`error.rs` で種類ごとの文言にするだけ。
  MCP (`ugoe-mcp/src/autorig.rs` の `ai_error`) は `InvalidProposal`・`InvalidRequest` のどちらも前置きを外して文だけを返すので、
  変更は要らない。CLI はリグの AI の経路を呼んでいない。
- `docs/ai-integration.md` の「AI が決める値」の手順 3 と `sway_scale` の行を直した。英語の公開仕様 (`spec/`) に対応する節はない。

### やらなかったこと

- エディタで AI を使わない経路 (`propose_rig`) のテンプレートの失敗は、文言のコードが `ai.invalid_proposal` から `ai.invalid_request`
  (「リクエストを送れません (…)」) に変わる。文言が実情に少し合わないが、エディタは範囲の外なので変えていない。

### 起票の候補

- テンプレートで組めない失敗の専用の種類 (`AiError::Template` など) と、エディタの文言 (「このモデルはテンプレートで組めません」)。
  エディタの `error.rs`・翻訳を同時に直す。
