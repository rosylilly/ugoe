---
id: T0165
title: ParameterValuesError の LipSyncNaN・PartOpacityNaN を有限でない値の名前にする (判断が要る)
status: done
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0165
---

# T0165: ParameterValuesError の LipSyncNaN・PartOpacityNaN を有限でない値の名前にする (判断が要る)

## 目的

`crates/ugoe/src/follow.rs` の `ParameterValuesError::LipSyncNaN`・`PartOpacityNaN` は、NaN だけでなく無限大も拒む
(docs/portability.md#ホストから来る数)。名前が中身と合っていない (T0139 で見送り、T0010 の候補)。

**判断が要る:** 公開 API の変更 (バリアントの改名)。1.0 の前に公開 API をまとめて直す時期に合わせるか、`#[deprecated]` の別名を残すかを決める。

## 受け入れ条件

- [x] 改名の方針 (時期・別名の有無) を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] 改名し、エラーの文・`code` (C ABI・wasm に出る値が変わるなら、その変更) と docs/portability.md を合わせた
- [x] 無限大でも同じバリアントが返るテストがある
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0010 のメモ (T0010 の結果の起票の候補)、T0139
- crates/ugoe/src/follow.rs、docs/portability.md#ホストから来る数

## メモ

- T0026 で起票した (元の候補: T0010 の改名)。
- 判断 (2026-10-05、利用者の判断): 今すぐ改名し、別名は残さない (公開前の 0.0.0 なので壊れる利用者はいない)。エラーの文・C ABI と wasm の code・docs を同じ PR で直す。
- やったこと: `LipSyncNaN` → `NonFiniteLipSync`、`PartOpacityNaN` → `NonFinitePartOpacity { index }` に改名した (別名なし)。
  名前はエディタ中核の `NonFiniteDepth`・`NonFiniteKey` に揃えた。テスト (`follow_rejects_non_finite_inputs` は無限大でも同じバリアントを確かめる。既存) の参照も直した。
- エラーの文は T0139 で既に「not finite」になっていたので変えていない。C ABI は `UGOE_ERROR_INVALID_ARGUMENT` とエラーの文、wasm は文だけの例外
  (`code` を持たない) で、どちらもバリアントの名前を出さないため、C ABI・wasm に出る値は変わらない。Unity・Web のバインディングの表にもバリアントは出てこないので変更なし。
- docs/portability.md#下の層の扱い の表にバリアントの名前を書き、上の層ではバリアントを区別しない (名前が外に出ない) ことを書いた。英語の公開仕様 (spec/) はバリアントの名前を書いていないので変更なし。
- `just ci`・`just wasm-check`・`just refs` が通った。
