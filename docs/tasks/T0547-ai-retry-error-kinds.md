---
id: T0547
title: AI: リグの値の問い直しで、AI の値の誤りとテンプレートの失敗を分ける
status: todo
milestone: M9
crates: [ugoe-ai, ugoe-mcp, docs]
depends_on: [T0528]
assignee:
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

- [ ] テンプレートの内部の失敗を、問い直さないエラー (新しい種類か `InvalidRequest`) で返す。AI の値が原因の失敗 (範囲の外・
      `validate` を通らない・組むものがなくなる値) だけを問い直す
- [ ] `max_attempts = 0` の扱いを決める (`InvalidRequest` にする、など)
- [ ] それぞれのテスト (`FakeProvider`)。エラーの種類を見ている呼び出し側 (エディタ・MCP・CLI) を確かめる
- [ ] `docs/ai-integration.md` の「AI が決める値」の問い直しの説明を直す
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/rig.rs` (`ask_until_valid`・`template`)、`crates/ugoe-ai/src/module_rig.rs` (`build`)
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ
