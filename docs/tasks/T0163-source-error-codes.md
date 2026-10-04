---
id: T0163
title: 検証・合成・推定のエラーにもコードを付け、エディタの自由な文の引数を訳せるようにする
status: todo
milestone: M9
crates: [ugoe-model, ugoe-rigging, ugoe-editor-core, apps/editor, docs]
depends_on: [T0015, T0678]
assignee:
---

# T0163: 検証・合成・推定のエラーにもコードを付け、エディタの自由な文の引数を訳せるようにする

## 目的

エディタのエラーは ADR 0029 のコードと引数で訳すようになった (T0681 でメッセージの一覧も検査する) が、引数のうち自由な文
(`reason`・`detail`・`details`) は英語のまま出る (`docs/editor.md` の「訳すには、その元のエラーにもコードを付ける必要があります (後続)」)。
元のエラー (モデルの検証 `ValidationError`、モジュールの合成 `ComposeError`、キーフォームの推定 `ugoe-rigging` のエラーなど) が
機械の読めるコードと引数を持てば、エディタはそれを訳せる。

**判断が要る:** `ugoe-model` の公開 API の変更 (エラー型にコードを足す)。multi-agent.md に従い、「インターフェース (エラー型のコードと引数の口)」を
先に小さな PR で入れ、エディタの訳を後に入れる。コードの名前の付け方を ADR 0029 と `DressUpError::code`・`LoadWarning::code` に揃える。

## 受け入れ条件

- [ ] 対象のエラー型と、コードの口の形を決めた (メモに表)。`ugoe-model` の分を先の PR に分けた
- [ ] `ValidationError` などにコードと引数を足し、テストで固定した。`docs/spec/model.md` と公開仕様 (`spec/`) を必要なら更新した
- [ ] `ugoe-editor-core` の `message_codes.rs` が元のエラーのコードを引数に載せ、エディタが訳す (翻訳と `docs/editor.md#エラーと警告のコード` を更新)
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0681 のメモ (起票の候補)、docs/editor.md、ADR 0029
- T0015 (検証エラーの列の見せ方)・T0678 (モジュールのエラーのコード)・T0157 (着せ替えのエラーの補足の値)

## メモ

- T0019 で起票した (元の候補: T0681)。エラーの列の見せ方 (T0015) とモジュールのエラー (T0678) が同じ型を触るので、その後に着手する。
