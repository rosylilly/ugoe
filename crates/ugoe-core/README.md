# ugoe-core

**層:** L0 基盤 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

全クレートが共有する最下層の型 (数学型の再エクスポート、文字列 ID `Id<T>`、`Rgb`、光源の型 `lighting::{Light, Lighting, MAX_LIGHTS}`)。

光源の型は、参照レンダラ (`ugoe-render`) と GPU に依存しない組み込み先 (`ugoe` の `render` 無し・C ABI・GDExtension) の両方が使うので、
両方が依存できるここに置く。ホストがレンダラに渡す状態で、モデルのデータではない ([docs/architecture.md](../../docs/architecture.md#レイヤーと依存ルール)、T0166)。

## 守ること

- `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない (`just nostd-check`。[規則](../../docs/portability.md#ランタイム層が守る制約-移植性のため))
- serde の derive 以外の、シリアライズ形式への依存を持たない (例外: コード生成用の ts-rs / schemars は feature `ts` / `schema` の裏に置く。[型の生成](../../docs/editor.md#型の生成))
- ドメインの概念 (パラメータやメッシュなど) を置かない。それは `ugoe-model` に置く (光源の型は上の理由による例外)
- `Id<T>` は種類 (`T`) の違う ID どうしを比べられない。文字列 (`str`・`&str`) とは両向きに比べられる (`id == "AngleX"`)。
  この比較を足したので、`id == "A".into()` や `ids == vec!["A".into()]` は `.into()` の行き先が決まらずコンパイルできない。
  `id == "A"`・`ids == ["A"]` と書く (T0159)
