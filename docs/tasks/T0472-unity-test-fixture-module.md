---
id: T0472
title: "Unity: unknown_mask のテストを Unity のテスト専用の小さなモジュールで行う"
status: todo
milestone: M9
crates: [bindings/unity]
depends_on: []
assignee:
---

# T0472: Unity: unknown_mask のテストを Unity のテスト専用の小さなモジュールで行う

## 目的

T0464 の Unity のテスト (`bindings/unity/tests/ResultTests.cs`) は、着せ替えの `unknown_mask` を、互換性テストの本体
(`assets/compat/dev/dress-up-base.ugoe`) に見本のモジュール (`assets/samples` の `long-hair.ugoemod`) を付けて起こしている。
どちらかを作り直してマスクの参照先が変わると落ちる (T0464 の候補)。Unity のテスト専用の小さなモジュールを `bindings/unity/tests/Fixtures/` に置き、
互換性テストの資料と見本に頼らないようにする。

## 受け入れ条件

- [ ] `unknown_mask` のテストが `bindings/unity/tests/Fixtures/` のファイルだけで (または本体も含めてそこに置いて) 失敗を起こす。ファイルの作り方を README か Fixtures の説明に書いた
- [ ] 置いたファイルは小さい (数 KB 程度)。`ugoe` の CLI か Rust のテストで作り直せる
- [ ] `just unity-check` と `just ci` が通る

## 参考

- T0464 のメモ (起票の候補)
- bindings/unity/README.md、assets/compat/README.md

## メモ

- T0036 で起票した (元の候補: T0464)。
