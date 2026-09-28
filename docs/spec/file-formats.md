# ファイル形式仕様

対応するコードは `crates/ugoe-format` (ランタイム形式) と `crates/ugoe-editor-core` (プロジェクト形式)。判断の経緯は [ADR 0004](../adr/0004-file-formats.md)。

## 一覧

| 種類 | 拡張子 | 形式 | 用途 |
| --- | --- | --- | --- |
| プロジェクト | `<name>.ugoeproj/` (ディレクトリ) | JSON + PNG + 元の PSD | エディタでの編集。git で差分を見られる |
| バンドル | `<name>.ugoe` | バイナリのコンテナ | ランタイムでの配布。1 ファイルで完結 |
| モーション | `<name>.motion.json` | JSON | 単体で配布・共有するとき |
| 表情 | `<name>.expression.json` | JSON | 同上 |
| 物理 | `<name>.physics.json` | JSON | 同上 |

## プロジェクト (`.ugoeproj/`)

```
character.ugoeproj/
  project.json          プロジェクトのメタ情報 (形式のバージョン、エディタの設定)
  model.json            ugoe_model::Model (整形済み JSON。キーの順序を固定する)
  physics.json          PhysicsSettings (任意)
  motions/*.motion.json
  expressions/*.expression.json
  textures/<name>.png   レイヤー単位の非乗算 RGBA 画像 (アトラス化する前の状態)
  sources/*.psd         取り込み元の PSD (任意)
  .ugoe-cache/          サムネイルなどのキャッシュ (git 管理外)
```

- JSON はすべて 2 スペースでインデントし、フィールドは型定義の順に出力します。浮動小数点は最短の往復表現で書きます (`serde_json` の既定)。こうすることで、エディタで保存するたびに無関係な差分が出るのを防ぎます。
- テクスチャのアトラス化はビルド時に行います。プロジェクト内では、レイヤーごとに別々の画像として持ちます。

## バンドル (`.ugoe`)

チャンク形式の、リトルエンディアンのバイナリです。

```
Header (16 bytes)
  magic         [u8; 4] = "UGOE"
  version       u32     コンテナ形式のバージョン (現在 0)
  flags         u32     予約 (0)
  chunk_count   u32
ChunkTable (chunk_count × 24 bytes)
  tag           [u8; 4]  "MODL" | "PHYS" | "MOTN" | "EXPR" | "TEXR" | "META"
  flags         u32      bit0: zstd で圧縮
  offset        u64
  length        u64
Chunks ...      各チャンクは 16 バイト境界に揃える
```

| チャンク | 内容 |
| --- | --- |
| `MODL` | `Model` をバイナリにシリアライズしたもの。1 つだけ |
| `PHYS` | `PhysicsSettings`。0〜1 個 |
| `MOTN` | `Motion`。複数可 |
| `EXPR` | `Expression`。複数可 |
| `TEXR` | テクスチャ 1 枚 (PNG または KTX2)。`Model.textures` と同じ順に並べる |
| `META` | 作者やライセンスなどの自由なメタデータ (JSON) |

- 未知のタグのチャンクは読み飛ばします (前方互換)。
- バイナリのシリアライズ方式は T0105 で決めます (候補: postcard、自前の SoA 形式)。決まったらこの文書を更新します。

> 未決: 読み込み時間を短くするため、`CompiledModel` をそのまま (ゼロコピーで) 読めるレイアウトにするか。まずは素直なシリアライズで実装し、計測してから判断する。

## バージョニング

- `ugoe_model::SCHEMA_VERSION` はデータのスキーマのバージョンです。互換性を壊す変更をしたら 1 上げます。
- 読み込み時は `schema_version` を必ず検査します。
  - 現在のバージョンより新しい → `UnsupportedVersion` エラー
  - 現在のバージョンより古い → マイグレーションを順に適用して現在のバージョンにする (`ugoe-format` の `migrate` モジュール。マイグレーションが必要になったときに作る)
- 1.0 より前 (M4 まで) は互換性を保証しません。`SCHEMA_VERSION` は 0 のままにし、形式を壊してよいものとします。
