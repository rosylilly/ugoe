# ファイル形式仕様

対応するコードは `crates/ugoe-format` (ランタイム形式) と `crates/ugoe-editor-core` (プロジェクト形式)。判断の経緯は [ADR 0004](../adr/0004-file-formats.md)。
バンドルとモジュールのファイル・機能の宣言は、英語の公開仕様 [`spec/bundle.md`](../../spec/bundle.md) にもあります (対応は [public-spec.md](public-spec.md)。食い違ったら英語版が正)。プロジェクト形式は公開仕様に含めません。

## 一覧

| 種類 | 拡張子 | 形式 | 用途 |
| --- | --- | --- | --- |
| プロジェクト | `<name>.ugoeproj/` (ディレクトリ) | JSON + PNG + 元の PSD | エディタでの編集。git で差分を見られる |
| バンドル | `<name>.ugoe` | バイナリのコンテナ | ランタイムでの配布。1 ファイルで完結 |
| モジュール | `<name>.ugoemod` | バンドルと同じコンテナ | 着せ替えのモジュールの配布 ([モジュール](#モジュール-ugoemod)) |
| モーション | `<name>.motion.json` | JSON | 単体で配布・共有するとき |
| 表情 | `<name>.expression.json` | JSON | 同上 |
| 物理 | `<name>.physics.json` | JSON | 同上 |

## プロジェクト (`.ugoeproj/`)

```
character.ugoeproj/
  project.json          プロジェクトのメタ情報 (形式のバージョン、名前、エディタの設定)
  model.json            ugoe_model::Model (整形済み JSON。キーの順序を固定する)
  physics.json          PhysicsSettings (任意。ないときはファイルを置かない)
  motions/<name>.motion.json          Motion (<name> は Motion::name)
  expressions/<name>.expression.json  Expression (<name> は Expression::name)
  textures/<name>.png   レイヤー単位の非乗算 RGBA 画像 (アトラス化する前の状態。<name> は TextureRef::name)
                        と法線マップの画像 (<name> は TextureRef::normal_map)
  sources/*.psd         取り込み元の PSD (任意。保存では触れない)
  .gitignore            `.ugoe-cache/` を書く
  .ugoe-cache/          サムネイルなどのキャッシュ (git 管理外。保存・読み込みでは触れない)
```

読み書きは `ugoe_editor_core::project` (`Project::save` / `Project::load`) が行います。

着せ替えのモジュールのプロジェクト ([モジュールのプロジェクト](#モジュールのプロジェクト)) は、`model.json` の代わりに `module.json` を置きます。

### `project.json`

| フィールド | 型 | 内容 |
| --- | --- | --- |
| `format_version` | u32 | プロジェクトの形式のバージョン (`PROJECT_FORMAT_VERSION`、現在 0)。ディレクトリ構成を変えたら上げる。モデルの `schema_version` とは別 |
| `name` | string | プロジェクトの表示名。エディタで「別名で保存」するとディレクトリ名 (拡張子を除く) になる |
| `kind` | string | プロジェクトの種類。`model` (本体のモデル。既定で、書き出さない) か `module` (着せ替えのモジュール) |
| `module` | object | モジュールのプロジェクトが参照する本体とスロット (`kind` が `module` のとき必須)。`base` は本体 (`.ugoeproj` のディレクトリか `.ugoe`) のパスで、保存するとプロジェクトのディレクトリからの相対パス (`/` 区切り) で書く (根が違えば絶対パス)。`slot` は取り付け先のスロットの ID |
| `editor` | object | エディタの設定 (自由な JSON)。空なら書き出さない |
| `editor.ai.send_externally` | bool | `false` なら、このプロジェクトのデータを外部の AI サービスに送らない ([エディタ](../editor.md#ai-の設定と送信の同意))。既定 (送ってよい) のときは書き出さない。省略・形の違う値は送ってよい扱い |
| `editor.lighting` | object | エディタのビューポートとプレビューで当てる光源 (`PreviewLighting`。モデルには入らない)。`enabled` (bool)・`ambient` (RGB 0〜1)・`ambient_intensity`・`lights` (最大 8 個。`{"type": "directional", "direction", "color", "intensity"}` か `{"type": "point", "position", "color", "intensity", "range"}`)。すべて既定値なら書かない。足りないフィールドは既定値、読めない・範囲の外ならすべて既定値で読む ([editor.md#ライティングの画面](../editor.md#ライティングの画面)) |
| `editor.dress_up` | object | エディタのプレビューで試す着せ替えのモジュール。`modules` (`[{"path": string, "attached": bool}]`。`.ugoemod` のパスと、プレビューで付けるか)。パスは `.ugoeproj` と同じフォルダの中なら相対パス (`/` 区切り)、それ以外は絶対パス。空なら書かない。読めなければ空、`attached` の欠けは `false` で読む ([editor.md#プレビューの着せ替え](../editor.md#プレビューの着せ替え)) |
| `editor.rigged_meshes` | object | AI のモジュールのリグの提案が本体の変形を写した衣装のメッシュの、採用した (または知らせを消した) ときの形の指紋 (`{"<メッシュの ID>": "<16 桁の 16 進>"}`。形は親のデフォーマの ID・UV・三角形の FNV-1a 64 ビット)。形を編集したら提案し直すよう知らせるために比べる ([editor.md](../editor.md#モジュールのプロジェクトでの-ai-の提案))。ID はファイルの中の ID。覚えたものがなければ書かない。読めない値・文字列でない値は捨てる |
| `editor.export` | object | 書き出しの設定 (`ExportSettings`)。`atlas` (bool、既定 `false`)・`atlas_max_size` (1〜16384、既定 2048)・`atlas_padding` (0〜64、既定 2)。すべて既定値なら書かない。足りないフィールドは既定値、読めなければすべて既定値で読む ([editor.md#書き出し](../editor.md#書き出し)) |

- `format_version` が読める版より新しければ、読み込みをエラーにします。
- `kind`・`module` がない既存のプロジェクトは、本体のモデルのプロジェクトとしてそのまま読めます (`format_version` は変えていません)。

```json
{
  "format_version": 0,
  "name": "CatEars",
  "kind": "module",
  "module": { "base": "../avatar.ugoeproj", "slot": "Headwear" }
}
```

### 出力の安定性

- JSON はすべて 2 スペースでインデントし、フィールドは型定義の順に出力し、末尾に改行を 1 つ付けます。浮動小数点は最短の往復表現で書きます (`serde_json` の既定)。こうすることで、エディタで保存するたびに無関係な差分が出るのを防ぎます。同じ内容を保存すると、バイト単位で同じファイルになります。
- 内容が変わらないファイルは書き直しません。書き直すファイルは、同じディレクトリの一時ファイル (`<file>.ugoe-tmp`) に書いてから名前を変えて置き換えます。書き込み中に落ちても、既存のファイルが途中までの中身で残ることはありません (複数のファイルをまたぐ原子性はありません)。`project.json` は最後に書きます。
- 保存すると、今の内容にない `motions/*.motion.json`・`expressions/*.expression.json`・`textures/*.png` と `physics.json` を消します。それ以外のファイルには触れません。
- 空でなく `project.json` もないディレクトリには保存しません (別のファイルを上書きしないため)。

### 名前と読み込み

- テクスチャ・モーション・表情の名前はファイル名になるので、`[A-Za-z0-9_.-]` だけからなり `.` で始まらない 128 バイト以下の文字列に限ります。大文字と小文字だけが違う名前は重複とみなします (大文字小文字を区別しないファイルシステムで衝突しないように)。保存の前にすべて検査し、違反があれば何も書きません。
- 読み込みでは、モーションと表情をファイル名の順に並べます。`Model::textures` のすべてに `textures/<name>.png` (PNG のシグネチャを持つもの) が必要です。参照されない PNG は無視します。
- 法線マップの画像 ([model.md#ライティング](model.md#ライティング)) は、テクスチャと同じ `textures/` に `TextureRef::normal_map` の名前で置きます (`textures/<normal_map>.png`)。名前の規則と大文字小文字の重複の検査は、テクスチャの名前と合わせて行います。複数のテクスチャが同じ法線マップを指してもよく、画像は 1 枚だけ置きます。テクスチャの名前と同じ名前の法線マップは、同じ画像を指します。
  読み込みでは `normal_map` のすべてに画像が必要です (ないと `MissingTexture`)。保存では、どこかの `normal_map` が指す画像を使われているものとして扱い、消しません。法線マップを持たない既存のプロジェクトは、そのまま読めます (形式のバージョンは変えない)。
- モデルは読み込み時に検証しません (不正なモデルもエディタで開いて直せるように)。`schema_version` は検査します。
- テクスチャのアトラス化はビルド時に行います ([テクスチャのアトラス化](#テクスチャのアトラス化))。プロジェクト内では、レイヤーごとに別々の画像として持ちます。
- `.ugoe` への書き出し (`ugoe_editor_core::export_bundle`) では、テクスチャを `Model::textures` の順に `TEXR` に入れ、法線マップを `normal_map` を持つテクスチャの順に `NRML` に入れ、物理・モーション・表情も含めます。

### モジュールのプロジェクト

着せ替えのモジュール ([modules.md](modules.md)) を作るプロジェクトです ([ADR 0024](../adr/0024-module-authoring.md)、[editor.md#モジュールのプロジェクト](../editor.md#モジュールのプロジェクト))。

```
cat_ears.ugoeproj/
  project.json          kind = "module" と、本体とスロット (module.base・module.slot)
  module.json           ugoe_model::Module (整形済み JSON。`ugoe_format::module_to_json`)
  textures/<name>.png   モジュールのテクスチャと法線マップ (<name> は Module::textures の name と normal_map)
  .gitignore
```

- `module.json` はモジュールだけを持ちます (本体の要素・画像は含めません)。ID はモジュールの中の ID (合成の接頭辞 `<name>/` を付けない)、
  最上位の要素はスロットの取り付け先の局所座標、描画順は 0〜1000 です。物理 (`rigs`・`colliders`) は `module.json` の中にあり、`physics.json` は置きません。
  モーション・表情も持ちません。`slot` は `project.json` の `module.slot` と同じ値で書きます (読み込みでは `project.json` のものを使う)。
- 読み込みでは本体も読みます。本体が見つからなければ `BaseNotFound`、スロットがなければ `UnknownSlot` のエラーで、エディタは本体を選び直して開けます。
  本体の読み込みの間違い (壊れた `.ugoe`、PNG でないテクスチャ、本体がモジュールのプロジェクト) もエラーです。
- 保存すると、`model.json`・`physics.json` を消します (本体のプロジェクトとして保存し直したときは、逆に `module.json` を消します)。
- `.ugoemod` への書き出し (`ugoe_editor_core::export_module`) は、`validate_module` と本体との合成 (`compose`・`compose_physics`) を確かめてから、
  テクスチャを `Module::textures` の順に `TEXR` に、法線マップを `NRML` に入れます。

### テクスチャのアトラス化

判断の経緯は [ADR 0016](../adr/0016-draw-batching.md)。`.ugoe` への書き出しで、レイヤーごとのテクスチャを数枚のアトラスにまとめ、
描画順で隣り合うメッシュが同じテクスチャを使うようにします。レンダラが描画をまとめられるようになります
([rendering.md#描画の最適化](rendering.md#描画の最適化))。書き出しのオプションで、既定では行いません
(`ugoe build --atlas`、`ugoe_editor_core::build_atlas` / `Project::with_atlas`)。

- **見た目は変えません。** 各画像の周りに、端の画素を `padding` (既定 2px) の幅だけ複製します。
  参照レンダラのサンプラ (バイリニア・端でクランプ・ミップマップなし) では、UV が 0〜1 の範囲なら元の画像と同じ色になります。
- メッシュの `texture` をアトラスの番号に、`uvs` をアトラスの中の座標に書き換えます (`(置いた位置 + uv × 画像の大きさ) / アトラスの大きさ`)。
- 画像は、既定のパラメータでの描画順で最初に使われる順に詰めます。1 枚のアトラス (幅・高さとも `max_size`、既定 2048px。
  WebGL2 でも使える大きさ) に収まらなくなったら次のアトラスに移ります。描画順で近いメッシュが同じアトラスに入ります。
  アトラスの中では、画像を高さの順に棚に並べます。大きさは 2 の冪に揃えません。
- 次のテクスチャはまとめずに、元の名前と画像のまま残します: 使うメッシュの UV が 0〜1 の外に出るもの (端のクランプに頼っている)、
  周りの複製を含めると 1 枚で `max_size` を超えるもの。どのメッシュも使わないテクスチャは取り除きます。
- 出力の `Model::textures` は、アトラス (`atlas-0`、`atlas-1`…。残したテクスチャの名前と重なれば番号を飛ばす) の後ろに、
  残したテクスチャを元の順で並べます。アトラスの画素は非乗算 RGBA8 の PNG で、画像を置いていない所は透明です。
- **画素がまったく同じ画像** (デコードした大きさと非乗算 RGBA8 の画素が一致するもの) は、描画順で先に使われるものだけを詰め、
  ほかのテクスチャを使うメッシュもその場所を指します (PSD の左右の目のように、同じ絵のレイヤーを複製したとき)。
  名前の違う別のテクスチャとしては残しません。
- **着せ替えのモジュール** (`ugoe_model::module`、[modules.md](modules.md)) のテクスチャは、本体のアトラスには入れません。
  モジュールは自分の `textures` を持ち、実行時に本体の後ろへ付け足して差し替えるので、本体のアトラスに入れると
  差し替えのたびにアトラスを作り直すことになるためです。モジュールのテクスチャは **モジュールの単位で** まとめます
  (`.ugoemod` の書き出しのオプション。`ugoe build <m>.ugoemod --atlas`、`ugoe_editor_core::build_module_atlas`)。
  規則は上と同じで、`Module::textures` と `Module::meshes` の `texture`・`uvs` を書き換えます。ただしモジュールは本体に付けないと
  描画順が決まらないので、画像は **メッシュの宣言順で** 最初に使われる順に詰めます。モジュールのほかの部分は変えません。
- 同じ入力と設定からは、同じ出力になります。

## バンドル (`.ugoe`)

チャンク形式の、リトルエンディアンのバイナリです。

```
Header (16 bytes)
  magic         [u8; 4] = "UGOE"
  version       u32     コンテナ形式のバージョン (現在 0)
  flags         u32     予約 (0)
  chunk_count   u32
ChunkTable (chunk_count × 24 bytes)
  tag           [u8; 4]  "MODL" | "PHYS" | "MOTN" | "EXPR" | "TEXR" | "NRML" | "META" | "FEAT" | "MODU"
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
| `NRML` | 法線マップ 1 枚 (PNG または KTX2)。`Model.textures` のうち `normal_map` を持つものと同じ数・同じ順に並べる ([rendering.md#ライティング](rendering.md#ライティング)) |
| `META` | 作者やライセンスなどの自由なメタデータ (JSON) |
| `FEAT` | 使っている機能の宣言 ([機能の宣言](#機能の宣言))。0〜1 個 |
| `MODU` | 着せ替えのモジュール。`.ugoemod` だけに置く ([モジュール](#モジュール-ugoemod))。`.ugoe` にあれば読み込みをエラーにする |

- 未知のタグのチャンクは読み飛ばします (前方互換)。未知のタグのチャンクは中身も展開もしません。
- 先頭のバイトが `X` (0x58) のタグは、読み込みと書き出しの試験 (未知のチャンクを読み飛ばすかどうかなど) のために予約しています。この形式のチャンクの種類には使いません。
- チャンク表の順序は自由ですが、書き出しでは `FEAT` `MODL` `PHYS` `MOTN`… `EXPR`… `META` `TEXR`… `NRML`… の順にします。宣言する機能がなければ `FEAT` は書きません。同じタグのチャンク (`MOTN` `EXPR` `TEXR` `NRML`) は表の順が意味を持ちます。
- ヘッダの `flags` と、チャンクの `flags` の bit0 以外は予約です。0 でなければ読み込みをエラーにします。

### ペイロードのエンコード

判断の経緯は [ADR 0008](../adr/0008-bundle-payload-encoding.md)。

| チャンク | エンコード |
| --- | --- |
| `MODL` `MODU` `PHYS` `MOTN` `EXPR` `FEAT` | **MessagePack**。ペイロード (圧縮されていれば展開後) はちょうど 1 つの MessagePack の値。構造体はフィールド名付きのマップ (`rmp-serde` の `to_vec_named`)。enum の表現は JSON と同じく serde の属性に従う |
| `TEXR` `NRML` | PNG / KTX2 ファイルの中身そのまま。形式は先頭のシグネチャで判別する。テクスチャの画素は、どちらの形式でも乗算していないアルファ (KTX2 の扱いは [rendering.md#前提](rendering.md#前提)) |
| `META` | UTF-8 の JSON |

- postcard などの自己記述的でない形式は、`ugoe-model` の内部タグ付き enum (`Segment` `DeformerKind`) を読めないため採用しませんでした。
- `flags` の bit0 が立ったチャンクは、ペイロード全体が 1 つの zstd フレームです。書き出し時の圧縮は任意で (既定は無圧縮)、`TEXR` と `NRML` は圧縮しません。書き出しは、展開後が 256 MiB (`MAX_DECOMPRESSED_CHUNK_LEN`) を超えるチャンクを圧縮しません (公開仕様では SHOULD NOT。既定の上限の読み込み器で読めるようにするため)。
- 形式は、1 チャンク・ファイル全体のどちらの展開後の大きさにも上限を決めません。上限は読む側の方針です ([読み込みの上限](#読み込みの上限))。
- MessagePack の入れ子の深さは 128 までに制限して読みます (スタックを使い切らないため。モデルの型の入れ子はずっと浅い)。
- MessagePack の値の後ろに余りのバイトを書きません。読み込みは余りを無視します (公開仕様では、拒否する読み込み器も適合します)。

### 読み込みの上限

数 KiB の zstd のフレームが数百 MiB に展開されることがあり、そうしたチャンクを並べると、小さなファイルでもメモリを使い切れます。
ファイルそのものに上限を決めると使える範囲を狭めるだけなので、上限は **読み込み器の設定** として持ちます (T0150。公開仕様は `spec/bundle.md` の Resource limits)。

- 読み込み器は資源の上限を設けてよい (MAY) とします。超えるときは、落ちたりメモリを使い切ったりせず、読み込みをやめてエラーを返さなければなりません (MUST)。
  展開しきってから大きさを調べたり、zstd のフレームに書かれた大きさを信用したりせず、展開しながら超えたことに気づくのがよい (SHOULD) とします。
- 1 チャンクの展開後の大きさに上限を設けるなら、その既定値は 256 MiB 以上にします (MUST。書き出しが頼れる大きさ)。合計の既定値もそれ以上にするのがよい (SHOULD) とします。利用者はより小さな値を指定できます。
- 参照実装 (`ugoe-format`) の上限は `ReadLimits { max_chunk_decompressed, max_total_decompressed }` で、既定値 (`ReadLimits::default()`) は 1 チャンク 256 MiB・合計 1 GiB です。
  `read_bundle_with(bytes, limits)` / `read_module_with(bytes, limits)` で指定し、`read_bundle` などは既定値で読みます。
  - 展開は、1 チャンクの上限と合計の残りの小さいほうまでしか読みません。それを超えるバイトが続けば、展開の途中でやめて `ReadLimitExceeded { tag, limit, max }` を返します
    (`limit` は `ChunkDecompressed` か `TotalDecompressed`、`max` はその上限の値)。先に確保はしません。
  - 合計に数えるのは、圧縮したチャンクを展開したバイトだけです。圧縮していないチャンクは入力のバイト列そのもので、大きさは呼び出し側が分かる入力の長さで抑えられるので数えません。展開に失敗したチャンクと、読み飛ばす未知のタグのチャンクも数えません。
  - エディタなどの画面に出すときのコードは `format.chunk_limit_exceeded` / `format.total_limit_exceeded` です ([editor.md](../editor.md#エラーと警告のコード))。ファイルが壊れているとは限らないので、`format.corrupt` と分けています。
  - 組み込む側からの指定 (T0152): SDK は `ugoe::format::read_bundle_with` / `read_module_with` をそのまま使い、超えたときの種類と補足の値は
    `ugoe::limits::ReadLimitExceeded` (`read_limit_exceeded`。`limit`・`chunk`・`max`) が決めます。C ABI は `UgoeReadLimits` と
    `ugoe_puppet_create_from_bundle_with_limits`・`ugoe_module_create_with_limits` (超えたら `UGOE_ERROR_LIMIT_EXCEEDED`。[crates/ugoe-ffi/README.md](../../crates/ugoe-ffi/README.md#読み込みの上限-t0152))、
    wasm は `load_bundle` / `load_module` の上限の JSON で、PNG の一辺の上限 (`max_image_side`) も指定できます
    ([crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#読み込みの上限-t0152))。`@ugoe/runtime` は `loadModel` の `limits`。
- テクスチャと法線マップの PNG は、コンテナの読み込み (`read_bundle` など) ではデコードしません (バイト列のまま返します)。
  デコードする側のための関数は `ugoe_format::image` (feature `png`) の `decode_png(bytes, ImageLimits)` / `decode_png_premultiplied` で、
  `ugoe-headless` と `ugoe-wasm` がこれを使います (T0009)。グレースケール・パレット・16 ビットは 8 ビットの RGBA に展開します。
  見出し (IHDR) の大きさだけで確保すると小さなファイルで数 GB を求められるので、一辺が `ImageLimits::max_side`
  (既定 16384。`ImageLimits::default().with_max_side(n)` で変えられる) を超える画像は、画素を確保する前に `ImageError::TooLarge` で止めます。

### 読み込み時の検査

`read_bundle` は入力を信用せず、次をすべてエラーとして返します (パニックしない)。

- 切り詰め、マジックの不一致、未対応のコンテナ `version`、予約フラグ
- チャンクの範囲外 (オフセット・長さのオーバーフローを含む)、16 バイト境界でないオフセット、ヘッダ・チャンク表・他のチャンクとの重なり。長さ 0 のチャンクはほかのチャンクと重なりませんが、オフセットの境界・チャンク表より後ろから始まること・ファイルの長さの検査は受けます
- `MODL` がない、`MODL` `PHYS` `META` `FEAT` が 2 つ以上ある、`MODU` がある (`UnexpectedChunk`。モジュールのファイルを取り違えたとき)
- 知らない必須の機能の宣言 (`UnsupportedFeatures`。ほかのチャンクをデコードする前に調べる)
- `TEXR` の数が `Model.textures` と一致しない、PNG / KTX2 でないテクスチャ
- `NRML` の数が `normal_map` を持つテクスチャの数と一致しない (`NormalMapCountMismatch`)、PNG / KTX2 でない法線マップ
- `MODL` の `schema_version` が新しすぎる (`UnsupportedVersion`。ほかのチャンクと `MODL` の残りをデコードする前に調べる。[バージョニング](#バージョニング))
- ペイロードの展開・デコードの失敗、読み込みの上限を超える展開 (`ReadLimitExceeded`。[読み込みの上限](#読み込みの上限))
- `ugoe_model::validate::validate` がエラーを返すモデル (下流の変形・描画で範囲外アクセスを起こさないため、ここで弾く)
- `PHYS` があり、`ugoe_model::validate::validate_physics` がエラーを返す物理の設定 (`InvalidPhysics`)
- `ugoe_model::validate::validate_motion` / `validate_expression` がエラーを返すモーション・表情 (`InvalidMotion`・`InvalidExpression`。最初に通らなかったもの。[model.md#モーションと表情の検証](model.md#モーションと表情の検証))

### API (`ugoe-format`)

- `Bundle { model, physics, motions, expressions, textures, normal_maps, meta }`。テクスチャは `TextureData { format: Png | Ktx2, bytes }`。
  法線マップ `normal_maps` は `NRML` チャンクの順で、テクスチャの番号からは `Bundle::normal_map(index)` で引きます。
- `write_bundle(&Bundle) -> Result<Vec<u8>, FormatError>` / `write_bundle_with(&Bundle, WriteOptions { compress })`
  - 使っている機能を `features::detect(&Bundle)` で自動で宣言し、宣言があれば `FEAT` を書きます。
  - 未知の値 (`Unknown`) を含むバンドルは `UnknownValues(場所の一覧)` で拒否します ([再保存](#再保存))。
- `read_bundle(&[u8]) -> Result<Bundle, FormatError>`
- `read_bundle_with(&[u8], ReadLimits) -> Result<(Bundle, ReadReport), FormatError>`: 読み込みの上限を指定して読みます ([読み込みの上限](#読み込みの上限))。ほかの読み込みの関数は `ReadLimits::default()` で読みます。
- `read_bundle_with_report(&[u8]) -> Result<(Bundle, ReadReport), FormatError>`: 警告も返します。
  `ReadReport { features, warnings }` の `features` は `FEAT` の宣言のまま、`warnings` は
  `ReadWarning::UnknownOptionalFeature(id)` (知らない任意の機能) と `ReadWarning::UnknownValue(場所)` (未知の値。場所は `MODL: meshes[Face].blend` や `MOTN#0: curves[1].target`) です。
- 機能の一覧は `features::FEATURES` (`Feature { id, description, fallback, baseline }`)。下の表と一致することをテストで検査します。
- 単体の JSON: `physics_from_json` / `physics_to_json`・`motion_from_json`・`expression_from_json` (`model_from_json` / `model_to_json` と並ぶ)。物理の参照先の検査は、組み合わせるモデルが決まってから呼び出し側で行います。モーションと表情は読み込みで数値を検査します (`InvalidMotion`・`InvalidExpression`)。
- ファイルシステムには触れません。読み書きはバイト列で行います。

- 単体の JSON: `module_to_json` / `module_from_json` (着せ替えのモジュール。`schema_version` だけを検査し、検証は呼び出し側で行う)。

> 未決: 読み込み時間を短くするため、`CompiledModel` をそのまま (ゼロコピーで) 読めるレイアウトにするか。まずは素直なシリアライズで実装し、計測してから判断する。

## モジュール (`.ugoemod`)

着せ替えのモジュール ([modules.md](modules.md)) を 1 つ入れるファイルです。**コンテナはバンドルと同じ** で
(マジック `UGOE`・コンテナの `version`・予約フラグ・チャンク表・16 バイト境界・圧縮)、`MODL` の代わりに `MODU` を持ちます。

| チャンク | 内容 |
| --- | --- |
| `MODU` | `Module` を MessagePack (フィールド名付き) にしたもの。1 つだけ。物理 (`Module::physics`) もこの中に入る |
| `TEXR` | テクスチャ 1 枚 (PNG または KTX2)。`Module.textures` と同じ順に並べる |
| `NRML` | 法線マップ 1 枚。`Module.textures` のうち `normal_map` を持つものと同じ数・同じ順に並べる |
| `META` | 作者やライセンスなどの自由なメタデータ (JSON)。0〜1 個 |
| `FEAT` | 使っている機能の宣言。0〜1 個 |

- 書き出しでは `FEAT` `MODU` `META` `TEXR`… `NRML`… の順にします。圧縮の扱いはバンドルと同じです (`TEXR` と `NRML` は圧縮しない)。
- バンドル用のチャンク `MODL` `PHYS` `MOTN` `EXPR` は置けません。あれば `UnexpectedChunk` で読み込みをエラーにします
  (バンドルを取り違えたとき、黙って読み飛ばさないため)。未知のタグのチャンクは読み飛ばします。
- 機能 `modules` は基準の機能なので宣言しません (下の [機能の宣言](#機能の宣言))。モジュールのファイルであることは `MODU` があることで分かります。
  モジュールの中で 1.0 以降の機能を使ったときは、バンドルと同じく `FEAT` で宣言します (`features::detect_module`)。

### 読み込み時の検査

`read_module` は、バンドルのコンテナの検査 (切り詰め・マジック・版・予約フラグ・範囲・境界・重なり・機能の宣言) に加えて、次をエラーとして返します (パニックしない)。

- `MODU` がない、`MODU` `META` `FEAT` が 2 つ以上ある、バンドル用のチャンクがある
- `TEXR` の数が `Module.textures` と一致しない、`NRML` の数が `normal_map` を持つテクスチャの数と一致しない、PNG / KTX2 でない画像
- `MODU` の `schema_version` が新しすぎる (`UnsupportedVersion`。バンドルと同じく、デコードする前に調べる)
- ペイロードの展開・デコードの失敗、読み込みの上限を超える展開 (バンドルと同じ `ReadLimitExceeded`)
- `ugoe_model::validate::validate_module` がエラーを返すモジュール (`InvalidModule`)。本体との整合 (スロット・`requires`・
  パラメータ・マスクの参照先) は、本体が決まってから合成 (`ugoe_runtime::compose`) が検査します

未知の値は警告として返し (場所は `MODU: meshes[Hair].blend`、物理は `MODU: physics.rigs[Sway].outputs[0].source`)、書き出しでは `UnknownValues` で拒否します。

### API (`ugoe-format`)

- `ModuleFile { module, textures, normal_maps, meta }`。法線マップはテクスチャの番号から `ModuleFile::normal_map(index)` で引きます。
- `write_module(&ModuleFile)` / `write_module_with(&ModuleFile, WriteOptions)`
- `read_module(&[u8])` / `read_module_with_report(&[u8])` (報告の形はバンドルと同じ `ReadReport`) / `read_module_with(&[u8], ReadLimits)` (上限を指定する)

## 機能の宣言と段階的な劣化

判断の経緯は [ADR 0013](../adr/0013-forward-compatibility.md)。

### 機能の宣言

バンドルは `FEAT` チャンクで、使っている機能を宣言します。ペイロードは次の構造体の MessagePack (フィールド名付き) です。

```
FeatureTable {
  features: [            省略時は空
    { id: "lighting.foo", required: false },   required は省略時 false
    { id: "physics.bar",  required: true },
    ...
  ]
}
```

- 基準の機能 (1.0 の時点の機能) は宣言しません。書き出しは `features::detect` で使っている機能を調べて自動で宣言します (1.0 の時点では常に空で、`FEAT` は書かれません)。
- 必須の機能を宣言するとき、書き出すツールは利用者に警告します (`detect` の結果の `required` を見る)。
- 同じ ID を必須と任意の両方で宣言したときは、必須として扱います。
- `test.` で始まる機能 ID は試験用に予約されていて、本物の機能には使いません ([testing.md#互換性テスト](../testing.md#互換性テスト))。
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
| `deform.blend_shape` | ブレンドシェイプ格子 ([model.md#ブレンドシェイプ](model.md#ブレンドシェイプ)) | ブレンドシェイプ格子を無視する (ブレンドシェイプのパラメータが既定値のときの形になる) |
| `deform.smooth_interpolation` | キーフォームの曲線補間 ([evaluation.md#曲線補間](evaluation.md#曲線補間)) | 線形で補間する (キーの上の形は同じで、キーの間の道筋が直線になる) |
| `physics.additive_output` | 物理の出力の加算 ([physics.md#合成の方法](physics.md#合成の方法)) | 上書きとして扱う (トラッキングなどの値が物理の値に置き換わる) |

上の表はすべて 1.0 の基準の機能です。

#### 1.0 以降の機能

| 機能 ID | 内容 | フォールバック |
| --- | --- | --- |
| (まだない) | | |

### 未知の値の読み込み

- 構造体の未知のフィールドは無視します。
- 拡張されうる enum は、未知のタグを `Unknown` として読みます (JSON と MessagePack のどちらでも。タグに付いた未知の中身は読み捨てます)。`Unknown` の要素には次のフォールバックを適用します。

| 値 | フォールバック | 適用する場所 |
| --- | --- | --- |
| `Mesh::blend` (`BlendMode`) | 通常の合成で描く | ランタイム (`DrawableState::blend`)・レンダラ |
| `Mesh::mask_mode` (`MaskMode`) | 和集合として扱う | ランタイム (`CompiledModel::mask_mode`)・レンダラ |
| `Deformer::kind` (`DeformerKind`) | 恒等変換 (子は親の変換だけを受ける。不透明度は 1) | ランタイム |
| `Parameter::kind` (`ParameterKind`) | 通常のパラメータとして扱う。格子の種類は置き場所 (`blend_shapes`) で決まるので評価は変わらず、検証はどちらの格子の軸にも置けるものとする | 検証・エディタ |
| `KeyformGrid::origins` の要素 (`KeyformOrigin`) | `authored` として扱う (推定で上書きしない) | エディタ |
| `KeyformGrid::inference` (`KeyformInference`) | 推定で作り直さない (`derived` のセルをそのまま使う) | エディタ |
| `KeyformGrid::interpolation` (`KeyformInterpolation`) | 線形で補間する | ランタイム・エディタ |
| `Curve::target` (`CurveTarget`) | そのカーブを無視する | モーション |
| `Curve::segments` の要素 (`Segment`) | そのカーブ全体を無視する | モーション |
| `ExpressionParameter::op` (`ExpressionOp`) | その項目を無視する | 表情 |
| `PhysicsInput::source` / `PhysicsOutput::source` (`PhysicsSource`) | その入力・出力を無視する | 物理 |
| `AvatarMeta` の許諾 (`AvatarPermission` `CommercialUsage` `CreditNotation` `Modification`) | 最も制限の強い値 (`only_author`・`personal_non_profit`・`required`・`prohibited`) として扱う (`effective()`) | アプリ |
| `TrackingOverride::source` (`TrackingSource`) | その上書きを無視する | トラッキング |
| `PhysicsOutput::target` (`PhysicsTarget`) | その出力を無視する | 物理 |
| `PhysicsOutput::blend` (`PhysicsBlend`) | `overwrite` として扱う (揺れを足さずに値を置き換える) | 物理・エディタ |
| `PhysicsCollider::shape` (`ColliderShape`) | その衝突体を無視する | 物理 |

- 未知の値を読んでも検証エラーにはしません。`read_bundle_with_report` が警告として場所を返します。

### 再保存

読み込んだバンドルを書き出し直すときの扱いです。

- **未知の値 (`Unknown`) は書き出せません。** 元のタグと中身を失っているので、書き出しは `UnknownValues` エラーになります (`model_to_json` と、`ugoe-model` の型の serde でのシリアライズも同じくエラーになる)。別の値 (たとえばフォールバックの値) として黙って保存しないためです。保存したいツールは、利用者に確認してから値を置き換えます。
- **知らない任意の機能** のデータは読み込みの時点で失われている (未知のフィールドは無視する) ので、書き出したファイルはフォールバックを適用した、その機能を使わないファイルになります。宣言は `detect` が作り直すので、知らない機能を宣言し直すことはありません。
- 互換性のテストのため、各リリースで書き出したファイルを `assets/compat/` に保存し、新しいランタイムで読めること、新しい機能を含むファイルを古い読み込みルールで劣化させて読めることを CI で検査します (T0115)。

### モジュールと法線マップのファイル

- 着せ替えのモジュールは、ヘッダが同じ形式のファイル (拡張子は `.ugoemod`) で、`MODL` の代わりに `MODU` チャンクを持ちます ([モジュール](#モジュール-ugoemod))。ヘッダの予約フラグは使いません。
- 法線マップは `TEXR` チャンクと同じ形式の `NRML` チャンクに入れ、`TextureRef::normal_map` を持つテクスチャの順に並べます ([バンドル](#バンドル-ugoe))。
  複数のテクスチャが同じ法線マップを名指すときも `NRML` はテクスチャごとに 1 つずつ置き、各テクスチャは自分の `NRML` を使います。
  書き出しでは、同じ法線マップを名指すテクスチャの `NRML` に同じ画像を入れるのがよい (SHOULD) とします。
  `lighting.normal_map` は基準の機能なので宣言しません。

## バージョニング

- `ugoe_model::SCHEMA_VERSION` はデータのスキーマのバージョンです。互換性を壊す変更をしたら 1 上げます。
- 読み込み時は `schema_version` を必ず検査します。
  - 現在のバージョンより新しい → `UnsupportedVersion` エラー
  - `.ugoe` / `.ugoemod` では、機能の宣言 (`FEAT`) を調べた後、どのペイロードもデコードする前に、`MODL` / `MODU` の
    MessagePack のマップから `schema_version` の値だけを読みます (ほかのキーの値は型に当てはめずに読み飛ばす。入れ子の深さの上限は同じ 128)。
    形の変わった新しい版のファイルも、デコードのエラーではなく `UnsupportedVersion` になります。
    展開できない・マップでない・`schema_version` が `u32` として読めないなどで先読みできなければ、ここではエラーにせず、
    チャンクを順にデコードするところで今までどおりのエラーになります (全体をデコードした後にも `schema_version` を検査します)。
  - JSON の `model_from_json` / `module_from_json` も同じく、全体をデコードする前に `schema_version` の値だけを読みます
    (先読みできなければ全体のデコードのエラーになります)。
  - 現在のバージョンより古い → マイグレーションを順に適用して現在のバージョンにする (`ugoe-format` の `migrate` モジュール。マイグレーションが必要になったときに作る)
- 1.0 より前 (M4 まで) は互換性を保証しません。`SCHEMA_VERSION` は 0 のままにし、形式を壊してよいものとします。
- 1.0 以降は、機能の追加を上の「機能の宣言」で行い、`SCHEMA_VERSION` は互換性を壊す変更 (必須の機能でも表現できない変更) のときだけ上げます。
