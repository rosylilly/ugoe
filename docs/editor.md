# エディタ設計

コードは `apps/editor/` (Tauri アプリ) と `crates/ugoe-editor-core/` (ヘッドレスな中核)。

## 構成

```
┌────────────── Tauri WebView (Svelte 5 + TS) ───────────────┐
│  UI パネル (パーツツリー / パラメータ / インスペクタ / タイムライン)  │
│  ビューポート <canvas> ── ugoe-wasm (ランタイム + wgpu レンダラ)   │
│         ▲ .ugoe のバイト列 / 変更の差分    │ invoke(Command)      │
└─────────┼──────────────────────────────┼─────────────────────┘
          │                              ▼
┌─────────┴──────── Tauri バックエンド (src-tauri) ───────────────┐
│  薄い IPC 層 (commands.rs)                                       │
│  ugoe-editor-core::Document  ← 正となる状態 (source of truth)    │
│  ファイル I/O / PSD 取り込み / 自動メッシュ / AI 呼び出し            │
└────────────────────────────────────────────────────────────────┘
```

### 責務の分け方

| 場所 | 持つもの | 持たないもの |
| --- | --- | --- |
| `ugoe-editor-core` | ドキュメントの状態、Command の適用と検証、Undo/Redo、選択状態、プロジェクトの読み書き | UI、Tauri、GPU |
| `src-tauri` | IPC コマンドの定義、ファイルダイアログ、ウィンドウ管理、非同期ジョブ (PSD・AI) の起動 | 編集ロジック |
| フロントエンド | 表示、入力、ギズモ、ビューポートの描画、パラメータスライダーのプレビュー | モデルの正となる状態 |

**ルール: 編集ロジックは必ず `ugoe-editor-core` に書き、ユニットテストします。** Tauri 側とフロントエンド側は、ユーザーの入力を `Command` に変換して送るだけにします。

## コマンド

- `Command` は serde でシリアライズできる enum です (`#[serde(tag = "op")]`)。JSON の例: `{"op":"set_parameter_range","id":"AngleX","min":-30,"default":0,"max":30}`
- **1 コマンド = ユーザーから見た 1 操作 = Undo の 1 単位** です。ドラッグ操作のように途中経過があるものは、フロントエンドでプレビューだけを行い、確定時に 1 つのコマンドを送ります。
- `Batch` は複数のコマンドを 1 つの Undo 単位にまとめます (AI の提案など)。
- `Document::apply` は、コマンドの検証に失敗した場合にモデルを一切変更しません。
- 対象は必ず ID で指定します。配列のインデックスは使いません (同時編集や AI の提案が古くなったときに、別の要素を壊さないようにするため)。
- `Command` の JSON Schema (AI のツール定義用) と TypeScript の型は Rust から生成します (下の「型の生成」)。
- **コマンドを追加する手順:** `Command` の末尾にバリアントを足し、`Document::apply` とテストを書き、`codegen.rs` の `sample_commands` に例を 1 つ足して `just gen` を実行します。フロントエンドの `Command` 型には自動で現れます。
- `Document::apply` は適用後のモデルを `ugoe_model::validate::validate` で検査し、**適用前になかった検証エラーが生じたら拒否します** (`EditError::Invalid` に新しいエラーの一覧が入る)。検証を通るモデルは、どのコマンドを適用した後も検証を通ります。読み込んだ時点で不正なモデルでも、エラーを増やさない編集 (エラーを直す編集を含む) はできます。`Batch` は全体を適用した後に 1 回だけ検査するので、途中の状態は不正でもかまいません。
- 新しい要素の ID は `ugoe_editor_core::ids` (`new_part_id` など) で作ります。`[A-Za-z0-9_]` 以外の文字を `_` に直し、既存の ID と重なれば `_2`、`_3`、… を付けます。

### コマンド一覧

`op` の値と内容です。要素の追加は完全な値 (`Parameter`・`Part`・`Deformer`・`Mesh`) を渡します。「名前の変更」は表示名 (`name`) だけを変え、ID は変えません。

| 対象 | `op` |
| --- | --- |
| パラメータ | `add_parameter`、`remove_parameter`、`set_parameter_range`、`rename_parameter`、`set_parameter_group` (`null` で外す)、`set_parameter_mirror` (`null` で外す。相手側は変えない。自分自身と対にすると値の符号を反転して対応させる) |
| パーツ | `add_part`、`remove_part`、`rename_part`、`set_part_parent`、`set_part_keyforms` |
| デフォーマ | `add_deformer`、`remove_deformer`、`rename_deformer`、`set_deformer_parent`、`set_deformer_part`、`set_warp_keyforms` (`rows` / `cols` は変えない)、`set_rotation_keyforms`、`create_warp_deformer`・`create_rotation_deformer`・`reparent_preserving_look` ([デフォーマの編集](#デフォーマの編集)) |
| メッシュ | `add_mesh` (`meshes` の末尾に追加)、`remove_mesh`、`rename_mesh`、`set_mesh_properties`、`set_mesh_parent`、`set_mesh_keyforms`、`set_mesh_keyform`、`reorder_mesh`、`regenerate_mesh` ([メッシュの再生成](#メッシュの再生成))、`move_mesh_vertices`・`add_mesh_vertex`・`remove_mesh_vertices`・`retriangulate_mesh` ([メッシュの編集](#メッシュの編集)) |
| キーフォーム | `add_keyform_key`、`remove_keyform_key`、`set_keyform_cell`、`set_mesh_draw_order` ([キーフォームの編集](#キーフォームの編集)) |
| テクスチャ | `add_texture`、`set_texture_image`、`remove_texture` |
| キャンバス | `set_canvas` (`width`・`height`・`origin`。大きさは 0 より大きい有限の値。要素の座標は変えない) |
| キーフォームの推定 | `infer_keyforms`、`mirror_keyforms`、`mark_keyform_authored`、`mark_keyform_derived` (下の「推定とミラー」) |
| その他 | `batch` |

- `set_mesh_properties` は `part`・`texture`・`blend`・`culling`・`masks`・`mask_mode`・`mask_only` のうち、指定したフィールドだけを変えます。
- `set_mesh_keyform` は格子の 1 セルを置き換え、そのセルの由来を `authored` にします。セルは「軸のパラメータ ID → キーの値」の対応で指定します (例: `"cell": {"AngleX": 30, "AngleY": 0}`)。格子のすべての軸を指定し、値はキーと完全に一致させます。軸のない格子は `{}` です。
- `reorder_mesh` は `meshes` の中でメッシュを `before` の直前 (`null` なら末尾) に移します。描画順 (`draw_order`) が同じメッシュの前後関係が変わります。
- `set_deformer_parent` と `set_mesh_parent` は親を付け替えるだけで、座標は変えません (子の座標は新しい親の局所座標として解釈されるので、見た目は変わりうる)。見た目を保つ付け替えは `reparent_preserving_look` です ([デフォーマの編集](#デフォーマの編集))。

- `add_texture` は `name` と `png` (PNG ファイルのバイト列) を受け取り、`Model::textures` の末尾に追加します。幅と高さは PNG の IHDR から読みます。名前の規則は下の「テクスチャ」を参照。`set_texture_image` は画像を差し替え、寸法も PNG に合わせます。`remove_texture` は使っているメッシュがあれば拒否し (`HasChildren`)、後ろのテクスチャを指すメッシュの `texture` (インデックス) を 1 つ詰めます。

### 推定とミラー

キーフォームの推定 ([spec/keyform-inference.md](spec/keyform-inference.md)) のコマンドです。アルゴリズムは `ugoe-rigging` にあります。
格子を持つ要素は `target` で `{"kind": "mesh", "id": "EyeL"}` のように指定します (`kind` は `part`・`mesh`・`deformer`・`glue`)。

| `op` | 内容 |
| --- | --- |
| `infer_keyforms` | `target` の格子の `derived` のセルを作り直す。`method` は `from_grid` (既定。格子の `inference` に従い、省略なら加算合成)・`additive`・`depth_rotation` (格子の `inference` が `depth_rotation` であること)。書き換えるのはその方法で作ったセルだけで、`overwrite_derived: true` なら別の方法 (ミラーなど) で作ったセルも書き換える |
| `mirror_keyforms` | `source` を反転して `target` のセルを作る (由来は `mirror`)。先の `authored` 以外のセルをすべて書き換え、`source` を先の格子の `mirror_source` に記録する。`axis` (対称軸)・`tolerance` (頂点の許容距離) は省略できる (指定すれば記録する)。`overwrite_authored: true` で先の `authored` のセルも上書きする |
| `mark_keyform_authored` | `cells` (セルの指定の配列。形は `set_mesh_keyform` の `cell` と同じ) の由来を `authored` にする。値は変えない |
| `mark_keyform_derived` | `cells` の由来を `derived` にし、格子の推定の設定で作り直す。基準のセルは `derived` にできない (推定のエラーで拒否) |

- 推定のエラー (中立のキーがない、基準のセルが `derived`、ミラーの対でない、など) は `EditError::Inference` で拒否し、モデルは変えません。
- **自動の作り直し:** 格子のセル (値や由来) を変えたコマンドの後、`Document::apply` はその格子の `derived` のセルを **セルに記録した推定の方法で** 作り直し、その格子をミラー元 (`mirror_source`) にする格子のミラーのセルもミラーし直します。同じ Undo 単位に含まれ、`batch` では全体を適用した後にまとめて行います。加算合成・深度からの回転で作り直せなければコマンドを拒否します (ミラーのやり直しの失敗は、ミラーのセルをそのままにして続ける)。規則は [spec/keyform-inference.md#自動の作り直し](spec/keyform-inference.md#自動の作り直し)。
  - 対象は `set_*_keyforms`・`set_mesh_keyform`・`set_keyform_cell`・`mark_keyform_*`・`infer_keyforms`・`mirror_keyforms` です。新しいコマンドは、格子のセルを変えたら `apply_to` の `touched` に要素を積むことで対象になります (`crates/ugoe-editor-core/src/document.rs`)。
  - 推定のコマンドも積みます。作り直しはセルごとの方法で行うので推定の結果を別の方法で上書きせず、その格子をミラー元にする格子へ変化を伝えます。
  - パラメータの範囲の変更など、セルを変えないコマンドでは作り直しません。必要なら `infer_keyforms` を実行します。
- **`derived` のセルの編集:** セルを書き換えるコマンドは、書き換えた `derived` のセルを `authored` にします。`set_*_keyforms` で格子を丸ごと置き換えるときは、軸が同じなら、値が変わったのに `derived` (方法を問わない) と指定されたセルを `authored` にします。
- ミラーで作ったセル (`mirror`) は、ミラー元の格子が変わるとミラーし直され、先の格子を編集しても上書きされません。方法を記録していない版 (T0614 より前) のファイルでミラーしたセルは `derived` のままなので、以前と同じく格子の方法で作り直されます (`mirror_keyforms` を再度実行すると `mirror` になる)。

### 削除の連鎖と拒否

| コマンド | 他の要素が参照しているとき |
| --- | --- |
| `remove_parameter` | **連鎖して直す。** このパラメータを軸に持つキーフォーム格子 (パーツ・デフォーマ・メッシュ・グルー) から軸を取り除き、パラメータの `default` と等しいキーの断面を残す。等しいキーがなければ最も近いキー (等距離なら小さい方) の断面を残す。セルの由来 (`origins`) も同じセルを残す。深度からの回転の `yaw` / `pitch` がこのパラメータなら外す (回転の中心などの設定は残す)。他のパラメータの `mirror` は `null` にし、エフェクト (目パチ・呼吸・リップシンク) の対象からは取り除く |
| `remove_part` | 子パーツ・所属するデフォーマ・所属するメッシュが残っていれば **拒否する** (`EditError::HasChildren` に子の一覧が入る)。ポーズからは取り除く: そのパーツの要素を消し、他の要素の `links` からも外し、空になったグループは消す |
| `remove_deformer` | 子デフォーマ・子メッシュが残っていれば **拒否する**。他のデフォーマの `mirror` からは外す |
| `remove_mesh` | **連鎖して直す。** 他のメッシュの `masks` と `mirror` から外し、このメッシュを使うグルーと当たり判定を削除する |

- 削除 (`remove_part`・`remove_deformer`・`remove_mesh`) で要素がなくなったら、その要素をミラー元 (`keyforms.mirror_source`) にしている格子から記録を外し、その格子の `mirror` のセルを `authored` にします (値は変えない)。

- 拒否する場合は、先に子を削除するか付け替えます。まとめて行うなら `batch` を使います。
- 物理・モーション・表情は `Model` の外のデータなので、`remove_parameter` はこれらを書き換えません。消したパラメータへの参照が残ることがあります。

### Undo

最初はスナップショット方式 (`Model` を丸ごと clone) で実装します。簡単で確実だからです。
モデルが大きくなってメモリや時間が問題になったら、各コマンドが逆操作を返す方式に移行します ([ADR 0005](adr/0005-editor-command-model.md))。

## PSD の取り込み

`ugoe_editor_core::import::import_psd(model, bytes, options)` が、PSD (`ugoe-psd` の `LayerTree`) から
**1 つの `batch` コマンド** (Undo の 1 単位) と警告の一覧を作ります。モデルは書き換えず、コマンドを返すだけです
(エディタ・CLI・AI が同じ関数を使う)。読み込み済みの `LayerTree` からは `layer_tree_command` で作れます。

| PSD | ugoe |
| --- | --- |
| ドキュメント全体 | 最上位のパーツを 1 つ作り (名前は `options.name`。エディタと CLI はファイル名)、すべてをその下に入れる |
| キャンバス | モデルが空 (パーツ・メッシュ・デフォーマ・テクスチャがない) なら `set_canvas` で PSD の大きさにし、原点は中央。空でなければ変えず、大きさが違えば警告する。どちらでもレイヤーは PSD のピクセル座標に置く |
| グループ | パーツ (階層を保つ)。不透明度をパーツのキーフォームの不透明度にする。非表示なら不透明度 0。通過 (`pass`) と通常 (`norm`) 以外のブレンドモードは警告して無視する |
| ラスタレイヤー | テクスチャ 1 枚とメッシュ 1 つ。テクスチャはレイヤーの画像 (外接矩形で切り出したもの) の周りに `ceil(margin) + 1` px の透明な余白を足した PNG。メッシュは余白付きの画像から `ugoe-mesher` で作り、UV はそのテクスチャ上、頂点位置は「レイヤーの位置 − 余白 + メッシュの座標」(キャンバスのピクセル座標)。キーフォームは 1 つ (パラメータなし) |
| 不透明度 | メッシュの不透明度 = レイヤーの不透明度 × 塗りの不透明度 |
| 非表示のレイヤー | **取り込み、不透明度を 0 にする** (差分の絵 (閉じ目など) を後で使えるように)。元の不透明度は残らない |
| 描画順 | ドキュメント全体で下から上へ、`draw_order = 1000 × (i + 1) / (n + 1)` (`i` は下から数えた番号、`n` はメッシュの数)。`meshes` にも下から上の順に並べる。既存のモデルに取り込むと、既存のメッシュと描画順が混ざるので、必要なら後で並べ直す |
| ブレンドモード | `norm` → `normal`、`mul` → `multiply`、`lddg` (覆い焼き (リニア)) / `add` → `additive`。ほかは `normal` にして警告する |
| クリッピング | クリッピングされたレイヤーは、同じ並びの中で下にある最も近いクリッピングでない要素 (土台) を通常マスク (`MaskRef { invert: false }`) にする。土台がグループなら、その中のすべてのメッシュ (和集合)。土台のレイヤーの不透明度 (非表示なら 0) をクリッピングされた側の不透明度に掛ける (Photoshop では土台の不透明度が掛かるため)。下に何もなければ無視し (Photoshop と同じ)、土台を取り込まなかった (空のレイヤーなど) なら警告してマスクなしにする。グループのクリッピングは警告して無視する |
| 空のレイヤー・完全に透明なレイヤー・頂点が多すぎるレイヤー | 取り込まずに警告する |
| 調整レイヤー・レイヤー効果など | `ugoe-psd` の警告をそのまま返す ([ugoe-psd](../crates/ugoe-psd/README.md#警告-layertreewarnings)) |

- ID は名前から `ids` の規則で作ります。名前が英数字を 1 つも含まなければ (日本語だけの名前など)、PSD のレイヤー ID から `Layer12` / `Group8` (ID がなければ `Layer` / `Group`) にします。表示名 (`name`) には元の名前を残します。
- テクスチャ名はメッシュの ID と同じにし、大文字小文字を区別せずに重なれば `_2`、… を付けます。
- 自動メッシュの設定 (`AutoMeshOptions`、JSON で省略したフィールドは既定値) は `ugoe_mesher::MeshOptions` と同じです。不正な値なら取り込みをエラーにします。

### メッシュの再生成

`regenerate_mesh { id, options }` は、メッシュのテクスチャの画像から自動メッシュで作り直します。

- すべてのキーフォームの頂点位置が同じ (キーフォームが 1 つの場合を含む) メッシュだけが対象です。キーフォームごとに頂点が動いていると、頂点が変わった後に変形を引き継げないので拒否します (`EditError::CannotRegenerateMesh`)。頂点番号を参照するグルーがあるメッシュ、頂点ごとの深度 (`depth`) を持つメッシュも拒否します。
- 頂点位置は UV のアフィン写像になっていなければなりません (取り込んだメッシュは「テクスチャの左上 + UV × 寸法」)。現在の UV と頂点位置からこの写像を最小二乗で求め (ずれが位置の広がりの 1e-4 倍を超えれば拒否)、新しい頂点の位置を決めます。親デフォーマの局所座標でも同じです。
- 各キーフォームの頂点位置以外の値 (不透明度・描画順・色など) と格子の軸は保ちます。
- `margin` を取り込み時より大きくすると、輪郭がテクスチャの余白の外 (UV が 0〜1 の外) に出ることがあります。

### メッシュの編集

メッシュの **トポロジーと基準の形** を編集するコマンドです (実装は `crates/ugoe-editor-core/src/mesh_edit.rs`)。キーフォームごとの形の編集 (パラメータのキーでの変形) は T0608 で扱います。

どのコマンドも、**既定値のパラメータで評価したメッシュ** (以下「基準の姿」。グルー適用後のキャンバス座標で、ビューポートに既定値で表示される形) を基準にします。評価はランタイム (`ugoe-runtime`) そのもので行います (モデルの複製に、評価用のメッシュを足して評価する)。モデルが評価できない (検証に通らない) ときは `EditError::MeshGeometry` で拒否します。

| `op` | 内容 |
| --- | --- |
| `move_mesh_vertices { id, moves: [{ index, delta }] }` | 頂点をキャンバス座標のずれ `delta` だけ動かす。同じ頂点を 2 回指定するとエラー |
| `add_mesh_vertex { id, position }` | キャンバス座標 `position` に頂点を 1 つ足し (頂点の末尾)、三角形を作り直す |
| `remove_mesh_vertices { id, indices }` | 頂点を消し (後ろの番号は詰める)、三角形を作り直す。3 頂点未満になるなら拒否 (`EditError::TooFewVertices`) |
| `retriangulate_mesh { id }` | 今の輪郭を保ったまま三角形を作り直す |

移動・追加・削除はすべてのセルの頂点位置を変えるので、推定で作ったセル (`derived`) は適用の後に作り直します (`authored` のセルは変えない。[keyform-inference.md](spec/keyform-inference.md))。

範囲外の頂点番号は `EditError::VertexOutOfRange`、形の上でできない編集 (退化した親・同じ UV の頂点・三角形が作れないなど) は `EditError::MeshGeometry` です。

**頂点の移動** (`move_mesh_vertices`)。頂点 `i` の基準の姿での局所座標を `l`、親デフォーマのワールド変換 (局所 → キャンバス、[evaluation.md](spec/evaluation.md#3-デフォーマのワールド変換)) を `F` とします。

```
J       = ∂F/∂l (l での 2×2 のヤコビ行列。中心差分で求める)
local   = J⁻¹ · delta                 // すべてのキーフォームの positions[i] に同じ量を足す
uv[i]   = 基準の姿のテクスチャの対応で、キャンバスの点 world[i] + delta に当たる UV
```

- 親なしは `J = I`。親が回転デフォーマ (と祖先の回転) ならワールド変換はアフィンなので、`J` は厳密にその線形部分 (回転・拡大率・反転) です。差分の幅は 1px (アフィンなら中心差分は幅によらず厳密)。
- 親がワープなら、`J` はその点でのセル単位の双線形写像の微分です (格子の外では外挿のアフィン近似の微分)。差分の幅は UV で 1e-3。双線形は一次式ではないので、大きく動かすと、評価後の位置は `delta` から交差項 (`∂²F/∂u∂v · Δu · Δv`) の分だけずれます。
- `J` の行列式がほぼ 0 (潰れた親) なら拒否します。
- すべてのキーフォームに同じ局所のずれを足すので、キーフォーム間の差 (動き) は保たれます。
- **UV:** 基準の姿の三角形 (キャンバス座標) のうち、移動先の点を含むもの (なければ最も近いもの) の重心座標で UV を補間します (外側では外挿)。テクスチャはキャンバス上で動かず、メッシュだけがその上を動きます。
- 三角形は作り直しません (必要なら `retriangulate_mesh`)。深度 (`depth`) は変えません。

**頂点の追加** (`add_mesh_vertex`)。基準の姿で `position` を含む三角形 (辺の上を含む) を探し、その重心座標 `(w0, w1, w2)` で次の値を補間します。

- UV、**各キーフォームの** `positions` (同じ重心座標を各キーフォームの三角形に当てる)、深度 (`depth` があれば)。
- `position` がどの三角形にも入らなければ、最も近い **境界辺** を選び、その辺を持つ三角形の重心座標で外挿します。その境界辺は新しい頂点を通るように 2 本に分けます (輪郭が外に広がる)。境界辺の上の点も同じく輪郭に入れます。内側の点は内部の頂点になります。
- 同じ UV の頂点がすでにあれば拒否します。

**頂点の削除** (`remove_mesh_vertices`)。輪郭 (境界辺) を保つため、境界辺をたどって消した頂点を飛ばし、残る前後の頂点を直接つなぎます (輪郭上の頂点を消すと、そこは前後を結ぶ直線になる)。内部の頂点を消した穴は埋まります。

- 各キーフォームの `positions`、`uvs`、`depth` から消した頂点を除きます。
- **グルー:** このメッシュを使うグルーのペアのうち、消した頂点を使うものは削除し、残りは新しい番号に付け替えます。マスクと当たり判定はメッシュ単位の参照なので変わりません。
- ミラー (`mirror`) の相手は変えません (頂点の対応は基準の位置から作るので、形が左右で変わると推定のときにエラーになりうる)。

**三角形の作り直し** (`retriangulate_mesh`、および追加・削除の後)。`ugoe_mesher::triangulate_points` で、**UV** の点を制約付きドロネー三角形分割します。

- 制約辺は今の輪郭 (1 つの三角形だけが使う辺)。重心が制約辺の内側 (偶奇規則) にある三角形だけを残すので、凹んだ輪郭と穴は保たれます。三角形がなければ (初めから空なら) 凸包全体を分割します。
- 三角形の向きは元のメッシュに合わせます (UV で見た向きの合計が時計回りなら時計回り)。カリングの表裏が変わらないようにするためです。
- どの三角形にも使われない頂点は残ります (輪郭の外にある頂点など)。

## キーフォームの編集

Live2D のリギング作業の中心: パラメータを作り、要素にキーを追加し、各キー (セル) の上で形を編集します。

### コマンド

要素は `target` で種類と ID を指定します: `{"kind": "part" | "deformer" | "mesh" | "glue", "id": "..."}` (`ugoe_editor_core::KeyformTarget`)。デフォーマはワープ・回転のどちらでもよく、格子の種類はデフォーマから決まります。

| `op` | 内容 |
| --- | --- |
| `add_keyform_key { target, parameter, key }` | 格子の `parameter` の軸にキー `key` を加える。軸がなければキー 1 つの軸を末尾に加える (セルの並びは変わらない)。軸があれば、新しいセルを既存の格子のその値での補間 ([evaluation.md](spec/evaluation.md#1-キーフォームの重み) と同じ重み。キーの範囲の外は端のセルの値) で埋めるので、**評価の結果 (見た目) は変わらない**。真偽値のフィールド (`reflect_x` など) は重みの大きい側 (同じなら小さいキーの側) の値。新しいセルの由来は、格子に推定で作ったセル (`derived`・`additive`・`depth_rotation`・`mirror`) があれば `derived` (方法の記録なし。その格子のセルを次に編集したときに格子の `inference` の方法で作り直される)、なければ `authored`。同じキーがすでにある (`DuplicateKey`)、有限でない、パラメータがない場合は拒否する |
| `remove_keyform_key { target, parameter, key }` | 軸からキーとその断面のセル (と由来) を取り除く。軸の最後のキーなら軸ごと取り除き、そのキーの断面が残る (`remove_parameter` と同じ畳み方) |
| `set_keyform_cell { target, cell, keyform }` | 1 セルを置き換え、そのセルを `authored` にする。`cell` は `set_mesh_keyform` と同じ「軸のパラメータ ID → キーの値」。`keyform` は `{"kind": "mesh" \| "warp" \| "rotation" \| "part" \| "glue", ...フィールド}` で、種類は格子と一致させる (`KeyformKindMismatch`)。頂点 (制御点) の数は変えられない (`ShapeMismatch`) |
| `set_mesh_draw_order { id, draw_order }` | メッシュの格子の **すべてのセル** の `draw_order` を同じ値にする (有限の値でなければ `InvalidDrawOrder`)。セルの由来と他の値は変えず、すべて同じ値なので作り直しもしない。描画順の違うメッシュどうしの前後を入れ替えるのに使う (`reorder_mesh` は描画順が同じメッシュの間でしか効かない)。セルごとに変えるなら `set_keyform_cell` |

`set_keyform_cell` の後は、その格子の `derived` のセルを同じ Undo の単位の中で作り直します ([推定とミラー](#推定とミラー))。`add_keyform_key` と `remove_keyform_key` は作り直しません (追加は補間で見た目を保ち、削除は残ったセルの値を変えないため)。

### 座標の変換

キーフォームの頂点・制御点・原点は親の座標系 (親なしならキャンバス座標、ワープの子は UV 空間、回転デフォーマの子は原点を (0,0) とした座標) で表されます。ビューポートでの操作 (キャンバス座標) を書き込むには、`ugoe_editor_core::space` で変換します。

- `local_to_canvas(model, target, values, points)`: 親の座標系 → キャンバス座標。親の祖先のデフォーマだけを残したモデルに、点を頂点とする仮のメッシュを置いてランタイム (`ugoe-runtime`) で評価するので、変換はランタイムの評価と必ず一致します。
- `canvas_to_local(model, target, values, points, guesses)`: 逆変換。差分で作ったヤコビアンでニュートン法を回します (初期値はドラッグ前の位置)。アフィンな親 (回転デフォーマ) なら 1 回で、ワープでも数回で収束します。親の変換が潰れていれば `Singular`、収束しなければ `NotConverged` を返します。
- `values` はビューポートの表示中のパラメータ値です (パラメータ値はドキュメントの状態ではないので、フロントエンドが渡す)。
- Tauri コマンドは `local_to_canvas(target, parameters, points)` と `canvas_to_local(target, parameters, points, guesses)` (`src-tauri/src/space.rs`)。点は `[x, y]` の配列です。

### 画面での操作

| 場所 | 操作 |
| --- | --- |
| パラメータ一覧 (`ParametersPanel.svelte`) | 「+」でパラメータを作る (ID・名前・最小・既定・最大)。名前を押すと名前・値域・既定値の編集と削除 (`rename_parameter` と `set_parameter_range` を `batch` で 1 つの Undo に)。スライダーの下に、主な選択の要素のキーを ◆ で示し (押すとそのキーへ)、「Snap to keys」で要素のすべての軸を最も近いキーへ動かす。行の ◆ ボタンで、今の値にキーを追加 (今の値がキーならそのキーを削除)。スライダーは表示の状態なので IPC を通らない |
| インスペクタ (`inspector/keyforms/KeyformInspector.svelte`) | 主な選択の要素の軸とキー (× で削除、押すとそのキーへ移動、「+ 値」で今の値にキーを追加)、軸のないパラメータへの軸の追加、セルの格子 (1 軸は 1 行、2 軸は表、3 軸目以降は今の値に最も近いキーの断面)。メッシュでは描画順を表示し、「Set for all cells」で `set_mesh_draw_order` を送る。セルは `A` (authored) / `D` (推定: `derived`・`additive`・`depth_rotation`、破線) / `M` (`mirror`、破線) で由来を示し、今の値がすべての軸でキーの上にあればそのセルを枠で示す。キーの上になければ「Move to nearest key」を出す |
| ビューポート (`viewport/tools/form/FormTool.svelte`) | 「Form edit」で切り替える。主な選択の要素のうち、メッシュは頂点、ワープは制御点、回転デフォーマは原点をハンドルで示す (主な選択がデフォーマのときは、代わりに [デフォーマのギズモ](#デフォーマの編集) を出す)。ドラッグ中はオーバーレイだけでプレビューし、離したときに `canvas_to_local` で親の座標系に直してから `set_keyform_cell` を 1 つ送る (Undo の 1 単位)。Shift を押しながらドラッグするとすべての点を同じだけ動かす |

- **キーの上でだけ編集できる:** 今のパラメータ値がどれかの軸でキーの上にないと、どのセルに書くか決まらないので編集を拒否し (ハンドルを出さない)、「Move to nearest key」を出します。
- キーの上かどうかは、パラメータの値域の 1e-6 倍 (最小 1e-6) の差まで同じとみなして判定します (スライダーの値は wasm の `f32` を経由するので、`0.1` のようなキーと完全には一致しない)。送るセルの値はキーの値そのものです。
- 判定などの純粋な関数は `inspector/keyforms/keyforms.ts` にあり、Vitest で検査します (`pnpm --filter @ugoe/editor test`、`just web-check` に含まれる)。

## デフォーマの編集

デフォーマを作るコマンドと、見た目を保つ親の付け替えです (実装は `crates/ugoe-editor-core/src/deformer_edit.rs`、テストは `deformer_edit_tests.rs`)。
[メッシュの編集](#メッシュの編集) と同じく、**既定値のパラメータで評価した姿** (基準の姿) を基準にします。基準の姿での各デフォーマのワールド変換 `F_D` ([evaluation.md](spec/evaluation.md#3-デフォーマのワールド変換)) は、モデルの複製に評価用のメッシュを足してランタイムで 1 回評価して作ります (ワープは制御点と同じ格子のメッシュを親の下に、回転デフォーマは `(0,0)`・`(1,0)`・`(0,1)` のメッシュを子に置く)。

| `op` | 内容 |
| --- | --- |
| `create_warp_deformer { id?, name, children, rows, cols, parent? }` | `children` (`{"kind": "mesh" \| "deformer", "id"}` の配列) を包むワープ (`rows` × `cols` セル、1〜64) を作り、子を付け替える |
| `create_rotation_deformer { id?, name, children, origin?, parent? }` | `children` を包む回転デフォーマ (角度 0・拡大率 1) を作り、子を付け替える。`origin` はキャンバス座標で、省略すると子の外接矩形の中心 |
| `reparent_preserving_look { element, new_parent }` | メッシュかデフォーマ (`element`) の親デフォーマを `new_parent` (`null` で最上位) にし、キーフォームを新しい親の座標系へ写し直す |

- **作るデフォーマ:** `id` を省略すると `name` から作ります (`ids::new_deformer_id`)。`parent` を省略すると、子の親の最も近い共通の祖先 (共通の祖先がなければ最上位)。パーツは子のパーツの最も近い共通の祖先 (なければ最初の子のパーツ)。格子は定数 (軸なし) です。
- **ワープの格子:** 子とその子孫の基準の姿の点 (メッシュの頂点・ワープの制御点・回転デフォーマの原点) を親の座標系へ写し (`F_parent⁻¹`)、その外接矩形に長い辺の 5% の余白を付け (点が 1 つなら 1px、親がワープなら UV で 0.01)、`rows` × `cols` に等分した点を制御点にします。一方の辺だけ 0 の矩形 (一列に並んだ点) は正方形に近づけます。
- **子の写し直し:** 子の古い親の変換を `F_old`、新しい親を `F_new` として、各キーフォームの点 `p` (メッシュの頂点・ワープの制御点・回転デフォーマの原点) を `p' = F_new⁻¹(F_old(p))` に置き換えます。
  - `F⁻¹` は、親なしなら恒等、回転デフォーマ (アフィン) なら逆行列、ワープならニュートン法 (初期値は四隅のアフィン近似の逆、収束しなければ各セルの中心。残差 1e-3px 以下で収束。歩幅は残差が減るまで半分にする) で求めます。潰れた格子など、逆に解けなければ `EditError::DeformerEdit` で拒否します。
  - 回転デフォーマの子は、ワールドの線形部分 `A_old(o)·R(angle)·S` が変わらないように角度・拡大率・反転も直します。`A(o)` は親が回転なら親のアフィンの線形部分、ワープなら原点での向き `R(δ)`、親なしなら単位行列です。`Q = A_new(o')⁻¹·A_old(o)` (相似変換) を `s·R(φ)` (行列式が正) と分けて `angle += φ`、`scale *= s`。行列式が負なら `Q = s·R(φ)·diag(-1, 1)` と分けて `angle = φ - angle`、`reflect_x` を反転します。`φ` は最初のキーフォームの値に近い角度 (360 度の違いを除く) にそろえ、キーフォーム間の補間を変えません。
  - 孫 (子の子) は変えません (子のワールド変換が保たれるので、孫の見た目も保たれる)。`mirror_axis` は、最初のキーフォームの点の重心の高さで同じ写像を通した x 座標にします (近似)。
  - 付け替えた要素は格子のすべてのセルが変わるので、`touched` に積みます (`derived` のセルは作り直す)。未知の種類のデフォーマは付け替えられません (子の座標系が決まらないため)。
- **見た目が保たれる範囲 (制限):** 写し直しは基準の姿での写像なので、次の場合に厳密です。それ以外では他のパラメータ値で見た目が変わりえます。
  - 基準の姿では、写像が点ごとなので **キーフォームの補間と入れ替えられる** とき (写像がアフィン、または既定値がすべての軸でキーの上にある) に厳密です。ワープへの出入りで既定値がキーの間にあると、補間の分だけずれます。
  - 他のパラメータ値では、古い親と新しい親の間にある (付け替えで経由しなくなる・新たに経由する) デフォーマがパラメータで変形すると変わります。たとえば子の親と同じ親の下にデフォーマを作るとき (`parent` の省略で子の親が同じ場合) は、親がアフィンな間はすべての値で厳密です。
  - **ワープの中のワープ:** 新しいワープは親のワープの写像を制御点でしか標本化しないので、親のワープがアフィンでない姿 (台形など) では子の見た目が少し変わります (Cubism と同じ性質)。
  - **ワープの子の回転デフォーマ:** 回転デフォーマはワープの伸縮を子に伝えない ([evaluation.md](spec/evaluation.md#回転デフォーマ)) ので、ワープの下に回転デフォーマを作ると、基準の姿以外ではワープの伸縮が子に伝わらなくなります。
  - 親の違う子をまとめて包むと、共通の祖先より下の親の動きには追従しなくなります (付け替えなので)。
- 他の拒否: 子がない・同じ子を 2 回指定・行数や列数が範囲外・`origin` が有限でない (`DeformerEdit`)、存在しない要素 (`NotFound`)、使用中の `id` (`DuplicateId`)、`parent` / `new_parent` が付け替える要素自身かその子孫 (循環、`DeformerEdit`)。

### ギズモとデフォーマの作成 (画面)

| 場所 | 操作 |
| --- | --- |
| インスペクタ (`inspector/DeformerProperties.svelte`) | 選択中のメッシュ・デフォーマ (複数可) を「Wrap in deformer」で包む。行数・列数を指定して「Warp」、または「Rotation」(原点は選択の中心)。作ったデフォーマを選択する。デフォーマを選ぶと種類・格子・キーフォームの数・親を示す |
| ビューポート (`viewport/tools/deformer/DeformerGizmo.svelte`) | 主な選択がデフォーマのとき、フォーム編集の代わりに出す。ワープは格子と制御点を示し、クリックで選択 (Shift で追加・解除)、格子の内側からのドラッグで矩形選択 (Shift で追加)、「All」/ Ctrl + A ですべて、選んだ点のドラッグでまとめて動かす。回転デフォーマは原点 (円)・回転の取っ手 (局所座標の上の向き)・拡大率の取っ手 (局所座標の右の向きの四角) を示し、原点の移動・回転 (Shift で 15 度刻み)・拡大率の変更 (原点からの距離の比) を行う |
| ツリー | デフォーマ・メッシュを別のデフォーマへドラッグすると `reparent_preserving_look` |

- ギズモの形は wasm の `WebPuppet.warp_points(id)` (キャンバス座標の制御点) と `rotation_frame(id)` (`[ox, oy, ax, ay, bx, by]`: 原点と局所座標の x 軸・y 軸の単位ベクトルの像) から読みます (今のパラメータ値。`deformer_ids()` で一覧)。ランタイムはデフォーマのワールド変換を公開しないので、wasm は評価用のメッシュを足したモデルの複製を別のインスタンスで評価します (`crates/ugoe-wasm/src/gizmo.rs`)。
- 編集は [フォーム編集](#キーフォームの編集) と同じく **今のパラメータ値のセル** に `set_keyform_cell` で書き込み (離したときに 1 つ、Undo の 1 単位)、キーの上にないときは編集を拒否して「Move to nearest key」を出します。制御点と原点は `canvas_to_local` で親の座標系に直します。角度は画面上の回転角を足します (親や自分の反転で向きが裏返っていれば符号を変える)。拡大率は正の値に限ります。
- ドラッグ中はギズモだけをプレビューし、子の描画は離して読み直すまで変わりません。
- 幾何の補助 (格子の辺・外周・取っ手の位置・角度・拡大率) は `viewport/tools/deformer/gizmo.ts` にあり、Vitest で検査します。

## テクスチャ

- `Document` はモデルと一緒に **テクスチャの画像** (`Textures` = テクスチャ名 → PNG のバイト列) を持ちます。モデルの `TextureRef` は名前と寸法だけで、画像はここにあります。画像はデコードせず、PNG ファイルのまま持ちます。
- 画像の追加・差し替え・削除もコマンド (`add_texture` など) で行うので、Undo/Redo の対象になります。[PSD の取り込み](#psd-の取り込み) はレイヤーごとに `add_texture` と `add_mesh` を `batch` にまとめて発行します。
- スナップショットでは画像を `Arc<[u8]>` で共有するので、Undo のたびに画像のバイト列が複製されることはありません。
- テクスチャ名は `textures/<name>.png` のファイル名になるので、`[A-Za-z0-9_.-]` だけからなり `.` で始まらないものに限り、大文字と小文字を区別せずに一意にします ([ファイル形式](spec/file-formats.md#名前と読み込み))。
- `png` は JSON では数値の配列になります。大きな画像をフロントエンドから送ることは想定しておらず、バックエンドの中 (取り込み処理) から使います。

## プロジェクト

- 保存形式は [`.ugoeproj/`](spec/file-formats.md#プロジェクト-ugoeproj) です。`ugoe_editor_core::Project` がプロジェクト 1 つ分の中身 (`ProjectMeta`・モデル・テクスチャ・物理・モーション・表情) を持ち、`save(dir)` / `load(dir)` で読み書きし、`export_bundle(&Project)` で `.ugoe` のバイト列を作ります。
- 編集中は、モデルとテクスチャを `Document` が持ちます (`Project::document()` で作り、`Project::from_document` で保存用に戻す)。物理・モーション・表情はまだ編集コマンドがないので、`Document` の外 (Tauri の `ProjectSession`) に置いたまま保存・書き出しします。これらの編集コマンドを作るときに `Document` へ移します。
- **未保存の変更:** `Document` は状態ごとに番号を振り、保存した状態の番号を覚えます。`is_modified()` は現在の状態が保存した状態と違うかを返すので、Undo で保存時の状態に戻ると「変更なし」に戻ります。失敗したコマンドは状態を変えません。

### Tauri コマンド

| コマンド | 内容 |
| --- | --- |
| `project_status` | 名前・保存先・未保存の変更の有無 (`ProjectStatus`) を返す |
| `new_project` | 空のモデル (1024×1024 のキャンバス) の新しいプロジェクトにする |
| `open_project(path)` | `.ugoeproj` ディレクトリを開く。失敗したら今のプロジェクトはそのまま |
| `save_project` | 開いた (前回保存した) 場所に保存する。保存先がなければエラー |
| `save_project_as(path)` | 別の場所に保存し、以後の保存先にする。拡張子 `.ugoeproj` がなければ付け、名前をディレクトリ名にする |
| `export_bundle(path)` | `.ugoe` ファイルに書き出す。保存状態は変えない |
| `import_psd(path, mesh_options?)` | PSD ファイルを今のドキュメントに取り込み ([PSD の取り込み](#psd-の取り込み)、Undo の 1 単位)、警告 (`PsdImportWarning[]`) を返す。最上位のパーツの名前はファイル名。`src-tauri/src/psd.rs` |

- ファイルのパスは、今はフロントエンドの入力欄で受け取ります (ファイルメニュー `src/lib/FileMenu.svelte`。「Import PSD…」も同じ。取り込みの警告はメニューの横に一覧で出す)。ファイルダイアログのプラグイン (`tauri-plugin-dialog` / `@tauri-apps/plugin-dialog`) は、依存の追加とライセンスの確認を別の作業に分けるため、まだ入れていません。入れるときは、入力欄をダイアログに置き換え、`capabilities/default.json` に `dialog:default` を足します。
- 未保存の変更がある状態で「新規」「開く」を押すと、もう一度押すまで実行しません (確認のダイアログの代わり)。
- 起動時は、白い 1×1 のテクスチャを付けた `single_quad` を、保存先のないプロジェクトとして開きます。

## ビューポート

[ADR 0003](adr/0003-editor-viewport-in-webview.md) のとおり、ビューポートは WebView 内の wasm で描画します。

- バックエンドはモデルが変わるたびに、モデルのバイト列をフロントエンドへ送ります (`tauri::ipc::Response` で生のバイト列を送る)。
- パラメータスライダーの操作はフロントエンドの wasm の中だけで完結させます (IPC を往復しない)。これにより 60fps を保ちます。スライダーの値はドキュメントの状態ではなく、表示の状態です。
- WebGPU が使えない WebView (Linux の WebKitGTK など) では、wgpu の WebGL2 バックエンドにフォールバックします。
- 大きなモデルで、モデル全体を送るのが遅くなった場合は、差分パッチを送る方式を検討します。

### ツール (選択・メッシュ編集)

ビューポートの上の編集ツールは `src/lib/viewport/ToolOverlay.svelte` が切り替え、ツールの本体は `src/lib/viewport/tools/` に置きます。選択は共有の `Selection` (`src/lib/selection.svelte.ts`) に書きます (Undo の対象ではない)。

- 評価後の頂点位置は wasm (`WebPuppet.drawable_positions`) から読みます (`tools/scene.svelte.ts`)。モデル・パラメータ・ビューが変わるたびに読み直します。幾何の補助 (最も近い頂点・矩形選択・辺) は `tools/geometry.ts` にあり、Vitest でテストします (`pnpm --filter @ugoe/editor test`)。
- **選択:** クリックで、その点にあるいちばん手前のメッシュを選びます (三角形の内外で判定。テクスチャの透明部分は考えない)。Shift / Ctrl クリックで追加・解除、何もない所のクリックで解除。**Alt + クリック** は、その点に重なっているメッシュを手前から順に選びます (`WebPuppet.hit_test_all`)。選択中のメッシュには輪郭を描きます。
- **メッシュ編集モード** (`tools/MeshEditTool.svelte`): メッシュを選んで「Edit mesh」かダブルクリックで入り、Esc か「Done」で抜けます (`Selection.meshEdit`)。コマンドは基準の姿を基準にするので、入るときにパラメータを既定値に戻します。
  - 頂点と辺を表示します。クリックで頂点を選び (Shift で追加・解除)、何もない所からのドラッグで矩形選択 (Shift で追加)、Ctrl + A ですべて選びます。選んだ頂点は `Selection.vertices[meshId]` に入ります。
  - 頂点のドラッグで選択中の頂点を動かします。ドラッグ中はフロントエンドでプレビューだけを描き、離したときに `move_mesh_vertices` を 1 つ送ります。
  - Ctrl (Cmd) + クリックで頂点を追加 (`add_mesh_vertex`)、Delete / Backspace で選択中の頂点を削除 (`remove_mesh_vertices`)、「Retriangulate」で三角形を作り直します。
  - パラメータを既定値から動かしたまま編集すると、プレビュー (今のパラメータでの位置) とコマンドの結果 (基準の姿での変換) がずれることがあります。
- **インスペクタ** の `src/lib/panels/inspector/MeshProperties.svelte` は、選択中のメッシュのテクスチャ・合成モード・カリング・マスク (参照・反転・結合方法)・`mask_only` を `set_mesh_properties` で変えます。

## 画面構成 (M2 の目標)

| 領域 | 内容 |
| --- | --- |
| 左 | パーツツリー、デフォーマツリー (ドラッグで親を付け替え) |
| 中央 | ビューポート (メッシュ編集、デフォーマのギズモ、キーフォームの編集) |
| 右 | パラメータ一覧 (スライダー、キーの追加と削除)、インスペクタ |
| 下 | タイムライン (M3: モーションの編集)、AI の提案パネル (M5) |

### ツリー (左)

- **Parts** はパーツの階層で、各パーツの下に子パーツ・所属するデフォーマ・メッシュを並べます。
  **Deformers** はデフォーマの階層で、各デフォーマの下に子デフォーマ・そのデフォーマを親に持つメッシュを並べます。
- メッシュは描画順で手前から並べます (パラメータが既定値のときの `draw_order`、同じなら `meshes` で後ろのものが手前)。
  パラメータが既定値のときに不透明度が 0 (祖先を含む) の要素には `hidden`、マスク専用のメッシュには `mask` の印を付けます。
- 選択はクリック (Shift / Ctrl で追加)、上下キー (Shift で追加)、左右キー (畳む・開く) で行います。
  外から選択された要素は祖先を開いてスクロールします。Enter / F2 / ダブルクリックで名前の変更、Delete で削除 (確認あり)。
- ドラッグ & ドロップ (ポインタイベントで実装。WebView のファイルのドロップと干渉しないように HTML5 の DnD は使いません):
  - パーツ / デフォーマ / メッシュを行に落とすと親を付け替えます (`set_part_parent`、`set_deformer_part`、
    `set_mesh_properties { part }`、デフォーマ・メッシュの親デフォーマは `reparent_preserving_look`)。空いた所に落とすと最上位 (親なし) へ。
    循環はフロントエンドでも先に弾きます。親デフォーマの付け替えは **既定値のパラメータでの見た目を保ちます**
    ([デフォーマの編集](#デフォーマの編集)。他のパラメータ値では変わりうる)。
  - 同じ親の中でメッシュの行の上半分 / 下半分に落とすと、その手前 / 奥へ並べ替えます (`reorder_mesh`)。
    `reorder_mesh` は描画順が同じメッシュどうしの前後しか変えられないので、描画順が違えば拒否します。

## フロントエンドの規約

- Svelte 5 の runes (`$state`、`$derived`、`$effect`) を使います。ストアのライブラリは入れません。
- バックエンドとの通信は `src/lib/ipc.ts` に集約します。コンポーネントから `invoke` を直接呼びません。
- IPC の引数・戻り値の型は Rust から生成したもの (`src/lib/generated/types.ts`) を使い、手で書きません。`ipc.ts` が再エクスポートするので、コンポーネントは `./ipc` から import します。
- lint と整形は Biome、型検査は `svelte-check` で行います (`just web-check`)。
- 表示や操作のロジックのうち DOM に依存しないもの (ツリーの組み立て、ドロップの可否など) は素の `.ts` に置き、
  Vitest で単体テストします (`*.test.ts`、`pnpm --filter @ugoe/editor test`。`just web-check` に含まれる)。

## 型の生成

Rust の型を正として、次のファイルを生成します。**生成物はリポジトリにコミットします** (Rust をビルドしなくてもフロントエンドを検査・ビルドでき、変更の差分がレビューで見えるように)。手で編集しません。

| 生成物 | 元 | 使うところ |
| --- | --- | --- |
| `apps/editor/src/lib/generated/types.ts` | `Command`・`Model`・`Motion`・`Expression`・`PhysicsSettings` (とそれらが参照する型)、IPC の戻り値 (`AppInfo`・`ProjectStatus`) | フロントエンド (`ipc.ts` が再エクスポート) |
| `apps/editor/src/lib/generated/samples.ts` | フィクスチャのモデルと、全バリアントのコマンドを serde で書き出した JSON | 型検査だけ (生成した型と serde の JSON が合うことを `svelte-check` で確かめる) |
| `crates/ugoe-editor-core/schema/command.schema.json` | `Command` の JSON Schema (draft 2020-12) | AI のツール定義 (`ugoe_editor_core::codegen::COMMAND_SCHEMA_JSON`、`command_schema()`) |

- **作り直す:** Rust の型 (`Command`・モデルの型・IPC の戻り値) を変えたら `just gen` を実行し、生成物も同じコミットに含めます。`just gen` は `apps/editor/src-tauri/examples/gen.rs` を実行します。
- **検査:** `just test` の `generated_files_are_up_to_date` (src-tauri) と `checked_in_command_schema_is_up_to_date` (ugoe-editor-core) が、生成物が最新でなければ失敗します。CI はさらに `just gen` の後に `git diff --exit-code` で差分がないことを確かめます。
- **道具:** TypeScript は [ts-rs](https://github.com/Aleph-Alpha/ts-rs)、JSON Schema は [schemars](https://github.com/GREsau/schemars) (どちらも MIT)。どちらも serde の属性 (`tag`・`rename_all`・`default`・`skip_serializing_if` など) を読むので、生成した型は serde の JSON と一致します。
- **ランタイム層:** `ugoe-core` と `ugoe-model` は feature `ts` / `schema` の裏で `TS` / `JsonSchema` を実装します。有効にするのは `ugoe-editor-core` だけで、ランタイム (`ugoe-wasm` など) の依存には入りません。
- **型の対応:**
  - `Id<T>` (`ParamId` など) は `string`。`Vec2` は `[number, number]` (glam の serde 形式。フィールドに `ts(as = "crate::Vec2Repr")` と `schemars(with = ..)` を付ける)。`u64` も `number`。
  - `skip_serializing_if` と `default` を持つフィールドは省略できる (`field?: T`)。`default` だけのフィールドは、書き出す JSON には必ずあるので必須にする。
  - 未知の値 (`Unknown` のバリアント) は書き出せないので、型にもスキーマにも含めない (`ts(skip)` / `schemars(skip)`)。
  - JSON Schema は読み込み側の形なので、`default` を持つフィールドは必須にしない。
- モデルに型を追加したら、`#[cfg_attr(feature = "ts", derive(ts_rs::TS))]` と `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` を付けます (既存の型を参照)。
