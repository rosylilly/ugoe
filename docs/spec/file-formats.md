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

- 未知のタグのチャンクは読み飛ばします (前方互換)。未知のタグのチャンクは中身も展開もしません。
- チャンク表の順序は自由ですが、書き出しでは `MODL` `PHYS` `MOTN`… `EXPR`… `META` `TEXR`… の順にします。同じタグのチャンク (`MOTN` `EXPR` `TEXR`) は表の順が意味を持ちます。
- ヘッダの `flags` と、チャンクの `flags` の bit0 以外は予約です。0 でなければ読み込みをエラーにします。

### ペイロードのエンコード

判断の経緯は [ADR 0008](../adr/0008-bundle-payload-encoding.md)。

| チャンク | エンコード |
| --- | --- |
| `MODL` `PHYS` `MOTN` `EXPR` | **MessagePack**。構造体はフィールド名付きのマップ (`rmp-serde` の `to_vec_named`)。enum の表現は JSON と同じく serde の属性に従う |
| `TEXR` | PNG / KTX2 ファイルの中身そのまま。形式は先頭のシグネチャで判別する |
| `META` | UTF-8 の JSON |

- postcard などの自己記述的でない形式は、`ugoe-model` の内部タグ付き enum (`Segment` `DeformerKind`) を読めないため採用しませんでした。
- `flags` の bit0 が立ったチャンクは、ペイロード全体が 1 つの zstd フレームです。書き出し時の圧縮は任意で (既定は無圧縮)、`TEXR` は圧縮しません。展開後の大きさの上限は 1 チャンクあたり 256 MiB です。

### 読み込み時の検査

`read_bundle` は入力を信用せず、次をすべてエラーとして返します (パニックしない)。

- 切り詰め、マジックの不一致、未対応のコンテナ `version`、予約フラグ
- チャンクの範囲外 (オフセット・長さのオーバーフローを含む)、16 バイト境界でないオフセット、ヘッダ・チャンク表・他のチャンクとの重なり
- `MODL` がない、`MODL` `PHYS` `META` が 2 つ以上ある
- `TEXR` の数が `Model.textures` と一致しない、PNG / KTX2 でないテクスチャ
- ペイロードのデコード失敗、`schema_version` が新しすぎる
- `ugoe_model::validate::validate` がエラーを返すモデル (下流の変形・描画で範囲外アクセスを起こさないため、ここで弾く)

### API (`ugoe-format`)

- `Bundle { model, physics, motions, expressions, textures, meta }`。テクスチャは `TextureData { format: Png | Ktx2, bytes }`。
- `write_bundle(&Bundle) -> Result<Vec<u8>, FormatError>` / `write_bundle_with(&Bundle, WriteOptions { compress })`
- `read_bundle(&[u8]) -> Result<Bundle, FormatError>`
- ファイルシステムには触れません。読み書きはバイト列で行います。

> 未決: 読み込み時間を短くするため、`CompiledModel` をそのまま (ゼロコピーで) 読めるレイアウトにするか。まずは素直なシリアライズで実装し、計測してから判断する。

## 機能の宣言と段階的な劣化

判断の経緯は [ADR 0013](../adr/0013-forward-compatibility.md)。

> 実装待ち: 仕組みは T0109 で実装します。1.0 より前に完成させます。

### 機能の宣言

バンドルは `FEAT` チャンク (T0109 で追加) で、使っている機能を宣言します。

```
features: [
  { id: "mask.intersection", required: false },
  { id: "physics.dynamic",   required: false },
  ...
]
```

- 基準の機能 (1.0 の時点の機能) は宣言しません。
- 読み込むランタイムは、宣言のうち知らない機能を調べます。
  - `required: true` の機能を 1 つでも知らなければ、`UnsupportedFeatures(ids)` エラーで読み込みを拒否します。
  - `required: false` の機能は、下の表のフォールバックを適用して読み込みを続け、警告の一覧を返します。

### 1.0 の基準の機能

[ADR 0018](../adr/0018-licensing-and-1.0-scope.md) により、1.0 は AI 連携を除く現在定義しているすべての機能を含みます。
次の表の機能は **1.0 の基準の機能** で、1.0 以降のファイルでは宣言しません。1.0 のランタイムはすべてに対応します。
表のフォールバックは、1.0 より前の開発中のランタイムで互換性を試すときと、仕組みの試験 (T0109・T0115) に使います。

### 機能とフォールバックの一覧

1.0 以降に機能を追加するときは、必ずこの表の下の「1.0 以降の機能」に行を追加し、フォールバックを定義します。

| 機能 ID | 内容 | フォールバック |
| --- | --- | --- |
| `authoring.keyform_origins` | キーフォームの由来と推定の設定 ([keyform-inference.md](keyform-inference.md)) | 読み飛ばす (ランタイムに影響しない) |
| `mask.intersection` | マスクの共通部分 | 和集合として扱う |
| `physics.dynamic` | 動的物理の出力先と衝突体 ([physics.md](physics.md#動的物理-揺れ用パラメータのいらない物理)) | パラメータ以外への出力と衝突体を無視する |
| `lighting.normal_map` | 法線マップとライティング ([rendering.md#ライティング](rendering.md#ライティング)) | ライティングなしで描く |
| `modules` | 着せ替えのスロット ([modules.md](modules.md)) | スロットを無視し、本体だけを表示する |
| `avatar.profile` | アバタープロファイル ([avatar-profile.md](avatar-profile.md)) | 読み飛ばす (アプリの連携機能が使えないだけ) |

上の表はすべて 1.0 の基準の機能です。

#### 1.0 以降の機能

| 機能 ID | 内容 | フォールバック |
| --- | --- | --- |
| (まだない) | | |

### 未知の値の読み込み

- 構造体の未知のフィールドは無視します。
- 拡張されうる enum (合成モード、デフォーマの種類、物理の出力先、カーブの対象など) は、未知のタグを `Unknown` として読みます。`Unknown` の要素にはその種類のフォールバック (上の表、またはデフォーマなら恒等変換、合成モードなら通常) を適用します。
- 互換性のテストのため、各リリースで書き出したファイルを `assets/compat/` に保存し、新しいランタイムで読めること、新しい機能を含むファイルを古い読み込みルールで劣化させて読めることを CI で検査します (T0115)。

### モジュールと法線マップのファイル

- 着せ替えのモジュールは、ヘッダが同じ形式のファイル (拡張子は `.ugoemod`) で、`MODL` の代わりに `MODU` チャンクを持ちます (T0113)。ヘッダの予約フラグは使いません。
- 法線マップは `TEXR` チャンクと同じ形式の `NRML` チャンクに入れ、`TextureRef::normal_map` から参照します (T0112)。

## バージョニング

- `ugoe_model::SCHEMA_VERSION` はデータのスキーマのバージョンです。互換性を壊す変更をしたら 1 上げます。
- 読み込み時は `schema_version` を必ず検査します。
  - 現在のバージョンより新しい → `UnsupportedVersion` エラー
  - 現在のバージョンより古い → マイグレーションを順に適用して現在のバージョンにする (`ugoe-format` の `migrate` モジュール。マイグレーションが必要になったときに作る)
- 1.0 より前 (M4 まで) は互換性を保証しません。`SCHEMA_VERSION` は 0 のままにし、形式を壊してよいものとします。
- 1.0 以降は、機能の追加を上の「機能の宣言」で行い、`SCHEMA_VERSION` は互換性を壊す変更 (必須の機能でも表現できない変更) のときだけ上げます。
