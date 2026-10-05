---
id: T0472
title: "Unity: unknown_mask のテストを Unity のテスト専用の小さなモジュールで行う"
status: done
milestone: M9
crates: [bindings/unity]
depends_on: []
assignee: claude-worker-T0472
---

# T0472: Unity: unknown_mask のテストを Unity のテスト専用の小さなモジュールで行う

## 目的

T0464 の Unity のテスト (`bindings/unity/tests/ResultTests.cs`) は、着せ替えの `unknown_mask` を、互換性テストの本体
(`assets/compat/dev/dress-up-base.ugoe`) に見本のモジュール (`assets/samples` の `long-hair.ugoemod`) を付けて起こしている。
どちらかを作り直してマスクの参照先が変わると落ちる (T0464 の候補)。Unity のテスト専用の小さなモジュールを `bindings/unity/tests/Fixtures/` に置き、
互換性テストの資料と見本に頼らないようにする。

## 受け入れ条件

- [x] `unknown_mask` のテストが `bindings/unity/tests/Fixtures/` のファイルだけで (または本体も含めてそこに置いて) 失敗を起こす。ファイルの作り方を README か Fixtures の説明に書いた
- [x] 置いたファイルは小さい (数 KB 程度)。`ugoe` の CLI か Rust のテストで作り直せる
- [x] `just unity-check` と `just ci` が通る

## 参考

- T0464 のメモ (起票の候補)
- bindings/unity/README.md、assets/compat/README.md

## メモ

- T0036 で起票した (元の候補: T0464)。
- やったこと (T0472):
  - `bindings/unity/tests/Fixtures/` に、本体 `mask-base.model.json` (手書きのモデル JSON。スロット `Hair` の `mask_sources` は `Face` だけ)、
    モジュール `unknown-mask.module.json` (メッシュ `Back` が本体の `Skin` でマスクする) と、それから作った `unknown-mask.ugoemod` (724 バイト) を置いた。
  - `ResultTests.UnknownMaskFillsTheMaskField` を、本体を `Puppet.FromJson` で、モジュールを `unknown-mask.ugoemod` で読むように変えた
    (`assets/compat`・`assets/samples` に頼らない)。作り方は `tests/Fixtures/README.md` に書いた。
  - **範囲外 (小さな変更):** `crates/ugoe-format/tests/unity_fixtures.rs` (テストだけ) を足した。`UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test unity_fixtures`
    で `.ugoemod` を JSON から作り直し、普段は置いたファイルのモジュールが JSON と同じことと、本体に付けると `ComposeError::UnknownMask`
    (`unknown_mask`・`Back`・`Skin`) になることを確かめる (`just ci` に含まれるので、dotnet の無い CI でも資料のずれに気づける)。
    CLI の `ugoe build` はモジュールの JSON を入力に取れない (プロジェクトのディレクトリかバンドル・`.ugoemod` だけ) ので、Rust のテストで作る形にした。
  - 確かめたこと: `just unity-check` (dotnet test 102 件、UnityCompile の netstandard2.1 と net48、release.mjs のテスト)・`just ci` が通る。
- やらなかったこと: 実際の Unity エディタでの実行は確かめていない (`just unity-check` の範囲まで)。`LimitsTests.cs` の `Fixtures` のパスの組み立ては共通化していない (T0473 と並行のため触らなかった)。
- 起票の候補:
  - `ugoe build` がモデル・モジュールの JSON (`*.model.json`・`*.module.json`) からバンドル・モジュールを書けるようにすると、テスト用の資料を CLI だけで作れる
    (今は spec/examples/bundles の `short-hair.ugoemod` も JSON から作る道具が無い)。
