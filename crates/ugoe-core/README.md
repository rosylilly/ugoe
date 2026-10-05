# ugoe-core

**層:** L0 基盤 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

全クレートが共有する最下層の型 (数学型の再エクスポート、文字列 ID `Id<T>`、`Rgb`)。

## 守ること

- serde の derive 以外の、シリアライズ形式への依存を持たない (例外: コード生成用の ts-rs / schemars は feature `ts` / `schema` の裏に置く。[型の生成](../../docs/editor.md#型の生成))
- ドメインの概念 (パラメータやメッシュなど) を置かない。それは `ugoe-model` に置く
- `Id<T>` は種類 (`T`) の違う ID どうしを比べられない。文字列 (`str`・`&str`) とは両向きに比べられる (`id == "AngleX"`)。
  この比較を足したので、`id == "A".into()` や `ids == vec!["A".into()]` は `.into()` の行き先が決まらずコンパイルできない。
  `id == "A"`・`ids == ["A"]` と書く (T0159)
