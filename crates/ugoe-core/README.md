# ugoe-core

**層:** L0 基盤 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

全クレートが共有する最下層の型 (数学型の再エクスポート、文字列 ID `Id<T>`、`Rgb`、光源の型 `lighting::{Light, Lighting, MAX_LIGHTS}`) と、
ランタイム層が使う数学関数 `math` (`sin`・`atan2`・`powf` など。`std` の有無にかかわらず常に `libm`。IEEE で結果が決まる `sqrt` だけは
`std` があれば `std` の命令で、結果のビット列は同じ)。

光源の型は、参照レンダラ (`ugoe-render`) と GPU に依存しない組み込み先 (`ugoe` の `render` 無し・C ABI・GDExtension) の両方が使うので、
両方が依存できるここに置く。ホストがレンダラに渡す状態で、モデルのデータではない ([docs/architecture.md](../../docs/architecture.md#レイヤーと依存ルール)、T0166)。

## 守ること

- `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない (`just nostd-check`。[規則](../../docs/portability.md#ランタイム層が守る制約-移植性のため))
- 数学関数は `std` の `f32::sin` などではなく `math` (`libm`) を使い、`glam` も feature `libm` で使う。OS でも `std` の有無でも結果のビット列が
  変わらないように ([規則](../../docs/portability.md#ランタイム層が守る制約-移植性のため))。`clippy.toml` の `disallowed-methods` が検査する
  (ランタイム層の各クレートに同じものを置く)
- `math::sqrt` は、`std` があれば `f32::sqrt` (CPU の命令) を使う。安定版の Rust の `libm` は wasm32 で `f32.sqrt` を使えず、ソフトウェアで
  反復するので、ブラウザで物理の段が約 2 倍遅かったため (T0193、[docs/testing.md](../../docs/testing.md#1-フレームの評価))。平方根は正しく丸めると
  決まっているので結果のビット列は変わらない (`libm` との一致は単体テストで確かめる)。`glam` の中の平方根 (`length`・`normalize`) は `libm` のままなので、
  毎フレーム何百回も長さを求める箇所は `math::sqrt(v.length_squared())` と書く (`ugoe-physics` の衝突の判定)
- serde の derive 以外の、シリアライズ形式への依存を持たない (例外: コード生成用の ts-rs / schemars は feature `ts` / `schema` の裏に置く。[型の生成](../../docs/editor.md#型の生成))
- ドメインの概念 (パラメータやメッシュなど) を置かない。それは `ugoe-model` に置く (光源の型は上の理由による例外)
- `Id<T>` は種類 (`T`) の違う ID どうしを比べられない。文字列 (`str`・`&str`) とは両向きに比べられる (`id == "AngleX"`)。
  この比較を足したので、`id == "A".into()` や `ids == vec!["A".into()]` は `.into()` の行き先が決まらずコンパイルできない。
  `id == "A"`・`ids == ["A"]` と書く (T0159)
