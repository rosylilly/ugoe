---
id: T0165
title: ParameterValuesError の LipSyncNaN・PartOpacityNaN を有限でない値の名前にする (判断が要る)
status: todo
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, docs]
depends_on: []
assignee:
---

# T0165: ParameterValuesError の LipSyncNaN・PartOpacityNaN を有限でない値の名前にする (判断が要る)

## 目的

`crates/ugoe/src/follow.rs` の `ParameterValuesError::LipSyncNaN`・`PartOpacityNaN` は、NaN だけでなく無限大も拒む
(docs/portability.md#ホストから来る数)。名前が中身と合っていない (T0139 で見送り、T0010 の候補)。

**判断が要る:** 公開 API の変更 (バリアントの改名)。1.0 の前に公開 API をまとめて直す時期に合わせるか、`#[deprecated]` の別名を残すかを決める。

## 受け入れ条件

- [ ] 改名の方針 (時期・別名の有無) を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] 改名し、エラーの文・`code` (C ABI・wasm に出る値が変わるなら、その変更) と docs/portability.md を合わせた
- [ ] 無限大でも同じバリアントが返るテストがある
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0010 のメモ (T0010 の結果の起票の候補)、T0139
- crates/ugoe/src/follow.rs、docs/portability.md#ホストから来る数

## メモ

- T0026 で起票した (元の候補: T0010 の改名)。
