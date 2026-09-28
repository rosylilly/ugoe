# 0008: バンドルのペイロードは MessagePack (フィールド名付き) と zstd

- 状態: Accepted
- 日付: 2026-09-28
- 関連: T0105、[ADR 0004](0004-file-formats.md)、[spec/file-formats.md](../spec/file-formats.md)

## 背景

`.ugoe` バンドルの `MODL` `PHYS` `MOTN` `EXPR` チャンクに `ugoe-model` の型をどう書くかを決める必要があった。制約は次のとおり。

- ADR 0004 により、`ugoe-model` の serde 実装をそのまま使う (形式ごとに別のデータモデルを持たない)。
- `ugoe-model` の型は JSON を主眼に作られており、内部タグ付き enum (`#[serde(tag = "type")]`: `Segment` `DeformerKind`)、隣接タグ付き enum (`CurveTarget`)、`#[serde(default)]` を使っている。
- ランタイム層なので wasm32 でビルドでき、純 Rust で軽いこと。壊れた入力でパニックしないこと。

候補だった postcard で実際に往復させたところ、`Model` のフィクスチャ (デフォーマなし) は通ったが、`Motion` は `This is a feature that PostCard will never implement` で失敗した。内部タグ付き enum は `deserialize_any` を必要とし、自己記述的でない形式では読めないためである。

## 決定

- ペイロードは **MessagePack** で、構造体を **フィールド名付きのマップ** として書く (`rmp-serde` の `to_vec_named`)。
- チャンク単位の圧縮には **zstd** を使い、純 Rust の `ruzstd` で圧縮・展開する。書き出し時の圧縮は任意 (既定は無圧縮)。テクスチャ (`TEXR`) は圧縮しない。
- 展開後の大きさには上限 (1 チャンク 256 MiB) を設け、MessagePack の入れ子の深さも 128 に制限する。

## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| postcard | 小さい・速い・no_std | 内部タグ付き enum を読めない。`ugoe-model` の serde 属性を変える必要があり、JSON 形式も変わる。フィールドの追加で互換が壊れる |
| bincode | 速い | postcard と同じく自己記述的でなく、内部タグ付き enum を読めない |
| JSON (+ zstd) | 追加の依存がない。デバッグしやすい | 浮動小数点の文字列変換で読み書きが遅く、大きい |
| CBOR (ciborium) | 自己記述的・標準化されている | MessagePack より依存が重く、得るものが少ない |
| 自前の SoA 形式 | ゼロコピーで最速にできる | 実装と保守が重い。スキーマが固まる前には早すぎる |
| MessagePack (配列表現) | フィールド名がなく小さい | フィールドの追加・`#[serde(default)]` の省略に弱い |

## 結果

- `ugoe-model` を変えずに全型が往復する。フィールド名付きなので、フィールドの追加や `#[serde(default)]` による省略にも JSON と同じように耐える。
- フィールド名の分だけ postcard より大きくなる (無圧縮でも `single_quad` フィクスチャで JSON のおよそ 8 割)。繰り返すフィールド名は zstd の圧縮で大きく縮む。
- 追加の依存は `rmp-serde` (+ `rmp`) と `ruzstd` (+ `twox-hash`)。いずれも純 Rust で wasm32 でビルドできる。
- 見直す条件: 読み込み時間が問題になったとき (`CompiledModel` のゼロコピー形式を検討する)。あるいは `ugoe-model` の enum を外部タグ付きに変えられるなら、postcard に切り替えてより小さく速くできる (コンテナ形式の `version` を上げる)。
