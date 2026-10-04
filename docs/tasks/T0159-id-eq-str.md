---
id: T0159
title: ugoe-core の Id<T> を文字列と比べられるようにする
status: todo
milestone: M9
crates: [ugoe-core, ugoe-rigging, ugoe-editor-core, ugoe-cli, ugoe-ai, ugoe-mcp, ugoe-model, ugoe-runtime, apps/editor, docs]
depends_on: []
assignee:
---

# T0159: ugoe-core の Id<T> を文字列と比べられるようにする

## 目的

T0142 で文字列のままの ID の参照を `Id<T>` にそろえた結果、テストや呼び出し側に `id.as_str() == "..."` が増えた。
`Id<T>` に `PartialEq<str>`・`PartialEq<&str>` (と逆向き) を足せば `id == "..."` と書ける。

**判断が要る:** `ugoe-core` の公開 API の追加 (crates.io に出すクレート)。型の違う ID どうしを比べられないという `Id<T>` の目的を
損なわないか (文字列との比較は型を問わない) を確かめて決める。足さないと決めたら、理由を `crates/ugoe-core/README.md` に書いて閉じる。

## 受け入れ条件

- [ ] 足すか決めた (上の判断)。足すなら `PartialEq<str>`・`PartialEq<&str>`・`PartialEq<Id<T>> for str`/`&str` を実装し、doc コメントとテストを書いた
- [ ] 足したなら、ワークスペースの `as_str() == "..."` の形を置き換えた (`crates:` の外のクレートのテストは、置き換えずに件数をメモに書くか、別のタスクにする)
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0142 のメモ (起票の候補)
- crates/ugoe-core/src/id.rs (`Id<T>`)

## メモ

- T0019 で起票した (元の候補: T0142)。

### 2026-10-04 blocked: 足すと既存のコードの型推論が壊れる (claude-worker-T0159)

**判断が要る点:** `Id<T>` に `PartialEq<str>`・`PartialEq<&str>`・`PartialEq<Id<T>> for str`/`&str` を足すと、
`ugoe-core` の追加だけでは済まず、`crates:` の外の既存のコードがコンパイルできなくなる。足すか、足すなら範囲を広げてよいか。

試したこと (実装は戻した):

- 4 つの impl・doc コメント・テストを `crates/ugoe-core/src/id.rs` に書いた。`just crate ugoe-core` は通る。
- 型の違う `Id<T>` どうしの比較は、足したあともコンパイルエラーのまま (impl は `Id<T>` と `str` の間だけ)。
  文字列との比較が種類を問わないのは `String == str` と同じで、`Id<T>` の目的は損なわない。ここは問題ない。
- ところが `cargo check --workspace --all-targets --all-features --keep-going` で **35 件の型推論のエラー** (E0282/E0283) が出た。
  `PartialEq` の相手の候補が `Id<T>` だけでなくなり、`.into()` の行き先が決まらなくなるため。
  - `x.id == y.into()` (ugoe-rigging のテストと例、ugoe-editor-core の inference_tests.rs)
  - `assert_eq!(v, vec!["A".into()])` / `r.colliders == ["A".into()]` (`Vec<Id<_>>` と配列の比較。ugoe-editor-core・ugoe-ai・ugoe-cli のテスト、ugoe-cli/src/sample/dress_up.rs)
  - `assert_eq!(g.bindings[0].parameter, "B".into())` (ugoe-editor-core/src/grid.rs のテスト)
  - ライブラリのコードでも 1 か所: ugoe-rigging/src/depth.rs の `|parameter: &Option<_>|` (`&Option<ParamId>` と書けば直る)
  - 件数: ugoe-rigging 16 (src 1・tests 13・examples 2)、ugoe-editor-core 15 (src のテスト 13・tests/m3 1・grid.rs 1)、ugoe-cli 4 (dress_up.rs 3・tests 1)、ugoe-ai 2。
    ugoe-editor-core のライブラリのビルドが grid.rs のテストで止まらない一方、それに依存する ugoe-mcp・apps/editor・apps/viewer・apps/studio のテストは今回の検査で全部は見えていない可能性がある (下流のクレートのエラーが隠れているかもしれない)。
- どれも機械的に直せる (`x.id == y` と書ける。`vec!["A".into()]` は `vec![ParamId::from("A")]` か `v == ["A"]`)。
  ただし crates.io に出す `ugoe-core` の利用者の同じ形のコードも壊す (Rust の慣習 (RFC 1105) では trait の impl の追加による推論の破壊は minor の変更として許されるが、壊れること自体は起きる)。いまの版は 0.0.0 で未公開。

案:

- **A. 足す。範囲を ugoe-rigging・ugoe-editor-core・ugoe-ai・ugoe-cli (と、出てきたら ugoe-mcp・apps) に広げて、壊れた 35 件と `as_str() == "..."` の置き換えを同じ PR で行う。** 公開前の今なら利用者は壊れない。差分は大きいが機械的。他のタスクと衝突しやすい (ugoe-editor-core のテストは多くのタスクが触る)。
- **B. 足さない。** `crates/ugoe-core/README.md` に「文字列と比べるときは `id.as_str() == "..."` か `id == Id::from("...")`。`PartialEq<str>` を足すと `.into()` を使う比較の型推論が壊れるため足さない」と書いて閉じる。
- C. 比較の trait は足さず、`Id::is(&self, s: &str) -> bool` のようなメソッドを足す。推論は壊れないが、`as_str() ==` と比べて得が小さい。

**推奨: A** (公開前で、壊れる箇所はどれもテストと 1 行の型注釈。`id == "..."` と `vec == ["..."]` が書けるようになり読みやすくなる)。
ただし範囲が 1 クレートから 5 クレート以上に広がり、並行しているタスクと衝突しやすいので、他のタスクが ugoe-editor-core のテストを触っていない時期に回すのがよい。
B でも実害はない (今の書き方のまま)。

実装の差分 (A を選んだときの出発点): ugoe-core の 4 つの impl とテスト、ugoe-rigging/src/depth.rs の型注釈。

- **判断 (2026-10-05、利用者の判断):** 案 A。impl を入れ、型推論が壊れる 35 か所 (rigging 16・editor-core 15・cli 4・ai 2、ほか見つかれば) を同じタスクで機械的に直す。`id.as_str() == "..."` も置き換える。実装の差分は以前のワーカーが残したパッチ (scratchpad の t0159-impl.patch) を参考にしてよい。
