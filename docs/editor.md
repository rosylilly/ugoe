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
| パラメータ | `add_parameter`、`remove_parameter`、`set_parameter_range`、`rename_parameter`、`set_parameter_group` (`null` で外す)、`set_parameter_mirror` (`null` で外す。対は双方向に張り、どちらかと対になっていたパラメータの対は外す (`set_mirror_pair` と同じ)。自分自身と対にすると値の符号を反転して対応させる)、`set_parameter_internal` (中継用のパラメータにする・戻す。[model.md#パラメータ](spec/model.md#パラメータ)) |
| パラメータのグループ | `add_parameter_group`、`remove_parameter_group`、`rename_parameter_group`、`reorder_parameter_group` ([パラメータのグループ](#パラメータのグループ)) |
| パーツ | `add_part`、`remove_part`、`rename_part`、`set_part_parent`、`set_part_keyforms` |
| デフォーマ | `add_deformer`、`remove_deformer`、`rename_deformer`、`set_deformer_parent`、`set_deformer_part`、`set_warp_keyforms` (`rows` / `cols` は変えない)、`set_rotation_keyforms`、`create_warp_deformer`・`create_rotation_deformer`・`reparent_preserving_look`・`move_rotation_origin`・`unwrap_deformer` ([デフォーマの編集](#デフォーマの編集)) |
| メッシュ | `add_mesh` (`meshes` の末尾に追加)、`remove_mesh`、`rename_mesh`、`set_mesh_properties`、`set_mesh_parent`、`set_mesh_keyforms`、`set_mesh_keyform`、`reorder_mesh`、`regenerate_mesh` ([メッシュの再生成](#メッシュの再生成))、`move_mesh_vertices`・`add_mesh_vertex`・`remove_mesh_vertices`・`retriangulate_mesh` ([メッシュの編集](#メッシュの編集))、`create_draw_clone`・`sync_draw_clone` ([描画用の複製](#描画用の複製)) |
| キーフォーム | `add_keyform_key`、`remove_keyform_key`、`set_keyform_cell`、`set_mesh_draw_order`、`set_keyform_interpolation` ([キーフォームの編集](#キーフォームの編集)) |
| テクスチャ | `add_texture`、`set_texture_image`、`remove_texture`、`set_normal_map` ([法線マップ](#法線マップ)) |
| ライティング | `set_lighting` (`Model::lighting` を変える。[法線マップ](#法線マップ)) |
| エフェクト | `set_effects` (`Model::effects` を丸ごと置き換える。[エフェクトの設定](#エフェクトの設定)) |
| グルー | `create_glue`、`set_glue`、`remove_glue` ([グルーの編集](#グルーの編集)) |
| ポーズ | `set_pose` (`Model::pose` を丸ごと置き換える。[ポーズの編集](#ポーズの編集)) |
| 着せ替えのスロット | `add_slot`、`set_slot`、`remove_slot` ([スロットの編集](#スロットの編集)) |
| モジュールのプロジェクト | `set_module_settings` (名前・スロット・条件と、取り付け先の違うスロットへ見た目を保って移すか `keep_look`。[モジュールの設定](#モジュールの設定)) |
| PSD のレイヤーの分類 | `set_layer_correction` (レイヤーのパスの分類を利用者の答えにする・`null` で消す。モデルは変えない。[分類の直し](#分類の直し)) |
| キャンバス | `set_canvas` (`width`・`height`・`origin`。大きさは 0 より大きい有限の値。要素の座標は変えない) |
| キーフォームの推定 | `infer_keyforms`、`mirror_keyforms`、`mark_keyform_authored`、`mark_keyform_derived`、`set_keyform_inference`、`set_mirror_pair` (下の「推定とミラー」)、`set_depth`・`apply_depth_preset` ([深度の編集](#深度の編集)) |
| モーション・表情 | `add_motion`、`remove_motion`、`rename_motion`、`set_motion_properties`、`add_motion_curve`、`remove_motion_curve`、`set_motion_curve_fade`、`set_motion_key`、`move_motion_key`、`remove_motion_key`、`set_motion_segment`、`set_motion_events`、`add_expression`、`remove_expression`、`rename_expression`、`set_expression_properties`、`set_expression_parameters` ([モーションと表情の編集](#モーションと表情の編集)) |
| 物理 | `set_physics_world`、`add_physics_rig`、`create_physics_rig`、`remove_physics_rig`、`rename_physics_rig`、`set_physics_inputs`、`set_physics_outputs`、`set_physics_particles`、`set_physics_normalization`、`set_physics_rig_colliders`、`set_physics_frame`、`add_physics_collider`、`remove_physics_collider`、`set_physics_collider`、`reorder_physics_rig`、`apply_physics_template` ([物理の編集](#物理の編集)・[物理のテンプレート](#物理のテンプレート)) |
| その他 | `batch` |

- `set_mesh_properties` は `part`・`texture_name`・`blend`・`culling`・`masks`・`mask_mode`・`mask_only`・`lit` のうち、指定したフィールドだけを変えます。
  - テクスチャは名前 (`texture_name`、`TextureRef::name`) で指定します。ない名前は `NotFound` で拒否します。`remove_texture` は後ろのテクスチャの番号を詰めるので、名前で指定すれば、古い状態で作った提案を後で適用しても別のテクスチャを指しません (T0679)。
  - **互換:** 以前の形の `texture` (`Model::textures` の番号) も読み、その番号をそのまま設定します (`.ugoeproj` に保存された提案など、古い形で書かれたコマンドを開けるように)。番号は非推奨で、エディタ・AI・MCP の送り手は使いません。AI・MCP のツール定義 (`command.schema.json`) には出しません。`texture_name` と同時に指定すると `edit.conflicting_texture` で拒否します。番号は古い提案が作られた後にテクスチャを消すと別のテクスチャを指しうるので、番号を使うコマンドは名前で作り直すのが安全です。
- `set_mesh_keyform` は格子の 1 セルを置き換え、そのセルの由来を `authored` にします。セルは「軸のパラメータ ID → キーの値」の対応で指定します (例: `"cell": {"AngleX": 30, "AngleY": 0}`)。格子のすべての軸を指定し、値はキーと完全に一致させます。軸のない格子は `{}` です。
- `reorder_mesh` は `meshes` の中でメッシュを `before` の直前 (`null` なら末尾) に移します。描画順 (`draw_order`) が同じメッシュの前後関係が変わります。
- `set_deformer_parent` と `set_mesh_parent` は親を付け替えるだけで、座標は変えません (子の座標は新しい親の局所座標として解釈されるので、見た目は変わりうる)。見た目を保つ付け替えは `reparent_preserving_look` です ([デフォーマの編集](#デフォーマの編集))。

- `add_texture` は `name` と `png` (PNG ファイルのバイト列) を受け取り、`Model::textures` の末尾に追加します。幅と高さは PNG の IHDR から読みます。名前の規則は下の「テクスチャ」を参照。`set_texture_image` は画像を差し替え、寸法も PNG に合わせます。`remove_texture` は使っているメッシュがあれば拒否し (`HasChildren`)、後ろのテクスチャを指すメッシュの `texture` (インデックス) を 1 つ詰めます。消したテクスチャの法線マップの画像は、ほかのテクスチャから指されていなければ表から除きます (Undo で戻る)。

### 推定とミラー

キーフォームの推定 ([spec/keyform-inference.md](spec/keyform-inference.md)) のコマンドです。アルゴリズムは `ugoe-rigging` にあります。
格子を持つ要素は `target` で `{"kind": "mesh", "id": "EyeL"}` のように指定します (`kind` は `part`・`mesh`・`deformer`・`glue`)。

| `op` | 内容 |
| --- | --- |
| `infer_keyforms` | `target` の格子の `derived` のセルを作り直す。`method` は `from_grid` (既定。格子の `inference` に従い、省略なら加算合成)・`additive`・`depth_rotation` (格子の `inference` が `depth_rotation` であること)。書き換えるのはその方法で作ったセルだけで、`overwrite_derived: true` なら別の方法 (ミラーなど) で作ったセルも書き換える |
| `mirror_keyforms` | `source` を反転して `target` のセルを作る (由来は `mirror`)。先の `authored` 以外のセルと、**未編集の `authored` のセル** (基準のセルと同じ値のまま。キーを追加しただけのセル。[keyform-inference.md#未編集のセル](spec/keyform-inference.md#未編集のセル)) を書き換え、`source` を先の格子の `mirror_source` に記録する。`axis` (対称軸)・`tolerance` (頂点の許容距離) は省略できる (指定すれば記録する)。`overwrite_authored: true` で先の `authored` のセルもすべて上書きする。書いたセルの数と残したセルの数を `ApplyReport::mirrored` で返す (下) |
| `mark_keyform_authored` | `cells` (セルの指定の配列。形は `set_mesh_keyform` の `cell` と同じ) の由来を `authored` にする。値は変えない |
| `mark_keyform_derived` | `cells` の由来を `derived` にし、格子の推定の設定で作り直す。基準のセルは `derived` にできない (推定のエラーで拒否) |
| `set_keyform_inference` | `target` の格子の推定の設定 (`inference`、[`KeyformInference`](spec/keyform-inference.md#回転と投影)) を置き換える。省略 (`null`) なら加算合成。方法を記録していない `derived` のセルを新しい設定で作り直す (`depth_rotation` のセルも、設定が深度からの回転なら作り直す)。未知の方法は `UnsupportedInference`、`yaw` / `pitch` が格子の軸でなければ検証エラーで拒否する |
| `set_mirror_pair` | メッシュ・デフォーマの左右の対 (`mirror`、相手の ID。省略で対を外す) と `target` の対称軸 (`axis` → `mirror_axis`。省略で親で決まる既定値) を設定する。対は双方向に張り、`target` と相手のどちらかと対になっていた要素の対は外す。パーツ・グルーは `MirrorPairNotSupported`。対が変わった要素をミラー元にする格子はミラーし直す (失敗すれば下の警告) |

- 推定のエラー (中立のキーがない、キーが昇順でない、基準のセルが `derived`、ミラーの対でない、など) は `EditError::Inference` で拒否し、モデルは変えません。
- **自動の作り直し:** 格子のセル (値や由来) を変えたコマンドの後、`Document::apply` はその格子の `derived` のセルを **セルに記録した推定の方法で** 作り直し、その格子をミラー元 (`mirror_source`) にする格子のミラーのセルもミラーし直します。同じ Undo 単位に含まれ、`batch` では全体を適用した後にまとめて行います。加算合成・深度からの回転で作り直せなければコマンドを拒否します (ミラーのやり直しの失敗は、ミラーのセルをそのままにして続ける)。規則は [spec/keyform-inference.md#自動の作り直し](spec/keyform-inference.md#自動の作り直し)。
  - 対象は `set_*_keyforms`・`set_mesh_keyform`・`set_keyform_cell`・`mark_keyform_*`・`infer_keyforms`・`mirror_keyforms`・`set_keyform_inference`・`set_mirror_pair`・頂点の編集 (`move_mesh_vertices` など)・深度の編集 (`set_depth`・`apply_depth_preset`) です。新しいコマンドは、格子のセルを変えたら `apply_to` の `touched` に要素を積むことで対象になります (`crates/ugoe-editor-core/src/document.rs`)。
  - 推定のコマンドも積みます。作り直しはセルごとの方法で行うので推定の結果を別の方法で上書きせず、その格子をミラー元にする格子へ変化を伝えます。
  - パラメータの範囲の変更など、セルを変えないコマンドでは作り直しません。必要なら `infer_keyforms` を実行します。
- **ミラーのやり直しの失敗の警告:** 自動のミラーのやり直しに失敗した (ミラー元の頂点を許容距離を超えて動かした、対を外した、など) 格子は、コマンドを拒否せずミラーのセルを前の値のまま残し、`RederiveWarning { target, source, message }` (格子・ミラー元・`InferError` の文) を 1 組に 1 つ返します。
  親を変えたコマンド (付け替え・デフォーマで包む) で、その要素の左右の対がミラーし直せなくなった場合も、同じ警告を返します ([keyform-inference.md#自動の作り直し](spec/keyform-inference.md#自動の作り直し))。
  `Document::apply_with_report` が `ApplyReport { warnings, mirrored }` で返し、`Document::apply` は同じ適用をして警告を捨てます (CLI などの既存の呼び出し側はそのまま)。
  Tauri の `apply_command` は `ApplyReport` を返し、エディタはインスペクタの上に閉じられる通知として出します ([画面での操作](#推定の画面での操作))。
- **ミラーの結果:** 明示的に実行した `mirror_keyforms` ごとに、`ApplyReport::mirrored` に `MirrorReport { source, target, written, unedited, kept_authored }`
  (書いたセルの数・そのうち未編集の `authored` だったセルの数・`overwrite_authored` なら書けたが `authored` なので残したセルの数) を積みます。
  1 つも書かなかったときに、何もしなかったように見えないよう、エディタが理由と「上書きしてミラーする」ボタンを出すのに使います (T0627)。
- **`derived` のセルの編集:** セルを書き換えるコマンドは、書き換えた `derived` のセルを `authored` にします。`set_*_keyforms` で格子を丸ごと置き換えるときは、軸が同じなら、値が変わったのに `derived` (方法を問わない) と指定されたセルを `authored` にします。
- ミラーで作ったセル (`mirror`) は、ミラー元の格子が変わるとミラーし直され、先の格子を編集しても上書きされません。方法を記録していない版 (T0614 より前) のファイルでミラーしたセルは `derived` のままなので、以前と同じく格子の方法で作り直されます (`mirror_keyforms` を再度実行すると `mirror` になる)。

#### 推定の画面での操作

操作はすべて上のコマンドを送るだけです (組み立ては `inspector/inference/inference.ts`、Vitest で検査)。

| 場所 | 操作 |
| --- | --- |
| キーの格子 (`inspector/keyforms/KeyformInspector.svelte`) | セルの記号と色で由来を示す: `A` authored (実線) / `+` additive / `R` depth_rotation / `M` mirror / `D` 方法の記録のない derived (推定のセルは方法の色の破線と薄い地)。凡例は `inspector/inference/OriginLegend.svelte`。ツールチップに、`derived` なら作り直す方法、`mirror` ならミラー元を出す。セルを押すとそのセルへ移動して選び、Shift / Ctrl / Cmd で押すと選択に加える (外す) |
| 推定 (`inspector/inference/InferenceSection.svelte`) | 格子の推定の設定 (加算合成 / 深度からの回転の `angle_scale`・`pivot`・`pivot_depth`・`yaw`・`pitch`。回転はメッシュとワープだけ。回転の中心の初期値は基準のセルの点の外接矩形の中心) を編集して「Apply settings」(`set_keyform_inference`)。「Infer」「Infer (overwrite derived)」(`infer_keyforms` の `from_grid`)。選んだセル (なければ今のセル) の「Mark authored」「Mark derived」。メッシュ・デフォーマでは「Mirror from…」(元は対の相手と自分自身。軸・許容距離・authored の上書きは任意) と、記録されたミラー元 (`mirror_source`) の表示、「Mirror partner」で対と対称軸の設定 (`set_mirror_pair`)。「Mirror」の後は `MirrorReport` から結果 (`Mirrored 1 cell from 左目のワープ (Warp_2).` など) を出し、1 つも書かなければ理由 (残した `authored` のセルの数) と「Overwrite authored cells and mirror」を出す。要素の選択欄は表示名と ID (`左目のワープ (WarpLayer8)`) を出す |
| 通知 (`inspector/inference/RederiveNotice.svelte`) | 適用したコマンドが返した `RederiveWarning` を、インスペクタの上に、閉じるボタン (×) で閉じるまで出す (`Notice`) (同じ格子とミラー元の組は新しいもので置き換える) |

- セルの選択 (`inspector/inference/cellSelection.svelte.ts`) は表示の状態で、Undo の対象にしません。別の格子のセルを選ぶと前の選択は消えます。

### 深度の編集

メッシュの頂点ごと (`Mesh::depth`) とワープデフォーマの制御点ごと (`WarpDeformer::depth`) の深度 (キャンバスのピクセル単位、手前が正) を設定するコマンドです。深度は [深度からの回転](spec/keyform-inference.md#深度からの回転) の入力で、ランタイムは読みません。
要素は `target` で `{"kind": "mesh" | "deformer", "id": "..."}` (`ChildRef`) と指定します。デフォーマはワープだけです (回転デフォーマは `DeformerKindMismatch`)。

| `op` | 内容 |
| --- | --- |
| `set_depth { target, depth }` | 深度を置き換える (省略・`null` で外す)。長さが点 (頂点・制御点) の数と違えば `DepthLength`、有限でない値があれば `NonFiniteDepth` で拒否する |
| `apply_depth_preset { target, preset }` | プリセットで深度を作り、`set_depth` と同じく置き換える。`preset` は下の表 |

| `preset` | 深度 |
| --- | --- |
| `{"kind": "uniform", "value": v}` | すべての点を `v` にする |
| `{"kind": "dome", "peak": h}` | 基準の姿の点の外接矩形の中心を `c`、半分の幅・高さを `r` とし、内接する楕円で正規化した距離 `ρ = |((x − cx) / rx, (y − cy) / ry)|` から `h · sqrt(max(0, 1 − ρ²))` (中心で `h`、楕円の上と外で 0 の半球)。幅・高さが 0 の向きは数えない。`h` が負ならくぼみ |
| `{"kind": "points", "indices": [...], "value": v}` | 点 `indices` (頂点・制御点の番号) だけを `v` にし、他の点は今の深度 (なければ 0) のまま。範囲外の番号は `DepthPointOutOfRange` |

- 基準の姿は、軸ごとにパラメータの既定値に最も近いキー (中立のキーがあればそれ) を採ったセルの点 (親座標系) です。計算は `ugoe_editor_core::depth` の純粋な関数 (`uniform_depth`・`dome_depth`・`points_depth`) にあります。
- 深度を変えると格子を作り直すので (`touched`)、深度からの回転のセルは新しい深度で作り直されます (同じ Undo 単位)。深度からの回転のセルが残っている格子の深度を外すと、作り直しが失敗して拒否されます (`Inference`。先にセルを `authored` にするか、推定の方法を変える)。
- 頂点の編集は深度の並びを保ちます: 追加した頂点の深度は重心座標で補間し、削除した頂点の深度は除き、移動では変えません ([メッシュの編集](#メッシュの編集))。

#### 深度の画面での操作

| 場所 | 操作 |
| --- | --- |
| 深度 (`inspector/depth/DepthSection.svelte`) | 主な選択がメッシュかワープのときに出す。点の数と深度の範囲、「Uniform」(値) の「Set all」、「Dome peak」(値) の「Dome」、「Selected」(値) の「Set selected」、「Clear depth」。選んだ点は、メッシュはメッシュ編集モードで選んだ頂点 (`Selection.vertices`)、ワープはギズモで選んだ制御点 (`Selection.controlPoints`) |
| ビューポート (`viewport/tools/depth/DepthOverlay.svelte`) | 深度の節の「Show in viewport」(`ViewportController.showDepth`) で、主な選択の点に深度の色の輪を重ねる (手前ほど赤、奥ほど青、0 は灰色。尺度は深度の絶対値の最大)。操作は受けず、ツールの上に描く |

- コマンドの組み立てと色は `inspector/depth/depth.ts` の純粋な関数で、Vitest で検査します。

### パラメータのグループ

パラメータのグループ (`Model::parameter_groups`。[model.md#パラメータ](spec/model.md#パラメータ)) は、エディタのパラメータのパネル・プレビュー・ビューアでスライダーを区切る見出しです。
ランタイムの挙動には影響しません。グループそのものを足す・消す・名前を変える・並べ替えるコマンドと、パラメータを入れる `set_parameter_group` があります。

| コマンド | すること | 拒否 |
| --- | --- | --- |
| `add_parameter_group { group }` | `{ id, name }` のグループを `parameter_groups` の末尾に足す | ほかのグループと ID が重なれば `edit.duplicate_id` (`kind` は `parameter group`) |
| `remove_parameter_group { id }` | グループを消し、属していたパラメータの `group` を `null` にする (パラメータは消さない) | 無いグループは `edit.not_found` |
| `rename_parameter_group { id, name }` | 表示名だけを変える (ID は変えない) | 無いグループは `edit.not_found` |
| `reorder_parameter_group { id, before }` | `parameter_groups` の中でグループを `before` の直前 (`null` なら末尾) へ移す。並びはプレビューとビューアの区切りの順になる | 無いグループ (`id`・`before`) は `edit.not_found`、`before` が自分なら `edit.self_reference` |

- どれも 1 つで Undo の 1 単位です。グループを消して Undo すると、グループとパラメータの割り当てがそろって戻ります。
- `set_parameter_group` で無いグループを指すと、適用後の検証 (`validation.unknown_reference`) で拒否します。
- モジュールのプロジェクトでは、グループは本体のものなので変えられません (`edit.read_only_base`)。
- **画面:** パラメータのパネル (`ParametersPanel.svelte`) の一覧の下の「Groups」(`panels/parameters/ParameterGroups.svelte`。折りたためる節で、
  見出しに数を出す)。「+」(空なら「New group」) で ID と名前を入れて作り、名前を押して名前の変更と削除をします。
  行にはグループに入っているパラメータの数を出します。パラメータをグループに入れるのは、各パラメータの編集欄の「Group」
  (`ParameterGroupSelect.svelte`。選ぶとすぐ `set_parameter_group`。「(none)」で外す) です。コマンドの組み立ては `parameters/groups.ts` (Vitest)。
- **並べ替え (T1630):** 行のドラッグ、▲▼、名前のボタンで Alt + ↑ / ↓ のどれでも、`reorder_parameter_group` を 1 つ送ります
  (ドラッグはポインタだけの近道で、キーボードでは ▲▼ か Alt + ↑ / ↓。Alt + ↑ / ↓ の後は動かした行の名前にフォーカスを戻す)。
  ドラッグはポーズの要素の並べ替え ([ポーズの編集](#ポーズの編集)) と同じくポインタのイベントで作り (HTML5 の DnD は Windows の WebView2 で
  ファイルのドロップと両立しない)、5px 動かして始まり、落とした行の前 (行の上半分) か後ろ (下半分) に入れます。落とす先は強調色の線で示し、
  動かない所 (自分の前後) には出しません。落としたときだけ送り、Esc とポインタの取り消しでは送りません。▲▼ と編集欄の上からは始まりません。
  モジュールのプロジェクトと、編集を送っている間は並べ替えません。落とす先の計算は `groups.ts` の `dropIndexAt`・`dropGroupCommand`。
- **一覧の区切り (T1630):** グループに入ったパラメータがあると、パラメータのパネルの一覧をプレビュー・ビューアと同じ規則で区切ります
  (`preview/preview.ts` の `groupParameters`。グループの定義順、区切りの中はモデルの定義順、空のグループは出さず、グループの無いものと
  定義に無いグループを指すものは最後の「Other」)。区切りは折りたためる見出し (`Section`) で、右に区切りのパラメータの数を出します。
  どのパラメータもグループに入っていなければ、見出しを出さずに並べます (グループの無いモデルで「Other」だけが出ないように)。
  区切りの開閉は localStorage の `ugoe.editor.parameterSectionsClosed` に覚えます ([覚える値](#覚える値)。T1646)。
  覚えるのは閉じた区切りのグループの ID (「Other」は空の文字列) だけで、既定は開きます。プロジェクトごとには分けないので、同じ ID の
  グループはどのプロジェクトでも同じ開閉になります (よく使うグループの ID はモデルをまたいでそろうことが多く、プロジェクトのファイルに
  表示の状態を入れないため)。数は新しく閉じたものから 200 個まで (`src/lib/panels/parameters/sections.ts`)。
- **絞り込み (T1646):** パラメータがあると、一覧の上に絞り込みの欄 (検索の欄。読み上げの名前は「Filter the parameters」) を出し、
  名前か ID に語を含むパラメータだけを出します (大文字と小文字を区別しない。プレビューと同じ `preview/preview.ts` の `filterSections`)。
  絞り込み中は覚えた開閉によらず一致した行のある区切りを開いて出し、一致の無い区切りは出しません。絞り込み中に閉じた区切りは覚えず、
  語を変えると開き直します。語を消すと覚えた開閉に戻ります。一致が無ければ「No parameter matches "…".」を `role="status"` で出します。
  グループの無いモデルでは、見出しの無い一覧をそのまま絞ります。
  - **一覧の中のドラッグでグループを変えることはしない:** 一覧の行はスライダー・値の欄・キーの印でほとんど埋まり、ポインタの操作
    (スライダーのドラッグ・印を押す) と重なるので、行をつかむ場所が取れません (つかみの印を足すと、100 個のパラメータの一覧の幅を
    さらに狭める)。グループを変えるのは編集欄の「Group」で、キーボードでもそのまま使えます。

### 削除の連鎖と拒否

要素を消すコマンドは、要素そのもの (と、それに従属して消える要素) だけを消すか、他の要素が依存していれば拒否します。
残った要素から消えた要素への参照は、どのコマンドで消えたかによらず、適用の後に 1 か所でまとめて外します (下の「参照の後始末」)。

| コマンド | コマンド自身がすること |
| --- | --- |
| `remove_parameter` | **連鎖して直す。** このパラメータを軸に持つキーフォーム格子 (パーツ・デフォーマ・メッシュ・グルー。ブレンドシェイプ格子を含む) から軸を取り除き、パラメータの `default` と等しいキーの断面を残す。等しいキーがなければ最も近いキー (等距離なら小さい方) の断面を残す。セルの由来 (`origins`) も同じセルを残す。軸がなくなったブレンドシェイプ格子は取り除く。深度からの回転の `yaw` / `pitch` がこのパラメータなら外す (回転の中心などの設定は残す) |
| `remove_part` | 子パーツ・所属するデフォーマ・所属するメッシュ・このパーツをモジュールの親 (`part`) にするスロットが残っていれば **拒否する** (`EditError::HasChildren` に子の一覧が入る) |
| `remove_deformer` | 子デフォーマ・子メッシュ・このデフォーマを取り付け先 (`anchor`) にするスロットが残っていれば **拒否する** |
| `unwrap_deformer` | 直接の子を親へ付け替えてから、`remove_deformer` と同じ規則で消す |
| `remove_mesh` | **連鎖して消す。** このメッシュを使うグルーと当たり判定を削除する。描画用の複製の元を消すと、複製はつなぐグルーを失って普通のメッシュ (基準の形) として残る |
| `remove_glue`・`remove_slot`・`set_hit_areas`・`remove_physics_collider` | 要素を消す (置き換える) だけ |
| `remove_parameter_group` | 属していたパラメータの `group` を `null` にする (パラメータは消さない) |

**参照の後始末** (`crates/ugoe-editor-core/src/cleanup.rs`)。`Document::apply` は、コマンド (`batch` なら全体) を適用した直後に、
適用の前後のモデルと物理を比べ、**前にあって後にない ID** を指す次の参照を外します。

| 消えた要素 | 外す参照 |
| --- | --- |
| パラメータ | 他のパラメータの `mirror` (`null` にする)、エフェクト (目パチ・呼吸・リップシンク) の対象、スロットの `parameters` |
| パーツ | ポーズの要素とその `links` (消して空になったグループは消す)、スロットの `replaces`・`pose_parts` |
| デフォーマ・メッシュ | 同じ種類の要素の `mirror` (`null` にする)。メッシュは他のメッシュの `masks` とスロットの `mask_sources` からも |
| パーツ・デフォーマ・メッシュ・グルー | その要素をミラー元 (`keyforms.mirror_source`) にしている同じ種類の格子 (ブレンドシェイプ格子を含む) の記録。その格子の `mirror` のセルは `authored` にする (値は変えない) |
| 当たり判定 | アバタープロファイルの標準の当たり判定 (`avatar.hit_areas`) |
| スロット | アバタープロファイルの `slots` |
| パラメータ・デフォーマ・メッシュ | 物理の参照 ([物理の編集](#物理の編集)) |
| 物理の衝突体 | リグとスロットの `colliders` |

- 前後を比べるので、`batch` の中で消して同じ ID で足し直した要素は消えたことになりません (その要素への参照は残る)。
  読み込んだ時点で既に存在しない要素を指していた参照も変えません (問題を増やさない編集はできる、の考え方)。
- 後始末は、`derived` のセルの作り直しと検証の前に行います。新しく要素を消すコマンドを足しても、参照を外すコードを書く必要はありません。
  新しい種類の参照をモデルに足したら `cleanup.rs` に足します。
- モジュールのプロジェクトの `set_module_settings` ([モジュールの設定](#モジュールの設定)) と本体の読み直し (`Document::rebase_module`・`rebase_module_on`。[本体の変化](#本体の変化)) は、
  この後始末を通しません。どちらも作業用のモデルを差分で書き換えず、モジュールに分けてから本体と合成し直す (`ugoe_runtime::compose`) からです。
  要素が消えることはあります (名前を変えるとモジュールの要素の ID がすべて替わる、スロットを変える・本体のスロットが置き換えるパーツを変えると
  置き換えを隠すパーツ `#replaced/…` が替わる、本体から要素が消える) が、合成は本体と、モジュールの中の参照 (接頭辞を付け替えたもの) だけから
  モデルと物理を作り直すので、消えた要素への参照は残りません。モジュールが参照する本体の要素 (マスクの元・公開されたパラメータ・衝突体・ポーズの要素・
  スロット) が無くなったときは合成がエラーになり、何も変えません。テストは `module_project_tests` の `module_settings_leave_no_references_to_removed_elements`・
  `rebase_leaves_no_references_to_removed_elements` (合成し直した結果に後始末を通しても変わらず、検証を通ることを確かめる)。
- 拒否する場合は、先に子を削除するか付け替えます。まとめて行うなら `batch` を使います。
- モーション・表情は `Model` の外のデータなので、`remove_parameter` はこれらを書き換えません。消したパラメータへの参照が残ることがあります (ランタイムは無視する。タイムラインは `(missing)` の行として出す)。
  アバタープロファイルのトラッキングの上書き・視線のパラメータも書き換えません (準拠の検査が知らせる)。

### Undo

最初はスナップショット方式 (`Model` を丸ごと clone) で実装します。簡単で確実だからです。
モデルが大きくなってメモリや時間が問題になったら、各コマンドが逆操作を返す方式に移行します ([ADR 0005](adr/0005-editor-command-model.md))。

- **履歴の上限:** `Document` が覚える状態は、Undo と Redo を合わせて **100 個** (`DEFAULT_UNDO_LIMIT`) までです。超えると最も古い Undo から捨てるので、
  101 回前の操作には戻れません。上限は `Document::set_undo_limit` / `with_undo_limit` で変えられます (下げると古い Undo から、Undo が尽きたら遠い Redo から捨てる。
  0 なら履歴を持たない)。
- **上限の設定:** 設定の「General」の「Undo history」で **1〜1000** (既定 100) から選びます (`src/lib/UndoLimitSetting.svelte`)。確定するとすぐに
  `set_undo_limit` で開いているドキュメントに当て (超えた古い履歴は捨てる)、アプリの設定のディレクトリの `editor-settings.json`
  (`{"undo_limit": 100}`) に書きます。エディタ全体の設定で、プロジェクトには残しません。起動時にこのファイルを読み (無い・読めない・範囲の外なら既定の 100)、
  プロジェクトを開く・作る・開き直すときは前のドキュメントの上限を引き継ぎます (`src-tauri/src/project.rs` の `replace_document`)。
  0 (履歴なし) は誤った編集を戻せなくなるので選ばせません。上限は 1000 までです (1 状態が数 MB のモデルで数 GB にならないように)。範囲の外は
  `editor.invalid_undo_limit` のエラーです。実装は `src-tauri/src/history.rs`。
- **戻れる・進めるか:** `Document::can_undo`・`can_redo` (と続けてできる数 `undo_count`・`redo_count`) で分かります。Tauri の `history_status` が
  これと上限を `HistoryStatus` で返し、画面は読み直し (`EditorDocument.refresh`。編集・Undo・Redo・エージェントの変更の後) のたびに取り直します。
  ヘッダーの「Undo」「Redo」のボタンは、戻れない・進めないときは押せません (ショートカットは押しても何もしない)。
- **上限を超えたときの画面:** 知らせません。最も古い状態まで戻った後は、履歴が空のときと同じく「Undo」のボタンが押せなくなります (`undo` は `false` を返す)。
- **未保存の印:** 保存した状態を履歴から捨てても `is_modified()` は正しいままです (状態の番号で比べるので、捨てた状態に戻れなくなるだけで「変更あり」のまま)。
- **メモリの目安:** スナップショット 1 つは、テクスチャの画像を除いたモデル・物理・モーションと表情の複製です (画像は `Arc` で共有)。
  実際の PSD (`assets/fixtures/psd-real/` の 25 メッシュのもの) を取り込んだだけのモデルで 1 状態あたり約 53 KB、上限まで積んで約 5 MB です。
  キーフォームの格子はメッシュの頂点をセルの数だけ持つので、リギングが進むとほぼセルの数に比例して増えます。
- **数値の欄の続けての確定は 1 つの Undo:** 数値の欄 (`@ugoe/ui` の `NumberField`) は矢印キー・PageUp / PageDown の 1 回ごとに確定するので、
  押し続けても 1 回ごとに Undo が積まれないよう、**同じ欄で、同じ種類のコマンドを、前の確定から 1 秒以内 (`COALESCE_WINDOW`) に続けて確定したもの**を
  1 つの Undo にまとめます (T0695)。1 回の Undo で、まとまりの最初の確定の前の値に戻ります。間隔は前の確定から測るので、
  1 秒より長く押し続けても 1 つです。欄に入れて Enter・フォーカスを外して確定したものも同じ欄の確定です。
  - **まとめないとき:** 1 秒より長く空いた、別の欄・別の種類のコマンド、間にほかの編集 (ほかのパネル・ビューポート・エージェントの変更)・
    Undo・Redo・保存・本体の読み直しが入った。拒否された確定はまとまりを切りません。スライダーのつまみ・ビューポートのドラッグ・ボタンは、
    もともと 1 操作で 1 コマンドなのでまとめません。
  - **場所:** 印はフロントエンドが付け、まとめるのは editor-core です。`NumberField` は `onchange` を呼ぶ間だけ欄ごとの印を
    `numberFieldCommit()` で返し (docs/ui.md#数値の部品と-onchange)、`EditorDocument` の `send`・`apply` は `onchange` の中で呼ばれたら
    `<欄の印>:<コマンドの op>` を `apply_command` の `coalesce` に付けます。Tauri は受けた時刻と合わせて `Document::apply_coalescing` を呼び、
    印と時刻が続いていて、前のまとめる適用で作った状態のままなら、前の状態を Undo に積みません。
  - **IPC も溜めない:** 列 ([パネルからの編集](#パネルからの編集)) の末尾にまだ始まっていない同じ欄の確定があれば、新しく並べずに
    その組み立てを置き換えます。押し続けても、適用と読み直しは「いま走っている 1 つ」と「待っている 1 つ」だけです。
- **検証の結果の記憶:** コマンドの適用では、適用後のモデルと物理を検証し、適用前になかった問題があれば拒否します。適用前の状態の検証の結果は
  状態ごとに覚え (適用で作った状態は、そのときの適用後の検証の結果を持つ)、同じ状態を 2 度検証しません。

### パネルからの編集

フロントエンドの `EditorDocument` (`apps/editor/src/lib/document.svelte.ts`) は、編集を 1 本の列に並べて 1 つずつ送ります (T0693)。

- **列:** `send`・`apply`・`applyWith`・`undo`・`redo` は、前に並べた編集の適用と読み直し (`refresh`) が終わってから始まります。
  前の編集が失敗しても次へ進みます。並べてまだ終わっていない数は `EditorDocument.pending` で、パネルは並べ替え・削除のように
  今の並びや番号に頼るボタンを、これが 0 でない間は無効にします。数値の欄・選択などの入力は無効にしません (確定して次の欄へ
  移った後の編集を、列に並べて送る)。
- **送る直前に組み立てる:** `send(build)` は、自分の番が来て前の編集を読み直した後に `build` を呼び、その時点の最新の写し
  (`physics`・`model` など) からコマンドを組み立てます (null なら送らない)。リスト・ベクトル・要素を丸ごと置き換えるコマンド
  (`set_physics_particles`・`set_physics_world`・`set_physics_collider`・`set_glue`・`set_pose`・`set_depth` など) は、画面が持つ
  写しから組み立てると、まだ読み直していない前の編集を戻してしまうので、`send` で組み立てます。対象は ID (番号しかないものは番号)
  で覚えておき、`build` の中で最新の写しから探し直します (消えていれば送らない)。
- **目印が変わる編集の後:** スロットの名前 (ID) やキーの時刻のように、編集で目印そのものが変わるときは、送った変更
  (前の ID → 新しい ID、前の時刻 → 新しい時刻) をパネルが覚え、後に並べた編集は `build` の中でそれをたどって今の目印を引きます
  (確定したときの画面の選択はまだ前の目印を指しているため。送っている編集が無くなったら忘れる。T1612)。キーの時刻の変更は
  モーションと行ごとに `TimelineState.keyMoves` が覚え、ドープシートとカーブのグラフ (キー・取っ手のドラッグ、欄の確定、
  区間の種類、選んだキーの削除) が共有します (T1623・T1628)。グルーの組の削除は
  番号でなく組の頂点 (A と B の番号) で指します。
- **成否:** 成否は戻り値で判定します (`send`・`apply`・`applyWith` は失敗したら null)。`EditorDocument.error` は読み直しの失敗でも立ち、
  前の失敗が残っていることもあるので、成否の判定には使いません (失敗の理由を出すときに読む)。

## モーションと表情の編集

モーションと表情 ([spec/motion.md](spec/motion.md)) は `Document` が持ち (Undo の対象)、次のコマンドで編集します (実装は `crates/ugoe-editor-core/src/motion_edit.rs`、テストは `motion_edit_tests.rs`)。

- **指定の仕方:** モーション・表情は名前 (`name`)、カーブは対象 (`target`、`{"kind": "parameter", "id": "AngleX"}`)、キー (カーブの点) は時刻 `t` で指定します (完全に一致する時刻。JSON の `f32` の値はそのまま往復する)。
- **名前:** プロジェクトのファイル名 (`motions/<name>.motion.json`) になるので、テクスチャ名と同じ規則 (`[A-Za-z0-9_.-]`、`.` で始まらない、128 バイト以下) で、大文字小文字を区別せずに一意にします (`InvalidName`・`DuplicateId`)。
- **対象:** 新しいカーブ・表情の項目の対象はモデルにあること (パラメータ・パーツ・エフェクトグループ `EyeBlink` / `LipSync`。なければ `NotFound`)。同じ対象のカーブは 1 つ (`DuplicateId`)、表情の同じパラメータも 1 回だけ。
- **検証:** 適用の後、編集したモーション・表情を検査し、**適用前になかった問題が生じたら拒否します** (`InvalidMotion` / `InvalidExpression`。モデルの検証と同じ考え方で、読み込んだ時点で不正なデータも問題を増やさない編集はできる)。
  モーションの問題: 長さが正の有限の値でない、フェードが負か有限でない、先頭が `start` でない (か途中に `start`)、未知のセグメント・対象、キーの時刻が増え続けない (同じ時刻も不可)、値が有限でない、ベジェの取っ手の時刻が区間 `[p0.t, to.t]` の外、イベントの時刻が負。
  表情の問題: フェードが負、値が有限でない、未知の op、同じパラメータの重複。

| `op` | 内容 |
| --- | --- |
| `add_motion { motion }` | モーションを末尾に加える (完全な値) |
| `remove_motion { name }`・`rename_motion { name, new_name }` | 削除・名前の変更 |
| `set_motion_properties { name, duration?, looping?, fade_in?, fade_out? }` | 指定したフィールドだけを変える |
| `add_motion_curve { motion, curve }`・`remove_motion_curve { motion, target }` | カーブの追加 (完全な値)・削除 |
| `set_motion_curve_fade { motion, target, fade_in?, fade_out? }` | カーブ個別のフェード (`null` ならモーションの値) |
| `set_motion_key { motion, target, point }` | 時刻 `point.t` にキーを打つ (下) |
| `move_motion_key { motion, target, t, to }` | キーを `to` へ動かす (下) |
| `remove_motion_key { motion, target, t }` | キーを消す (下) |
| `set_motion_segment { motion, target, t, segment }` | 時刻 `t` のキーで **終わる** 区間の種類: `{"type": "linear" \| "stepped" \| "inverse_stepped"}` か `{"type": "bezier", "c1"?, "c2"?}` (下) |
| `set_motion_events { motion, events }` | イベントの一覧を置き換え、時刻の順 (同じ時刻は指定の順) に並べる |
| `add_expression { expression }`・`remove_expression { name }`・`rename_expression { name, new_name }` | 表情の追加 (完全な値)・削除・名前の変更 |
| `set_expression_properties { name, fade_in?, fade_out? }` | フェード |
| `set_expression_parameters { name, parameters }` | パラメータの一覧 (`parameter`・`value`・`op`) を置き換える |

**キーを打つ** (`set_motion_key`)。カーブがなければ、そのキー 1 つのカーブを作ります。その時刻にキーがあれば値だけを変えます (隣のベジェのこのキーの側の取っ手も同じだけ動かす)。なければ:

- 最初のキーより前: 新しい `start` にし、元の最初のキーへは最初の区間と同じ種類でつなぐ。
- 最後のキーより後: 最後の区間と同じ種類で加える (区間がなければ直線)。
- 区間の中: 区間を同じ種類の 2 つに分ける。ベジェは、その時刻の曲線上の点で de Casteljau 法で分け、新しいキーの側の 2 つの取っ手を、打った値と曲線の値の差だけ上下にずらす (曲線の上に打つと形が変わらない)。
- 新しくできたベジェの区間の取っ手 (分けたときを除く) は、区間の 1/3 と 2/3 の時刻に両端の値を置いたもの (ゆっくり始まりゆっくり終わる)。

**キーを動かす** (`move_motion_key`)。隣のキーを越える (同じ時刻を含む) 移動は拒否します。前後の区間がベジェなら、このキーの側の取っ手 (入る区間の `c2`・出る区間の `c1`) を同じだけ動かし、前後の区間のすべての取っ手の時刻を区間の中に収めます。

**キーを消す** (`remove_motion_key`)。最初のキーなら次のキーを `start` にします。途中のキーなら前後の区間を、消したキーに **入る** 区間の種類で 1 つにします (ベジェなら入る区間の `c1` と、出る区間がベジェならその `c2`、でなければ既定の取っ手)。最後の 1 つのキーならカーブを消します。

**区間の種類** (`set_motion_segment`)。最初のキーには入る区間がないので拒否します。ベジェの取っ手を省略すると、今の区間がベジェならその取っ手を保ち、そうでなければ既定の取っ手にします。取っ手は区間の外なら拒否します (画面は収めてから送る)。

### タイムライン

画面はビューポートの下のパネル (`src/lib/panels/timeline/`) です。「Motions」「Expressions」のタブを持ち、畳める (開閉は[覚える値](#覚える値)。空のプロジェクトと狭い窓では自動で畳む。[画面の配置](#画面の配置))。操作はすべて上のコマンドを送るだけで、表示の状態 (選んだモーション・再生位置・再生中・選んだ行とキー・表情のプレビュー) は `TimelineState` (`timeline.svelte.ts`) にあり、Undo の対象ではありません。

| 場所 | 操作 |
| --- | --- |
| 上の段 | モーションの選択・「+ New motion」(`add_motion`、長さ 2 秒)・「Settings…」(Popover。名前 (`rename_motion`)・長さ・フェード・「Loop」(`set_motion_properties`))・「Events」(Popover。`set_motion_events`。「+ Event at … s」で再生位置に足す)・「Delete」 |
| 再生の段 | 再生 / 停止 (アイコン)・繰り返し (押したままのアイコン。最後で折り返す。モーションの `looping` でも折り返す)・前のフレーム・時刻の欄・次のフレーム・「/ 長さ」・「◆ Key changed」・「Delete key」 |
| 行 (ドープシート) | モデルのすべてのパラメータと、パラメータ以外のカーブ (パーツの不透明度・エフェクトの強さ。モデルにないパラメータのカーブは `(missing)`)。行の「◆」は今のスライダーの値でキーを打つ (`set_motion_key`)。キーの印は押すと選んで再生位置をその時刻にし、左右にドラッグすると時刻を変える (`move_motion_key`、1 フレームに揃える)。印の形はそのキーに入る区間の種類で、直線と最初のキーは菱形、ベジェは円、ステップと逆ステップは四角 (`timeline.ts` の `shapedKeys`。色に頼らない)。目盛りの押す・ドラッグでシーク (Shift で揃えない) |
| カーブのグラフ (`CurveGraph.svelte`) | 選んだ行のカーブ。ベジェは (時刻, 値) の平面の 3 次ベジェなので SVG の `C` で描く。左端に上端と下端の値、既定値は破線。キーの円のドラッグ (`move_motion_key`)、取っ手の四角のドラッグ (`set_motion_segment` の `c1` / `c2`、区間の中に収める)、ダブルクリックでキー。左の欄は 4 行: 行の名前と「Delete curve」、選んだキーの時刻・値の欄、入る区間の種類 (`Select`。説明は Tooltip)、カーブのフェード (`set_motion_curve_fade`) |
| キーボード | パネルの中 (入力欄の外) で Space: 再生・停止、← / →: 1 フレーム、Delete / Backspace: 選んだキーを消す |

- **並びと高さ (T0675):** 上の段と再生の段は 1024px の窓 (中央の列 460px ほど) でも 1 行ずつに収め、送らずに残します。行の表を送り
  (2 行は残す)、グラフはその下に出します。1024×720 の窓でタイムラインとグラフを出しても、2 つの段・2 行の表・グラフと左の欄が送らずに見えます。
  それでも収まらない低い画面 (200% の拡大) だけはパネルの全体を送ります。行の名前の列は表とグラフで同じ幅 (`--label-width`。時刻の軸をそろえる) で、
  中央の列が狭いときは 150px まで縮めます。
- **「◆ Key changed」:** スライダーの値が、その時刻のカーブの値 (カーブがなければ既定値) と違うパラメータすべてに、今の値でキーを打つ (値域の 1e-4 倍までは同じとみなす)。複数なら 1 つの `batch` (Undo の 1 単位)。
- **フレーム:** キーの時刻は 30 fps のフレームに揃えます (`snapToFrame`)。時刻の欄とキーの欄に数を入れれば、揃えない時刻も使えます。
- **ドラッグ中** は表示だけを変え (ドープシートのひし形・グラフの曲線)、離したときに 1 つのコマンドを送ります。
  コマンドは `doc.send` で送る直前のカーブから組み立てます。離したときの表示がまだ前の移動を読み直していなくても、
  送った時刻の変更をたどって今の時刻のキー (取っ手ならそのキーで終わる区間) を動かし、行き先は表示で離した位置にします
  (キーの時刻を動かさなかったドラッグは今の時刻のまま。[パネルからの編集](#パネルからの編集)。T1623)。
- **選んだキーへの編集:** 区間の種類のボタン (`set_motion_segment`)・「Delete key」と Delete キー (`remove_motion_key`) も
  `doc.send` で送る直前のカーブから組み立て、選択の時刻から `keyMoves` をたどった今の時刻のキーを指します (時刻の移動を
  送っている間に押しても、動かした後のキーに効く)。キーが無くなっていれば送りません。区間の種類が既に同じなら送りません。
  カーブのフェードも送る直前のカーブから組み立てます (続けて確定したもう一方のフェードを戻さない。同じ値なら送らない)。T1628。
- **ダブルクリックのキー・行の「◆」・「◆ Key changed」** は、表示の位置 (再生位置) とスライダーの値という決まった時刻と値に
  キーを打つ (`set_motion_key`) だけで、既にあるキーの時刻を指さないので、`doc.apply` のまま送ります (列の順は保たれる)。

**再生とシーク:** `TimelineState` は、選んだモーションの各カーブをその時刻で評価した値 (wasm の `MotionSampler`、`ugoe_motion::curve::evaluate` そのもの。フェードなし) を、スライダーと同じ `ViewportController.setParameter` で書き込みます。再生は `requestAnimationFrame` の経過時間で再生位置を進めます (`advance`)。

- `MotionSampler` はカーブごとの区間の索引を持ちます。`TimelineState` は 1 つを持ち続け、評価するモーションが前と違うとき (モーションを選び直したとき・編集や Undo の後。ドキュメントは編集のたびにモーションの配列を新しいオブジェクトで読み直すので、オブジェクトが違えば中身が変わったとみなす) だけ `set_motion_json` で渡し直します。再生中のフレームは `sample(t)` だけを呼び、モーションの JSON を毎回作りません。渡し直しに失敗したら (wasm は前のモーションを保つ) 値を書き込まず、次の評価でまた渡し直します (古い索引の値を返さない。`timeline-state.test.ts`)。

- WebPuppet のモーションの再生 (`start_motion_json` と `update`) は使いません。エディタは編集のたびにモデルを読み直し、そのとき WebPuppet の再生状態は初めからになる (T0309) ためです。この方式なら読み直しても再生位置は保たれ、同じ時刻は常に同じ姿になります。
- そのため **プレビューはカーブの値だけ** で、フェード・目パチ・呼吸・物理・パーツの不透明度とエフェクトの強さのカーブは掛かりません (後続の候補)。
- モーションを選んだり開いたりしただけではスライダーに書き込みません (リギング中の値を壊さないため)。シーク・再生・キーの操作の後だけ、編集 (と Undo) のたびに今の再生位置の姿を書き込み直します (`TimelineState.engaged`)。
- 純粋な関数 (時刻とピクセルの変換・パス・行の一覧・キーのコマンドの組み立て・名前) は `timeline.ts` にあり、Vitest で検査します。

### 表情

「Expressions」のタブ (`ExpressionEditor.svelte`)。

| 操作 | 内容 |
| --- | --- |
| 「New from sliders」 | 既定値から (値域の 1e-4 倍より大きく) 動かしたパラメータで表情を作る (`add_expression`、名前は `Expression`、`Expression_2`…、フェード 0.5 秒)。値は選んだ op で、既定値に重ねると今の値になるように: `add` は `今 − 既定`、`multiply` は `今 ÷ 既定` (既定値が 0 なら `add` にする)、`overwrite` は `今` |
| 「Preview」/「End preview」 | 今のスライダーの値に表情を重み 1 で重ねて見せ (`applyExpression`。ランタイムの `ExpressionManager` と同じ式)、もう一度押すと重ねる前の値に戻す。表情を編集するとプレビューを作り直す |
| 表 | パラメータごとの op と値 (`set_expression_parameters`)、「×」で外す、「Add parameter…」で足す (`add`、今の値と既定値の差)、「Set values from sliders」で今の値から作り直す (op は保つ) |
| 上の段 | 「Settings…」(Popover。名前 (`rename_expression`)・フェード (`set_expression_properties`)。モーションと同じ)・「Delete」 |

- プレビュー中の「New from sliders」「Set values from sliders」は、重ねる前の値を基準にします。
- 純粋な関数は `expressions.ts` (Vitest)。

## 物理の編集

物理の設定 (`PhysicsSettings`、[spec/physics.md](spec/physics.md)) を編集するコマンドです (実装は `crates/ugoe-editor-core/src/physics_edit.rs`、テストは `physics_edit_tests.rs`)。
物理はモデルの外のデータですが、`Document` がモデルと一緒にスナップショットに持つので、同じ Undo の単位で変わります (`Document::physics()`・`with_physics`)。
リグと衝突体は ID (`rig`・`id`) で指定します。入力・出力・粒子は ID を持たないので、並びごと置き換えます。

| `op` | 内容 |
| --- | --- |
| `set_physics_world { gravity, wind, fps }` | 重力・風・ステップの周波数。物理の設定がなければ作る (リグなし) |
| `add_physics_rig { rig }` | リグを末尾に追加する。物理の設定がなければ既定値 (重力 `(0, 1)`・風なし・60 Hz) で作る。使用中の ID は `DuplicateId` |
| `create_physics_rig { id?, name, deformers }` | 回転デフォーマの鎖 (根元から順) に沿った動的物理のリグを作って追加する (下) |
| `remove_physics_rig { id }`・`rename_physics_rig { id, name }` | リグの削除・表示名の変更 |
| `reorder_physics_rig { id, before }` | リグを `before` の直前へ移す (`null` なら末尾)。リグは並びの順に処理されるので、中継 ([spec/physics.md#リグの順序と中継](spec/physics.md#リグの順序と中継)) の結果が変わる。見つからないリグは `NotFound`、自分自身の前は `SelfReference` |
| `set_physics_inputs { rig, inputs }`・`set_physics_outputs { rig, outputs }`・`set_physics_particles { rig, particles }`・`set_physics_normalization { rig, normalization }` | リグの入力・出力・粒子の鎖・正規化の範囲を置き換える |
| `set_physics_rig_colliders { rig, colliders }` | リグと衝突する衝突体の ID の列 (重複は拒否) |
| `set_physics_frame { rig, frame? }` | 重力と風の基準のデフォーマ (省略でキャンバス) |
| `add_physics_collider { collider }`・`set_physics_collider { collider }`・`remove_physics_collider { id }` | 衝突体の追加・置き換え (`collider.id` のもの)・削除 (リグとスロットの `colliders` からも外す。[削除の連鎖と拒否](#削除の連鎖と拒否)) |

- **検証:** `Document::apply` は、適用後の物理を `validate_physics` で検査し、適用前になかったエラーが生じたら `EditError::Invalid` で拒否します (存在しない参照先・種類の違うデフォーマ・頂点の重みの数・負の半径など。モデルの検証と同じ規則)。
  `validate_physics` が見ない値はコマンドが検査し、`EditError::InvalidPhysics` で拒否します: 有限でない値、`fps` が 0 以下、粒子が 0 個、`mobility` が 0〜1 の外、`delay`・`radius`・`collision_radius` が負、出力の `particle` が 0 (根元)、正規化の範囲が `min <= default <= max` でない、空の ID。
- **未知の値:** 新しい版のファイルから読んだ未知の値 ([spec/file-formats.md#未知の値の読み込み](spec/file-formats.md#未知の値の読み込み)) は、どの種類も `EditError::InvalidPhysics` で拒否します
  (入力・出力の `source`、出力先 `target`、頂点への出力の対象 `owner`、出力の `blend`、衝突体の `shape`。`add_physics_rig`・`set_physics_inputs`・`set_physics_outputs`・`add_physics_collider`・`set_physics_collider` のどれでも)。
  未知の値は書き出せず保存も拒否される (`FormatError::UnknownValues`) ので、編集で作らせません。ほかのコマンド (モーション・表情・アバターのプロファイル・格子の補間と推定) が未知の値を拒否するのと同じ扱いです。
  ファイルに既にある未知の値は、それを置き換えるコマンドを適用しない限り残ります (モデルの要素を消しても外さない)。置き換えるコマンドでは、未知の値を含まない列を渡します。
- **モデルの編集との関係:** どのコマンドの後でも、そのコマンドが消したパラメータ・デフォーマ・メッシュ (`unwrap_deformer` を含む) を参照する入力・出力を削除し、重力の基準を外します。
  取り付け先のデフォーマが消えた衝突体は取り付けを外し、形状の位置を消す前の基準の姿でのキャンバス座標に写します (見た目の位置を保つ)。
  `add_mesh_vertex` は、そのメッシュへの頂点の出力の重みの末尾に 0 を足し、`remove_mesh_vertices` は消した頂点の重みを除きます。
  頂点の数を変えるほかの編集 (`regenerate_mesh` など) は、重みの数が合わなくなるので検証で拒否します (先に出力を消す)。
- **鎖からリグを作る (`create_physics_rig`):** `deformers` は回転デフォーマだけ (`DeformerKindMismatch`)、1 つ以上、重複なし。基準の姿での各デフォーマの原点を `o_k` とし、粒子 0 は最初のデフォーマの局所座標 `(0,0)` (根元はそのデフォーマに取り付く)、粒子 k (1 ≤ k < n) は `o_{k+1} - o_1`、粒子 n は最後のデフォーマの原点から重力の向きに、その子孫の点 (メッシュの頂点・ワープの制御点・回転デフォーマの原点) の最も深い所まで進んだ点 (1px 未満ならキャンバスの高さの 10%) です。鎖が重力の向きに並ぶので、静止姿勢は初期位置と一致し、出力の角度は 0 です。
  `radius` は隣の粒子との距離、係数は `mobility` 0.9・`delay` 0.9・`acceleration` 1.5 (根元は 1・1)。出力は粒子 k の `angle` を k 番目のデフォーマの角度に足す (`deformer_angle`、倍率 1・重み 100%)。入力はなく、正規化の範囲は -10〜10。
  `name` が空なら最初のデフォーマの名前、`id` を省略すると名前 (英数字を含まなければ `Physics` + 最初のデフォーマの ID) から `ids::unique_id` で作ります。

### 物理のテンプレート

`apply_physics_template { template, options }` (T0634) は、定番のリグの組を作ります (実装は `crates/ugoe-editor-core/src/physics_template.rs`、テストは `physics_template_tests.rs`)。
`physics_template::commands` がコマンドの列 (`add_parameter`・`add_physics_rig`・`reorder_physics_rig`・`set_physics_*`) を作り、`Document::apply` がそれを `batch` と同じく 1 つの Undo 単位で適用します (途中で失敗すれば何も変えない)。
AI や CLI は `commands` の結果を確かめてから `batch` として適用することもできます。

| `template` | 作るリグ | 中身 |
| --- | --- | --- |
| `body_follow` | `BodyFollowX` / `Y` / `Z` | `AngleX/Y/Z` → 角度 (正規化の範囲 = 入力のパラメータの値域。値 = 根元の角度 (度)) → 粒子 1 の角度 × 0.3 を `BodyAngleX/Y/Z` へ。係数 `mobility` 0.65・`delay` 0.5・`acceleration` 1.5 (0.1 秒ほど遅れ、1 割ほど行き過ぎ、1 秒ほどで収まる) |
| `bounce` | `BodyBounce` → `BodyBounceAngle` | 1 段目: `AngleX` → x → 粒子 1 の x を中継パラメータ `BodyBounce` へ (上書き、よく揺れる係数 0.95・0.8・1.0)。2 段目: `BodyBounce` → x (反転) → 粒子 1 の x × 0.4 を `BodyAngleX` へ |
| `neck_follow` | `NeckFollow` | `AngleZ` → 角度、`AngleX` → x (重み 50%) → 粒子 1 の角度 × 0.3 を首の回転デフォーマの角度へ (`deformer_angle`。動的物理になる) |

- **合成 (`options.blend`):** 体の回転への出力の合成の方法。既定の `add` は、トラッキング・モーションが書いた値に「振り子の角度 − 静止時の角度」を足すので、遅れと行き過ぎだけを加えて最後は元の値に戻ります ([spec/physics.md#合成の方法](spec/physics.md#合成の方法))。
  トラッキングが体の回転を頭の回転 × 0.3 にしている (体のトラッキングが無いとき。[avatar-profile.md#頭と体](spec/avatar-profile.md#頭と体)) と、体の追従の結果は振り子の角度 × 0.3 になり、`overwrite` と同じ動きです。体の回転を書かないホストでは `overwrite` を選びます。中継パラメータへの出力は常に `overwrite` です。
- **ないパラメータ (`options.create_missing`):** `true` なら標準の値域 (`ugoe_model::standard::range`) で `add_parameter` を先に置きます。`false` (既定) なら、それを使う組 (体の追従は軸ごと、弾みは 2 つのリグ一緒、首は 1 つ) を作りません。作る組が 1 つもなければ `InvalidPhysics` で拒否します。
  中継パラメータ `BodyBounce` (値域 -10〜10) はこの選択によらず、なければ内部用 (`internal`) で作ります。
- **首 (`options.neck`):** 回転デフォーマの ID。省略すると、ID か名前に `neck` (大文字小文字を問わない) か `首` を含む最初の回転デフォーマです。なければ `InvalidPhysics`、回転デフォーマでなければ `DeformerKindMismatch`。
- **同じ ID のリグ:** 並びの位置を保って中身 (名前・入力・出力・粒子・正規化の範囲) を置き換え、重力の基準と衝突体を外します。もう一度適用すると初期値に戻ります。
- **並び:** 新しいリグは末尾に足します。ただし、そのリグが書くパラメータを読む既存のリグがあれば、その最初のものの直前に置きます (中継が同じステップで伝わるように。[spec/physics.md#リグの順序と中継](spec/physics.md#リグの順序と中継))。

### 物理の画面での操作

| 場所 | 操作 |
| --- | --- |
| 物理のパネル (`panels/physics/PhysicsPanel.svelte`) | 右の列の上の区画の「Physics」タブ (「Inspector」と切り替える)。「Preview」/「Stop preview」、「World」(重力・風・周波数 → `set_physics_world`)、リグの一覧 (上から処理する順。▲▼ で並べ替える → `reorder_physics_rig`。前のリグの出力を入力に読むリグには「relay」の印) と「+ Rig」(空のリグ → `add_physics_rig`)、「+ From selected deformers」(ツリーで選んだ回転デフォーマを親から子の順に並べて `create_physics_rig`) |
| テンプレート (`panels/physics/TemplateSection.svelte`) | 物理のパネルの「Templates」。「Create missing parameters」(`create_missing`、既定で入)・「Body blend」(`blend`)・「Neck」(首の回転デフォーマ。既定は名前から選ぶ)、「+ Body follow」「+ Bounce」「+ Neck follow」で `apply_physics_template` を 1 つ送り、作ったリグを選ぶ。ないパラメータを作るか飛ばすかを横に出す。説明とパラメータの確認は `templates.ts` (Vitest) |
| リグの編集 (`panels/physics/RigEditor.svelte`) | 名前、「Gravity frame」、「Delete rig」、粒子 (「Length」`radius`・「Sway」`mobility`・「Speed」`delay`・「Settle」`acceleration`・「Hit r」`collision_radius`・位置。「+ Particle」は最後の区間と同じ向き・長さで足す)、入力、出力 (種類・書き込み先・粒子・成分・倍率・重み・反転・パラメータへの出力の合成の方法「Overwrite」/「Add」([physics.md#合成の方法](spec/physics.md#合成の方法))。頂点の出力は作ったとき重みがすべて 1)、「Collides with」、「Input range」。欄を確定するたびにコマンドを 1 つ送る。数値の欄は物理の検証 ([model.md#物理の検証](spec/model.md#物理の検証)) の範囲に収める (`mobility` は 0〜1、`radius`・`delay`・`acceleration`・`collision_radius` は 0 以上、「Input range」は `min <= default <= max`。`physics.ts` の `PARTICLE_BOUNDS`・`normalizationBounds`) |
| 衝突体 (`panels/physics/ColliderSection.svelte`) | 「+ Circle」「+ Capsule」(主な選択がデフォーマならその原点 (ワープは中心) に取り付け、なければキャンバスの中央)。名前・「Attach to」・「Shape」(どちらも今の見た目の位置を保って写し直す)・位置・半径・削除。モジュールのプロジェクトで、スロットが公開する本体の衝突体が本体に無い (本体に物理が無い) ときは、見出しの下に案内を出す (`ModuleStatus.unavailable_colliders`、`moduleProject.ts` の `unavailableCollidersNotice`。T0660) |
| ビューポート (`viewport/tools/physics/PhysicsOverlay.svelte`) | 物理のパネルを開いている間、衝突体 (青) と粒子の鎖 (オレンジ、根元は四角。選んだリグは濃い色) を重ねる。粒子は動的物理のリグのときだけ (物理空間がキャンバス座標なので。パラメータにだけ出力するリグは出さない)。衝突体をドラッグで動かし (カプセルは端の丸で端だけ)、離したときに `set_physics_collider` を 1 つ送る。鎖を押すとそのリグを選ぶ |

- 衝突体と粒子の位置は wasm の `WebPuppet.deformer_point(id, x, y)` (デフォーマの局所座標 → キャンバス座標) で置き、ドラッグの逆写像は `deformer_jacobian` を使ったニュートン法です (`viewport/tools/physics/physicsGeometry.ts`、Vitest)。
  止めているときの粒子は初期位置 (根元は取り付け先のデフォーマで写し、ほかは根元からの相対位置)、プレビュー中は `WebPuppet.physics_particles(rig)` の今の位置です。
- パネルの組み立て (新しいリグ・衝突体・粒子の足し引き・出力先) は `panels/physics/physics.ts` の純粋な関数で、Vitest で検査します。

### 物理のプレビュー

`ViewportController.startPhysicsPreview()` / `stopPhysicsPreview()` (`viewport/controller.svelte.ts`)。

- プレビュー中は `requestAnimationFrame` ごとに `WebPuppet.update(dt)` (1 フレーム最大 0.1 秒) を呼んで描き、`animationFrame` を増やしてオーバーレイに読み直させます。スライダー (`set_parameter`) は `Puppet::update` の基準値になるので、動かすと根元が動いて揺れます。
- WebPuppet の段は、プレビュー中は物理だけを有効にし、それ以外 (目パチ・呼吸・リップシンク・ポーズ) は常に止めます (モデルを読み直すたびに設定し直す)。
- 始めるときと止めるときに `reset_physics` で粒子を静止姿勢に戻します。止めると物理の段を止め (動的物理の加算量が 0 に戻る)、物理が書き込んだパラメータをスライダーの値に戻すので、編集中の姿に戻ります。
- プレビュー中に編集すると、モデルと物理を読み直し、揺れは静止した姿からやり直します。

### 物理の警告

モデルだけを見る `validate_physics` (エラー。[物理の編集](#物理の編集)) では分からず、モデルを評価して初めて分かる問題を警告として
知らせます (T0315)。今は「静止姿勢で衝突体に入っている鎖」だけです
([physics.md#静止姿勢で衝突体に入っている場合](spec/physics.md#静止姿勢で衝突体に入っている場合))。警告は編集を拒否せず、読むだけでコマンドも発行しません。

- **検出** (`ugoe_editor_core::physics_check::rest_pose_collisions(model, physics)`): モデルをコンパイルし、パラメータの既定値で
  `PhysicsSystem::rest_pose_collisions` を呼んで、当たっている区間をリグと衝突体の組ごとにまとめます
  (`RestPoseCollision { rig, collider, particles }`。`particles` は当たっている区間の子の粒子の添字の昇順。並びはリグの順、同じリグの中は最初に当たる粒子の順)。
  動的物理を使わない設定 (パラメータにだけ出力する) では空です。モデルをコンパイルできなければエラー。
- **`ugoe validate`** も同じ関数を使い、``warning: physics rig `Hair` hits collider `Shoulder` in its rest pose (…)`` を標準エラーに出します
  (終了コードと `ok` は変えない。検証エラーがあるときは評価できないので出さない)。物理はバンドルのもの、または `--physics <PATH>` の JSON。
- **Tauri コマンド** (`src-tauri/src/physics_check.rs`): `check_physics()`。物理の設定がなければ空。
- **画面** (`panels/physics/RestPoseWarnings.svelte`、表記は `panels/physics/restPose.ts` で Vitest): ドキュメントが変わるたびに
  (`EditorDocument.revision`) 問い合わせ直し、当たっている組を 1 行ずつ (`前髪 (Hair) hits Shoulder` と区間の番号) 出します。
  - 「Verify」のパネルの「Physics」: 当たりがなければ「No rig hits a collider in the rest pose.」。行を押すと「Physics」のタブに移り、
    そのリグと衝突体を選びます (`PhysicsView.rig` / `collider`。ビューポートに衝突体と鎖が出る)。
  - 物理のパネル: 当たりがあるときだけ、「Preview」の下に同じ一覧を出します。行を押すとそのリグと衝突体を選びます。
- **取り込み・テンプレートの直後に知らせるか:** 別の通知 (ヘッダーの警告や、`apply_physics_template` の報告) は **足しません**。
  物理のテンプレートの適用・衝突体の移動・「Collides with」の変更はすべて物理のパネルで行い、その直後にパネルの一覧が更新されるので、
  操作した場所でそのまま気づけます。PSD の取り込みは物理の設定を作らず、既存の物理がある場合も、取り込みの後に「Physics」か「Verify」の
  タブを開けば出ます。`ApplyReport` に警告を足すと、すべてのコマンドの後に評価が要るので避けました。

## アバタープロファイルの編集

モデルの `avatar` (アバタープロファイル。準拠レベル・メタデータ・利用許諾・トラッキングの上書き・視線・標準の当たり判定・スロット。
[avatar-profile.md](spec/avatar-profile.md)) と `hit_areas` (当たり判定) を編集し、準拠の検査の結果を出します (T0631。
実装は `crates/ugoe-editor-core/src/avatar.rs`)。

| `op` | 内容 |
| --- | --- |
| `set_avatar_profile { profile }` | プロファイルを丸ごと置き換える。`null` で外す (どのレベルにも準拠しなくなる) |
| `update_avatar_profile { level?, meta?, tracking?, look_at?, hit_areas?, slots? }` | 与えたフィールドだけを置き換える。プロファイルがなければ既定値 (レベル 1、許諾は最も制限の強い値) から作る。`look_at: null` は視線の設定を外し (既定の `EyeBallX` / `EyeBallY` に戻す)、省略は変えない |
| `set_hit_areas { hit_areas }` | 当たり判定を丸ごと置き換える。なくなった当たり判定を指す、プロファイルの標準の当たり判定 (`avatar.hit_areas`) は外す |

**拒否するもの** (`EditError::InvalidAvatarProfile`): モデルによらず不正な値だけです。レベルが 1〜3 の外、許諾・トラッキングの入力の種類が
未知の値 (新しい版のファイルから読んだもの。書き出せない)、トラッキングの `scale` / `offset` が有限でない、視線の可動範囲が 0 以下か有限でない、
標準の当たり判定の名前の重複、当たり判定の ID が空。当たり判定の ID の重複と存在しないメッシュは、適用後の検証 (`validate`) が拒否します。

**拒否しないもの:** パラメータ・当たり判定・スロット・テクスチャへの参照、名前や作者が空であること、主張したレベルに必要なパラメータが
足りないことは、要素を作る順に縛られないように拒否せず、準拠の検査がエラーとして示します。

**連鎖:** `remove_mesh` がメッシュの当たり判定を消すと、それを指す標準の当たり判定も外します ([削除の連鎖と拒否](#削除の連鎖と拒否))。

**準拠の検査** (`ugoe_editor_core::avatar::check(model)`。`ugoe validate --profile avatar` と同じ `ugoe_model::avatar::check::check_profile`):
名乗るレベル (`claimed`。プロファイルがなければ `null`)・満たすレベル (`satisfied`)・エラーと警告の説明を返します。モデルは変えません。
Tauri コマンドは `check_avatar_profile()` (`src-tauri/src/avatar.rs`)。

**画面** (`panels/inspector/avatar/AvatarSection.svelte`、コマンドの組み立てと表記は `avatar.ts` で Vitest):
インスペクタで何も選んでいないときに「Avatar profile」の節を出します。

- プロファイルがなければ「Create avatar profile」で作ります (`update_avatar_profile`。名前はモデルの名前、許諾は最も制限の強い値)。
- 「Level」、メタデータ (名前・作者 (1 行に 1 人)・バージョン・著作権・連絡先・利用規約の URL・原作 (1 行に 1 つ)・サムネイル (テクスチャ))、
  利用許諾 (アバターとして使ってよい人・商用利用・クレジット・改変・真偽値の許諾・第三者のライセンス)、標準の当たり判定 (`Head` / `Body` と
  `hit_areas` の対応) を編集します。欄を離れたときに `update_avatar_profile` を 1 つ送ります (値が変わらなければ送らない)。
  メタデータは丸ごと送るので、1 つの欄の変更が 1 つの Undo の単位です。空にした省略できる文字列は `null` にします。
- 「Conformance」に準拠の検査の結果を出します。ドキュメントが変わるたびに (`EditorDocument.revision`) 問い合わせ直すので、
  パラメータを足しただけでも満たすレベルが変わります。
- トラッキングの上書き・視線・当たり判定そのもの (`set_hit_areas`) は、今は画面では編集しません (コマンドは使えるので、
  MCP サーバや CLI の見本のジェネレータからは設定できる)。プロファイルの `slots` (アプリに見せるスロット) は、スロットのパネルで選びます
  ([スロットの編集](#スロットの編集))。

## スロットの編集

本体のモデルの着せ替えのスロット (`Model::slots`。[modules.md#スロット](spec/modules.md#スロット)) を定義します (T0647、[ADR 0024](adr/0024-module-authoring.md)。
実装は `crates/ugoe-editor-core/src/slot_edit.rs`)。モジュール (衣装) は別のプロジェクトとして作り、スロットの ID だけで取り付け先を指定します。

| `op` | 内容 |
| --- | --- |
| `add_slot { slot }` | スロットを `slots` の末尾に足す |
| `set_slot { id, slot }` | スロット `id` を丸ごと置き換える。`slot.id` が違えば名前を変え、アバタープロファイルの `slots` の参照も付け替える |
| `remove_slot { id }` | スロットを消し、アバタープロファイルの `slots` からも外す |

**拒否するもの:** ID が `[A-Za-z0-9_]+` でない・ほかのスロットと重なる (`EditError::InvalidSlot` / `DuplicateId`)、
公開する衝突体 (`colliders`) が物理の設定にない (`NotFound`。物理の設定がないと適用後の物理の検証が走らないため、コマンドが調べる)、
**描画順の範囲がほかのスロットの範囲と重なる** (`InvalidSlot`。重なるとモジュールのメッシュが互いに入り組む)。端が接するだけ (`300..400` と `400..500`) はよく、
幅 0 の範囲は 1 点でも共有すれば重なるとみなします。読み込んだ時点で重なっていた組は、ほかの項目の編集を妨げないように見逃します
(置き換える前の範囲とも重なっていたスロットとの重なりは拒否しない)。
取り付け先・置き換えるパーツ・パラメータ・マスク・モジュールの親のパーツ・ポーズの要素の参照先と、範囲の値 (有限・`min <= max`) は、
適用後の検証 (`validate`) が拒否します。

**連鎖:** [削除の連鎖と拒否](#削除の連鎖と拒否) のとおり、パラメータ・メッシュ・パーツを消すとスロットのリストから外し、
取り付け先のデフォーマとモジュールの親のパーツは消せません (先にスロットを変える)。物理の衝突体を消すとスロットの `colliders` からも外します。

**画面** (`panels/slots/SlotsPanel.svelte`、組み立ては `slots.ts` で Vitest): 右の列の「Slots」タブ。

- スロットの一覧と「+ Slot」(空いている描画順の範囲を選んで足す)。スロットを選ぶと編集の欄を出します。項目ごとに、何のための設定かを短い説明で添えます。
- 取り付け先 (デフォーマかキャンバス)・モジュールの親のパーツ・描画順の範囲・置き換えるパーツ・公開するパラメータ (標準パラメータは常に使えるので
  出さない)・マスクに使ってよいメッシュ・衝突体・ポーズの要素・アバタープロファイルでアプリに見せるか。欄を離れるかチェックを変えるたびに
  `set_slot` を 1 つ送ります。アプリに見せるかは `update_avatar_profile { slots }` で、プロファイルがなければ選べません。
- 「Draw order」に、本体のメッシュを描画順 (既定の姿) に並べ、スロットの範囲を帯で重ねます。範囲に入る本体のメッシュ (モジュールと前後が
  入り組むもの) が分かります。「Select meshes in range」で範囲の中のメッシュを選び、ビューポートで確かめます。

## プレビューモード

組み上げたモデルがランタイムでどう動くか (物理・目パチ・呼吸・モーション・表情を掛けた仕上がり) を、すべての
パラメータのスライダーで確かめるモード (T0635)。コードは `apps/editor/src/lib/preview/`。

- ヘッダーの「Preview」で入り、同じボタン (「Exit preview」) で出ます。プレビュー中は左の列・タイムライン・右の列
  (インスペクタ・物理・パラメータ) とビューポートの編集のツール (選択・ギズモ・物理の表示) を隠し、右にプレビューの
  パネル (`PreviewPanel.svelte`) を出します。編集のパネルは壊さずに CSS で隠すので、出ると元の状態に戻ります。
- **コマンドは送りません。** Undo / Redo のボタンを隠し、ショートカットも受けません。ファイルメニューは使えますが、
  モデルが変わる操作 (新規・開く・取り込み) の後はプレビューを出ます。
- **動かし方:** `PreviewMode` (`preview.svelte.ts`) を `ViewportController.setDriver` に渡すと、ビューポートは
  `requestAnimationFrame` ごとに `WebPuppet.update(dt)` (1 フレーム最大 0.1 秒) を呼んで描きます
  (`FrameDriver`。`update` の前後に `beforeUpdate` / `afterUpdate`、モデルを読み込むたびに `loaded`)。
  適用順と手で動かした値との関係は [spec/motion.md](spec/motion.md#フレーム内の適用順) のとおりです
  (たとえば、再生中のモーションの対象はスライダーで動かしても上書きされる)。
- **段:** 物理・目パチ・呼吸・リップシンク・ポーズをそれぞれ `set_enabled` で切り替えます。最初はリップシンクだけ止め、
  ほかは有効です (リップシンクは音量を与えると口のパラメータを上書きするため)。リップシンクの音量はパネルの
  スライダー (0〜1) で与え、段を止めると `set_lip_sync_level(undefined)` に戻します。モデル (`effects`・`pose`) や
  物理の設定の無い段には「(not set up)」と出します。物理の「Reset」は `reset_physics`。
- **ポーズ (T0660):** 編集中のモデル (`doc.model`) にポーズのグループがあれば「Pose」の節を出し、グループごとの欄で表示する要素を
  `show_pose` で切り替えます (`PreviewMode.showPose`。クロスフェードはポーズの時間で、毎フレームの `update` が進める)。表示する要素は
  `PreviewMode.poseShown` (グループに 1 つまで。`poses.ts` の `showEntry`) に持ち、入るときは編集中のビューポートで見ていた要素
  (`ViewportController.posePreview`) から始め、モデルを読み直すたびに渡し直します。ポーズの段を止めている間は選べません。
  モジュールのプロジェクトで今の編集を本体に付けて比べている間は、今の編集を別の名前 (`<名前> (editing)`) で付けるので、モジュールの
  パーツの接頭辞を付け替えて渡します (`preview.ts` の `previewPart`)。
  着せ替えで付けたモジュールのグループ (T0661) は、編集中のモデルのグループの後に「`<モジュール>` group N」で並べます。WebPuppet の
  合成したポーズ (`pose_json`。モデルの `pose` の形) から、編集中のモデルのグループ (パーツを `previewPart` で WebPuppet の ID にして、
  要素を 1 つでも共有するもの) を除いたものです (`preview.ts` の `modulePoseGroups`)。付け外し (`DressUp` の `changed`) と読み込みのたびに
  読み直します。選んだ要素は WebPuppet の ID で `PreviewMode.modulePoseShown` に持ち、読み込みのたびに渡し直します (外したモジュールの
  パーツは `show_pose` が `false` を返すだけで、付け直すと選んでいた要素に戻る)。このグループがあれば、モデルにポーズが無くても
  「Pose」の段に「(not set up)」を出しません。本体の要素に追従する衣装の差分は、本体の要素と一緒に切り替わります。
  モーションがパーツの不透明度のカーブで要素を切り替えると、欄も追従します (T0736): `PreviewMode.afterUpdate` が毎フレーム
  表示中の要素 (`pose_visible_json`) を読み、前のフレームから変わったときだけ `poseShown`・`modulePoseShown` を合わせます
  (`preview.ts` の `followShown`。編集中のモデルの要素は `previewPart` で WebPuppet の ID にして突き合わせる)。選んだ要素は
  `show_pose` の直後から、モーションが要求した要素は次の `update` から表示中になるので、後から来たほうが欄に出ます (モーションが
  要求している間はモーションが勝つ。[docs/spec/motion.md](spec/motion.md#ポーズ))。
  欄はビューア・スタジオと同じ `@ugoe/player` の `PosePicker` で (T1640)、要素が 2〜4 個のグループは `SegmentedControl`、5 個以上は
  `Select` です。`preview.ts` の `previewPoseGroups` が編集中のモデルのグループ (`module: null`) の後にモジュールのグループを並べ、
  見出し (`previewPoseGroupLabel`。「Group N」「`<モジュール>` group N」) と、編集中のモデルの要素の表示名 (パーツの名前と ID。
  モジュールの要素は接頭辞を除いたもの) をエディタの側で渡します。選んだグループの位置で `showPose`・`showModulePose` に振り分けます。
- **モーション・表情:** ドキュメントのもの (`EditorDocument.motions` / `expressions`、保存前のものも) を JSON で
  `start_motion_json(json, "force")`・`set_expression_json` に渡します。モーションはフェード・ループ・イベント込みで
  再生し (発火したイベントを最近の 5 つまで出す)、「Stop」は `stop_motions` (フェードアウト)。表情は 1 つだけ選び、
  「(none)」で `stop_expressions`。
- **物理の設定:** バンドル (`get_model_bundle`) にはドキュメントの物理が入っています。バンドルが作れず JSON で表示している
  ときは物理が無いので、`WebPuppet` に物理が無ければ `set_physics_json` でドキュメントの物理を渡します。
- **読み込みの警告 (T0125):** モデルを読み込むたびに `WebPuppet.load_warnings_json()` の `code` から英語の知らせを作り、
  パネルの上に出します (`loadWarnings.ts`。読み込むたびに置き換える)。テクスチャ・法線マップを代用した (`texture_ktx2`・
  `texture_missing`・`normal_map_ktx2`)、前の物理を外した (`physics_removed`)、新しい版のファイルの知らない機能・値を
  フォールバックで読んだ (`unknown_optional_feature`・`unknown_value`。T0412)。外したモジュール (`module_detached`) は
  [着せ替え](#プレビューの着せ替え) の知らせに出すので除き、`physics_removed` もドキュメントの物理を渡し直せたときは除きます
  (編集のたびの読み直しで、外れたのは古い設定のため)。知らない `code` は wasm の文をそのまま出します。
  着せ替えの一覧のモジュールの警告 (テクスチャの代用) も、`modules_json()` の `warning_details` から同じ言い回しで作ります
  (T0652。`dressUp.ts` の `parseModules`。構造の無い文はそのまま)。
  プレビューの外 (編集の画面) では、今までどおり文の警告をコンソールに出すだけです。
- **値を別に持つ:** プレビューのスライダーの値 (`PreviewMode.values`) は、編集用の値
  (`ViewportController.parameterValues`) と別です。入るときに編集用の値を写して始め、毎フレーム `update` の後の値
  (物理・呼吸などを掛けた値) を表示します。プレビュー中は `ViewportController.setParameter` が編集用の値を書き換えても
  `WebPuppet` に渡しません。出ると `setDriver(null)` がモデルを読み直し (物理・モーション・表情・目パチの状態を
  初めからにする)、段をすべて止めて編集用の値を書き戻すので、入る前の姿に戻ります (ヘッドレスの Chrome で、
  入る前と出た後のビューポートの画像がバイト単位で一致することを確かめた)。次に入るとまた編集中の姿から始めます。
- **ほかのプレビューとの優先:** プレビューモードが優先です。入ると物理のプレビュー (T0629) を止め
  (`setDriver` が `stopPhysicsPreview` を呼ぶ)、タイムラインの再生と表情のプレビューを止めます (`TimelinePanel` が
  `ViewportController.driven` を見て `stop`・`endPreview`)。プレビュー中は物理のプレビューを始められません。
- スライダーは値域の 1/1000 程度のきりのよい刻み、値の欄は編集のパネルと同じ検査 (`parseValue`) で、値域の外は拒否します。
  並びはパラメータグループの定義順 (区切りの中はモデルの順、グループの無いものは最後の「Other」)。純粋な関数は
  `preview.ts` (Vitest)。
- **フェイストラッキング (T0636):** ビューア (T0703) と同じ部品 `@ugoe/face-tracking` (`packages/face-tracking/`、
  GPL-3.0-only。MediaPipe の呼び出し・頭の向き・キャリブレーション・平滑化・記録と再生・パネル) を使います。写さずに
  共有のパッケージへ切り出したのは、ビューアとエディタで値の約束・既定値・不具合の直しを一致させるため
  (どちらも pnpm のワークスペースの `"@ugoe/face-tracking": "workspace:*"`)。
  - `PreviewMode` が `TrackingTarget` (`setTracking` / `setAutoEyeBlink`) になり、パネルの「Face tracking」
    (最初は閉じている) の `FaceTracker` が値を渡します。値は `FrameDriver.beforeUpdate` で `set_tracking` に渡し
    (1 フレームに 1 回。モデルを読み直したら最後の値を渡し直す)、パラメータへの変換は Rust の対応付け
    (`avatar.tracking`) です。止めると `clear_tracking` し、トラッキングが書いていたパラメータを既定値に戻します。
  - 目パチはビューアと同じく、トラッキング中は既定で止めます (「Stop eye blink」)。目パチの段は「Eye blink」と
    トラッキングの指定のどちらも有効なときだけ動きます。プレビューを出るとトラッキングも止まります。
  - 体のトラッキング (T0645): 同じパネルの「Body (shoulders)」(既定で入れる) で、MediaPipe Pose Landmarker (lite) の
    体の向きと肩の上下も渡します (`setPuppetTracking` が `set_tracking` の体の引数に渡す)。式とキャリブレーションの扱いは
    [spec/avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker](spec/avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker)。
  - MediaPipe の wasm とモデルは、`pnpm dev` / `pnpm build` の前に `ugoe-mediapipe-assets public/mediapipe`
    (パッケージの `scripts/mediapipe-assets.mjs`) が用意します。wasm は npm パッケージから写し、モデル
    (`face_landmarker.task`・`pose_landmarker_lite.task`) は版を固定した URL から取って SHA-256 を検証します。`public/mediapipe/` はコミットしません。
    パネルの「Third-party licenses」は、MediaPipe とバンドルに入る JS のライブラリのライセンスの表記 (`public/third-party-notices.txt`。
    ビューア・スタジオと同じ形) を表示します。生成と検査は [release.md](release.md#同梱物のライセンスの表記)。
  - 映像と画像はこの計算機の中で処理し、どこにも送りません。MediaPipe が 60 秒ごとに送ろうとする利用状況
    (`https://odml.pa.googleapis.com/v1/log`) は、`@ugoe/face-tracking` が送る前に断り、CSP の `connect-src` も許しません
    ([packages/face-tracking](../packages/face-tracking/README.md#利用状況の送信-t0943)、T0943)。
  - Tauri 側 (`src-tauri/src/tracking.rs`): webview からのカメラの許可の要求だけを許し (`on_permission_request`)、
    記録の保存は `write_json_file` (保存のダイアログで選んだパスの `.json` だけ。[読み書きできるパス](#読み書きできるパス))。macOS は `Info.plist` の
    `NSCameraUsageDescription` と `Entitlements.plist` の `com.apple.security.device.camera`。
  - ヘッドレスのブラウザでは、記録した値の列 (JSON) か画像・動画を「File…」で入れて確かめます
    (`window.__UGOE_TRACKER__` でトラッカーを読める)。
- **まだ無いもの:** 目パチ・呼吸・リップシンクの設定 (`Model.effects`) を編集する画面はまだ無い
  (読み込んだモデルの設定で動く)。コマンドは `set_effects` ([エフェクトの設定](#エフェクトの設定))。

### プレビューの着せ替え

本体のプロジェクトのプレビューで、`.ugoemod` を読み込んで付け外しする (T0649、[ADR 0024](adr/0024-module-authoring.md))。
モデルは変えず、コマンドも送りません。付け外しは ugoe-wasm の `WebPuppet` の口 (`load_module`・`attach_module`・
`detach_module`・`set_modules`・`unload_module`、[crates/ugoe-wasm/README.md](../crates/ugoe-wasm/README.md#着せ替え-t0123)) で行い、
規則は [spec/modules.md](spec/modules.md#puppet-での着せ替え) のとおりです。コードは `apps/editor/src/lib/preview/` の
`DressUpSection.svelte` (パネルの節)・`dressUp.svelte.ts` (`DressUp`、状態と `WebPuppet` の操作)・`dressUp.ts`
(エラーの説明・スロットの行。Vitest) と、`src-tauri/src/dress_up.rs`。

- **読み込み:** 「Load .ugoemod…」(OS のダイアログ) か、プレビュー中のウィンドウへのドロップ。Tauri のウィンドウでは
  OS のドロップを Tauri が受けてパスを渡すので (`getCurrentWebview().onDragDropEvent`)、バイト列は `read_module_file`
  (`.ugoemod` だけを読む) で取ります。ブラウザ (ヘッドレスでの確認) では HTML のドロップの `File` を読み、パスは分かりません。
  読み込むとすぐに付けます (同じスロットのモジュールは差し替わる)。
- **付け外し:** スロットの行 (本体のスロットの定義順。そのスロットを指す読み込んだモジュールから選ぶ。「(none)」で外す) と、
  モジュールの行のチェックボックス。行の「↻」はファイルから読み直し (書き出し直した新しい版に付け替わる)、「✕」は一覧から除く。
- **公開していないスロットの印 (T0655):** アバタープロファイルがアプリに公開していないスロット (`avatar.slots` に無いもの) の行には
  `authors only` の印を付けます (`slotRows` の `published`)。ビューアはこのスロットを利用者に見せませんが (T0715)、エディタは作者のツールなので
  隠さず、付け外しもできます。プロファイルが無ければ区別しません (印を付けない)。
- **プレビューの間だけ付ける:** 出るときに `set_modules([])` で外し (付けたい組は覚えておく)、入るときに付け直します。
  編集の画面 (ツリー・選択・ヒットテスト) は本体の要素だけを扱うので、モジュールの要素を編集の画面に出さないためです。
  入るたびに、パスのあるモジュールをファイルから読み直します。
- **状態を保つ:** 付け外しで、パラメータの値・ポーズ・トラッキング・モーション・表情は続きから動きます (物理はリセット。
  `Puppet::set_modules` の規則)。付け外しの後、スライダーの表示とトラッキングの対象のパラメータを写し直します。
  バンドルが作れず JSON で表示しているときは、ドキュメントの物理を `set_physics_json` で渡してから付けます
  (付けるときの物理が合成の元になるため)。
- **読み直しと外れたことの知らせ:** エディタは編集のたびにモデルを読み直し、`WebPuppet` は付けていたモジュールを付け直します
  (付けられないものは外す、T0123)。`PreviewMode.loaded` で `DressUp.loaded` が、`WebPuppet` の外したもの
  (`detached_modules_json`。`DressUpError` と同じ `code` と補足の値、T0124) を、そのモジュールの行に「… was taken off: 理由」と出し、
  付けたい組から外します。そのうえで付けたい組と比べ、まとめて付けられなければ 1 つずつ試して、付けられないものを同じように外して知らせます
  (プレビューの外でスロットを消したり変えたりしてから入ったとき。出るときに外しているので `WebPuppet` は付け直していない)。
  知らせるのは行だけで、パネルの頭には出しません (T1644 まではパネルの頭にも閉じられる知らせを出しており、同じ理由が 2 回出ていた)。
  行は、直すときに見る場所 (付け直すチェックボックス・読み直し・除く) の隣で、付け直せるまで残るためです。
  `WebPuppet` を作り直したとき (GPU の喪失など) は、持っているバイト列から読み込み直します。
- **GPU の喪失 (T1644):** 読み込み (`load_module`) と付け外しは、GPU を失った後 (`is_lost()` が真) に呼ぶと `code` が `device_lost` の
  例外を投げます。このときは失敗の文を出さず、ビューポートの作り直し ([docs/ui.md](ui.md#webpuppet-の寿命)。プレビューは毎フレーム描くので
  次のフレームで気づく) の後に付け直します。読み込もうとしたバイト列は取っておき (`DressUp` の `#pending`)、付け外しは付けたい組
  (`wanted`) だけを変え、作り直した `WebPuppet` で `PreviewMode.loaded` → `DressUp.loaded` が読み込んで付けます。作り直した `WebPuppet` を
  付けている途中にまた失っても、付けたい組は変えません (次の作り直しで付ける)。Player (`@ugoe/player`) の読み込みの途中の喪失 (T0955) と
  同じ扱いです。作り直しを諦めたときはビューポートが文を出します。
- **エラー:** `DressUpError` の `code` を分かる言葉にします (`describeProblem`。「スロットがない → Slots タブで足す」
  「公開していないパラメータ → スロットの Parameters に入れる」など)。元の文言は「Details」に出します。
  読み込めないファイル (`invalid_file`) はパネルの上に、付けられないモジュールはその行に出します。
  知らせの形はプレビューのパネルとそろえます (T0676): 読み込みの失敗 (パネルの頭・閉じられる) と付けられないこと (行) は `Notice` の
  danger (自動で外れたこともその行の danger)、行の読み込みの警告 (代わりの画像を使ったテクスチャなど) は `Notice` の warning。
  行のファイルは名前だけを 1 行で出し、全体のパスは `title` にします (3 行に折り返していた)。
- **違いの一覧の見せ方 (T0676):** 「N differences from your current edits」を開くと、違いの 1 行の文の下に、要素の表 (ID・何が違うか・
  「Select」) を出します。要素の行が 8 件 (`DIFFERENCE_FILTER_FROM`) を超えると絞り込みの欄を出し、空白で区切った語のすべてを含む行
  (行の文が合えばその行の要素のすべて、合わなければ合う要素 (ID か違いの文) だけ) に絞ります (`filterDifferences`)。1 行の要素は 8 件
  (`DIFFERENCE_ROWS`) まで出し、残りは「さらに N 件を出す」で出します (パネルの中に縦の送りを重ねないため。絞り込み中はすべて出す)。
- **一覧をプロジェクトに残す:** 一覧 (パスと、付けるか) を `project.json` の `editor.dress_up` に残し、次に開いたときに
  読み込み直します (`get_preview_modules` / `set_preview_modules`。Undo の対象ではなく、変えると未保存の印が付く)。
  モジュールを書き出し直しては本体で試す、を繰り返すためです。パスは `.ugoeproj` と同じフォルダの中なら相対パスで書きます
  ([file-formats.md](spec/file-formats.md#プロジェクト-ugoeproj))。「Save As…」で別のフォルダーに保存し直すと、新しい保存先から
  同じファイルを指すように書き直します (T0669)。パスの分からないもの (ブラウザでドロップしたもの) は残しません。
  自動で外れたものは、残した一覧を書き換えません (次に開いたときも付けようとして、また知らせる)。
- **モジュールのプロジェクトのプレビュー (T0650・T0651):** プレビューの間は、作業用のモデル (本体と合成したもの) の代わりに
  **本体だけのモデル** (`get_preview_base_bundle`、`ugoe_editor_core::export_base_bundle`) を描き、**今の編集を書き出したモジュール**
  (`get_preview_module`。`export_module` と同じ検査をし、ファイルには書かない) を一覧の先頭の行 (`DressUpEntry.own`、画面では `editing` の印)
  として付けます (`PreviewMode` の `comparing`、`ViewportController.setPreviewModel`、`DressUp.setOwn`)。同じスロットのほかの `.ugoemod` と
  差し替えて見比べられ、ほかのスロットのモジュールとも重ねられます。
  - 今の編集の行は、除けず・ファイルから読み直さず・プロジェクトの一覧 (`editor.dress_up`) に残しません。付けるかは、差し替えても保ちます。
  - 編集でドキュメントを読み直すたびに (プレビューの中での編集。`EditorDocument.revision` が変わったとき)、今の編集を取り直します。
    本体だけのモデルは、本体の版 (`ModuleStatus.base_revision`、`ModuleContext::base_revision`) が描いているものと違うときだけ取り、
    読み込み直します (T0653)。本体の版は本体を読み込むたびにプロセスの中で重ならない番号になり、名前・スロット・条件を変えても変わりません。
    本体だけのバイト列は **バックエンドのセッション** (`AppState.preview_base`、`module::PreviewBaseCache`) が最後の 1 つを覚えておき、
    本体の版が同じか、本体のファイルの指紋 (`ProjectSession` が本体を開いた・読み直したときに取るもの) とパスが同じなら作り直しません (T0654)。
    フロントエンドはバイト列を覚えないので、プレビューに入り直すと取り直しますが、同じ本体なら作り直しは起きません。同じ本体のファイルを
    開き直したとき (モジュールのプロジェクトを開き直す・本体を読み直したが中身が同じ) も、指紋が同じなので作り直しません。
  - 今の編集は、モジュールの名前に ` (editing)` を付けた名前で読み込みます (`get_preview_module(name)`、`ugoe_editor_core::export_module_as`、
    `editingName`。T0653)。`WebPuppet` は同じ名前のモジュールを 1 つしか持てないので、こうして前に書き出した同じ名前の `.ugoemod` も並べて読み込み、
    同じスロットで差し替えて見比べられるようにしています。一覧の行にはモジュールの名前 (`DressUpEntry.label`) と `editing` の印を出し、
    スロットの選択肢には読み込んだ名前 (「CatEars (editing)」) を出します。名前を変えても要素の ID の接頭辞が変わるだけで、見た目は同じです。
    その名前と同じ名前のファイルを読み込んだときだけ、今の編集を残してファイルは付けません (`same_name` の説明「This file is also named …」)。
  - 今の編集と同じ名前 (モジュールの名前) のファイルの行には、今の編集との違いを文字でも出します (`compare_preview_module`、
    `ugoe_editor_core::compare_with_module_file`、`DressUpEntry.differences`。T0654)。違いは「ファイル → 今の編集」の向きの英語の 1 行ずつで、
    要素 (パーツ・デフォーマ・メッシュ・グルー) の数と、足した・除いた・変えたものの ID、テクスチャ (足した・除いた・画像が違うもの)、
    条件 (アバタープロファイルのレベル・スロット・パラメータ)、使うパラメータ、物理のリグと衝突体、ポーズのグループと追従です。メッシュのテクスチャは番号でなく
    名前で比べます。同じなら「Same as your current edits.」。今の編集を取り直すたび・そのファイルを読み込み直すたびに比べ直し、
    途中の結果は捨てます。今の編集を付けて比べていない (書き出しの検査に通らない) 間は出しません。
  - 足した・変えた・除いた要素 (パーツ・デフォーマ・メッシュ・グルー・物理のリグ・衝突体) は、行の下に要素ごとに何が違うかを添えます
    (`ModuleDifference.items`。T0656・T0657)。メッシュは
    名前・パーツ・親・テクスチャ・頂点 (数か、UV の変わった頂点の数)・三角形・キーフォーム (軸・数・頂点の位置の変わったキーフォームの数)・
    描画順 (どのキーフォームでも同じ値なら `draw order 980 → 990`、でなければ変わったキーフォームの数。値はモジュールの空間
    (0〜1000) から今のスロットの範囲に写したもので、ツリーやインスペクタと同じ)・不透明度と色・合成とマスク。
    デフォーマは名前・パーツ・親・種類・格子の大きさ・キーフォーム、パーツは名前・親・キーフォームの不透明度です。
    グルーは両端のメッシュ・頂点の組 (数か、変わった組の数)・強さのキーフォーム、物理のリグは名前・入力と出力 (数か、変わった数)・
    粒子 (数か、設定の変わった粒子の数)・正規化・衝突体 (足した・除いた ID)・重力の基準、衝突体は名前・取り付け先・形・位置・半径です。
    ポーズのグループ (T0659) は ID を持たないので、要素のパーツを ` / ` でつないだもの (`HatA / HatB`。最初に表示する要素から) を ID とし、
    要素のパーツを 1 つでも共有するグループを同じグループとみなして、要素の並び (`entries HatA / HatB → HatB / HatA`) と要素ごとの
    リンク (`links of HatA (added: …; removed: …)`) を添えます。追従 (本体の要素に加えるリンク、`ModulePose::links`) は本体の要素の
    パーツを ID とし、リンクの足した・除いたパーツを添えます。どちらにも出ない違い (グループの並びだけ) は「Pose: order of groups changed」。
    足したものは `added`、除いたものは `removed`。
  - 要素の ID を押すと、プレビューのままビューポートでその要素を強調します (`DressUp.highlight`・`DifferenceOverlay.svelte`。T0657)。
    もう一度押すか、ビューポートの下の「Clear」でやめ、プレビューを出ると消えます。描くのは、いまビューポートに出ている版 (同じスロットに
    付いているのが今の編集 `<名前> (editing)` かファイル `<名前>` か。比べていなければ作業用のモデルで今の編集) の `WebPuppet` の
    要素です (`shownVersion`)。要素ごとに強調するメッシュ (`DifferenceItem.meshes`。モジュールの中の ID で、ファイルと今の編集の
    どちらかにあるもの) の輪郭を描き (パーツ・デフォーマはその下のメッシュ、グルーは両端、物理のリグは出力先のデフォーマの下と
    頂点の出力先のメッシュ、ポーズのグループは要素とリンクのパーツの下、追従はリンクのパーツの下のメッシュ)、
    デフォーマはワープの制御点か回転の原点と軸、物理のリグは粒子の鎖 (動いていれば今の位置、でなければ初期位置。
    パラメータにだけ出すリグは描かない)、衝突体は円・カプセルを重ねます。輪郭は作業用のモデルに同じ頂点数のメッシュがあればその
    三角形の外周、無ければ (ファイルだけの形) 頂点の凸包です。下の知らせに、どちらの版の要素かを出し、描けなければ見える方法
    (「… was removed in your current edits. Put on the file to see it.」など) を出します。描いている版に無い要素 (今の編集で除いたもの・
    ファイルに無い足したもの。`missingIn`) のメッシュがその版に残っていれば (除いたポーズのグループのパーツの下のメッシュなど)、輪郭を
    破線で描き、知らせを「Removed: … is not in your current edits; the outline shows its meshes that are still there. Put on the file to see it.」
    (足したものは「Added: …」) にします (T0661。T0659 では「… in your current edits」と出て、除いたと分からなかった)。比べ直すと、強調は同じ要素 (種類と ID) の
    新しい結果に替わり、無くなればやめます。
    知らせの「Fit」で、ビューポートを描いた要素に寄せます (T0659。`fitMove`: 描いた輪郭・点・形の範囲が、ビューポートの 80% に収まるように
    要素の中心でズームし (`ViewportController.zoomAt`)、中央へずらす (`panBy`)。範囲は 24 ピクセルより小さくみなさない)。
    全体に戻すのはビューポートの「Fit」です。
  - 「Select」で、プレビューを出てその要素を選びます (`differenceTarget`。ツリーと Physics タブはプレビューの間は隠れる。入り直すと
    付けていたモジュールは付き直る)。パーツ・デフォーマ・メッシュは作業用のモデルの ID (`DifferenceItem.element`。接頭辞
    `<モジュールの名前>/` の付いたもの) でツリーで選び、グルーは Glue タブを開いてそのグルーを選び、今の編集にある両端のメッシュも
    ツリーで選びます (T0659)。物理のリグと衝突体は Physics タブを開いてそのリグか衝突体を選びます。ポーズのグループと追従は Poses タブを
    開き、そのパーツ (`DifferenceItem.parts`。作業用のモデルの ID で、グループは要素とリンクのパーツ、追従は本体の要素とリンクのパーツ) を
    ツリーで選びます (T0659)。除いた要素は今の編集に無いので選べません (強調はできる)。
  - ファイルがアトラス化してあれば (`atlas-` で始まり法線マップの無いテクスチャで、今の編集に同じ名前のテクスチャが無いもの)、
    アトラスの中で今の編集の画像を置いた場所を見つけ、メッシュのテクスチャと UV をアトラスの前に戻してから比べます (T0656)。ファイルには
    置いた場所が書かれていないので、同じ ID のメッシュの UV の組ごとに置いた左上の画素 (`round(u' × アトラスの幅 − u × 画像の幅)`) を
    推し量って多数決を取り、その場所の画素が今の編集の画像と同じかを確かめます。画素が違っても過半数の組が同じ場所を指せばその場所を使い、
    テクスチャは「画像が違う」に出します。どちらでもなければ、メッシュの UV が占める範囲を覆う位置を画素で探します (不透明な画素を先に
    32 個見る。試す位置は 65536 まで)。今の編集で除いたメッシュも、見つけた場所の中にあればそのテクスチャ、無ければまだ見つけていない
    今の編集の画像を画素で探して戻します。UV は、今の編集の UV をアトラスに写した値 (`Placement::map`。書き出しと同じ計算) がファイルの
    値と同じならその UV に戻すので、変えていなければ丸めの違いも出ません。使うメッシュをすべて戻せたアトラスはテクスチャとして比べず、
    戻せないメッシュが残れば、アトラスをテクスチャのまま比べて「Some meshes in the file's texture atlases could not be matched …」と添えます。
  - `.ugoemod` にアトラスの対応表 (元のテクスチャの名前と置いた場所) は **残しません** (T0657 で決めた)。理由: 対応表を使うのは
    この比較だけで、ランタイム (付けて描く側) には要らない。残すとファイル形式の変更になり、公開仕様・`SCHEMA_VERSION`・互換性の資料と、
    `.ugoe` のアトラスとの揃え方まで決め直すことになる。比べる側で UV の組と画素から戻す方法 (上) で、書き出したファイルとは
    「同じ」と出せている。戻せないのは、今の編集で画像も UV も描き替えたメッシュだけで、そのときは注記を出して違いとして数えるので、
    違いを見落とすことはない。画素の探索が重いほど大きなアトラスで困ることが分かったら、そのときに見直す。
  - 今の編集を書き出せない (検査に通らない) 間は、理由 (「Your edits can't be put on the base …」) を出し、作業用のモデルを描きます。このときは
    T0650 と同じく、自分のスロットの行は出さず、そのスロットのモジュールは付けません (`DressUpHost.ownSlot`。`own_slot` の説明)。
  - 本体だけのモデルには本体の物理がバンドルに入っているので、ドキュメントの物理 (モジュールのものを含む) を `set_physics_json` で渡しません。

## モジュールのプロジェクト

着せ替えのモジュールは、本体を参照する別のプロジェクトとして作ります ([ADR 0024](adr/0024-module-authoring.md)、T0648)。
ファイルの形は [file-formats.md#モジュールのプロジェクト](spec/file-formats.md#モジュールのプロジェクト)、使い方は [manual/dress-up.md](manual/dress-up.md#衣装を作る)。

### Document がモジュールを編集する方法

**`Document` は本体とモジュールを合成した「作業用のモデル」を持ち、`Command` はモデルのものをそのまま使います**
(`ugoe_editor_core::module_project`)。モジュール専用の `Command` や、モデルとモジュールの 2 通りに効く `Command` は作りません。

- 開くと、本体と `module.json` を `compose`・`compose_physics` で合成し、それを `Document` のモデル・物理・画像にします。
  モジュールの要素の ID は合成と同じ `<モジュールの名前>/<ID>` になり、本体の ID と衝突しません。
  `Document` は本体・スロット・名前・条件 (`ModuleContext`) を `Arc` で、スナップショットごとに持ちます (`Document::with_module`。名前・スロット・条件は [モジュールの設定](#モジュールの設定) で変わる)。
- **コマンドを適用するたびに、作業用のモデルをモジュールと本体に分けます** (`ModuleContext::split`。`Document::apply` の最後の検査)。
  - 本体の要素 (モジュールで増えた要素を除いたもの) が本体と同じでなければ `EditError::ReadOnlyBase` で拒否します (`` mesh `face` belongs to the base model… ``)。
    比べるのは、パラメータ・グループ・パーツ・デフォーマ・メッシュ・グルー・テクスチャと画像・当たり判定・スロット・ポーズ・アバタープロファイル・
    キャンバス・エフェクト・ライティングと、物理の本体のリグ・衝突体・重力などです。本体の要素は ID で見分けます (本体にない ID の要素がモジュールの要素)。
  - モジュールとして表せなければ `EditError::ModuleShape` で拒否します: モジュールのメッシュ・デフォーマが本体のパーツに属する・スロットの取り付け先以外の
    本体のデフォーマの子になる・本体のテクスチャを使う・本体の要素をミラー元にする、グルーやリグの出力が本体の要素を指す、接頭辞を外すと ID が重なる
    (`Hat/Brim` と新しく作った `Brim`)、モーションや表情を作る、など。
  - 分けた結果は捨てます (保存と書き出しのときに同じ関数でもう一度分ける)。検査はモデルの複製と比較なので、コマンドごとに線形の時間がかかります。
- **整える:** コマンドの後、キャンバスの直下にあるモジュールのデフォーマとメッシュを、見た目を保ったまま (`ReparentPreservingLook` と同じ処理で)
  スロットの取り付け先の下へ移し、最上位のパーツをスロットのパーツの下へ移します (`ModuleContext::normalize`)。モジュールの最上位の要素の座標は
  取り付け先の局所座標なので (`docs/spec/modules.md#モジュール`)、キャンバスの座標のまま残すと合成したときにずれるためです。
- **描画順:** 作業用のモデルでは本体の空間 (スロットの範囲に写したもの) で見せ、編集します。分けるときに 0〜1000 に戻します
  (小数第 3 位で丸めた値が同じ描画順に写るなら丸めた値。範囲の幅が 0 なら 500)。PSD の取り込みは 0〜1000 で描画順を付けるので、
  モジュールのプロジェクトでは取り込みのコマンドの描画順をスロットの範囲に写してから適用します (`ModuleContext::map_new_meshes`、`import_psd`)。
- **テクスチャ:** 作業用の画像は本体の画像と、名前に接頭辞を付けたモジュールの画像 (法線マップの名前も付け替える) です。本体の画像は変えられません。
- Undo はモデルと同じスナップショットです。検証 (`validate`) も作業用のモデルに対して、モデルと同じ規則で行います。

**選んだ理由:** 代わりに `Document` の中身を「モデルかモジュール」の列挙にし、各 `Command` がモジュールにも効くようにする案もありました。
しかしモジュールの最上位の要素はスロットの取り付け先の局所座標で表すので、本体 (取り付け先とその親) が無いと、ビューポートの表示・当たり判定・
見た目を保った付け替え・推定などの座標の計算ができません。本体を含めた作業用のモデルなら、今の `Command`・推定・メッシュの編集・AI の提案がそのまま動き、
本体の下敷きも同じ描画で出せます。本体を書き換えないことと表せない形は、1 か所の検査 (`split`) で守れます。

### 本体

- 本体は `.ugoeproj` (本体のモデルのプロジェクト) か `.ugoe` です (`project::load_base`)。`.ugoe` のテクスチャは PNG に限ります。
  モジュールのプロジェクトを本体にはできません (その本体まではたどらずにエラー)。読む前に本体のパスを確かめる口 (`load_base_checked`・
  `Project::load_checking_base`) があり、MCP の `--root` が使います (エディタでは確かめません)。
- 本体の参照は `project.json` の `module.base` に、保存するたびにプロジェクトからの相対パスで書きます。
- 開くときに本体が見つからなければ `base model not found: …` のエラーで、ファイルメニューに「Locate base project…」「Locate base .ugoe…」が出ます
  (`open_module_with_base`。付け替えは保存するまでファイルに書かず、未保存の印が付く)。
- 本体のファイルは [外での書き換え](#外での書き換え) の指紋に含めません。モジュールのプロジェクトは本体を書かず、開き直せばそのときの本体と合成し直すので、
  本体が変わっても保存した内容は失われません (下敷きが古いだけ)。本体の変化には、本体の指紋を別に取って気づきます ([本体の変化](#本体の変化))。
- 本体のパラメータ (標準パラメータとスロットが公開するもの以外も) のスライダーで動かせます。公開していないパラメータをモジュールのキーフォームに使うと、
  書き出しの合成の検査で `ParameterNotExposed` になります。

### 画面と Tauri コマンド

- ファイルメニューの「New Module…」で新規作成の画面 (`NewModuleDialog.svelte`) を開き、本体 (「Project folder…」か「.ugoe file…」)、スロット、
  モジュールの名前 (既定はスロットの名前) を選びます。画面の「What you can do in a module project」に、できること・本体のプロジェクトで直すことを
  出します (`MODULE_CAN`・`MODULE_CANNOT`、`src/lib/moduleProject.ts`)。モジュールのプロジェクトを開いている間は、名前の横に `Module: <スロット>` の印
  (押すと [モジュールの設定](#モジュールの設定)。ツールチップに本体の場所と、できること・できないこと) が出て、
  「Export…」が「Export Module…」(`.ugoemod`) に替わります。
- ツリーでは本体の要素に `base` の印を付けて薄く表示します (`src/lib/moduleProject.ts`)。選んだり確かめたりはできますが、編集の欄は無効です
  ([本体のものの表示](#本体のものの表示))。
- ビューポートとプレビューは作業用のモデル (`get_model_bundle`) を描くので、本体と合成した姿が出ます。

| コマンド | 内容 (`src-tauri/src/module.rs`) |
| --- | --- |
| `module_status` | モジュールのプロジェクトなら名前・本体のパス・スロット (`pose_parts` を含む)・本体の要素の ID・本体のポーズのグループの数 (`base_pose_groups`)・スロットが公開するが本体に無い衝突体 (`unavailable_colliders`。T0660) (`ModuleStatus`)、でなければ `null` |
| `read_module_base(path)` | 本体の候補を読み、名前とスロットの一覧 (`BaseSummary`) を返す |
| `new_module_project(base, slot, name)` | 空のモジュールのプロジェクトにする (未保存) |
| `open_module_with_base(path, base)` | 本体を付け替えてモジュールのプロジェクトを開く |
| `export_module(path)` | `validate_module` と本体との合成を確かめて `.ugoemod` を書き、大きさと警告とアトラス化の結果 (`ModuleExportReport`) を返す。書き出しの設定でアトラス化を選んでいればモジュールのテクスチャをまとめる ([書き出し](#書き出し)) |
| `open_module_with_base(path, base, slot, drop_colliders)` | 本体 (`base`) かスロット (`slot`) を付け替えて開く (`null` なら `project.json` のまま。`Project::load_module_with`)。`drop_colliders` なら本体に無い衝突体へのリグの参照を外して開く (T0661) |
| `module_open_problem(path, base)` | 本体の問題で開けないなら、問題の種類と直すための情報 (`ModuleProblem`。`base_needs_physics` で外せば開けるなら `collider_references`)。`base` なら選び直した本体で確かめる |
| `check_base_change()` | 本体のファイルが開いた (読み直した) 後で変わったか (`ExternalChange` か `null`) |
| `reload_base(base, slot)` | 本体を読み直す。`base`・`slot` があれば本体・スロットを付け替える (開き直さない。未保存の印が付く)。読み直せなければ `problem` 付きで、ドキュメントはそのまま (`BaseReload`) |
| `get_preview_base_bundle()` | 本体だけの `.ugoe` のバイト列 (プレビューで今の編集と比べる。[プレビューの着せ替え](#プレビューの着せ替え)) |
| `get_preview_module(name)` | 今の編集の `.ugoemod` のバイト列 (ファイルには書かない。書き出しと同じ検査)。`name` があればモジュールの名前をそれにする |
| `compare_preview_module(<bytes>)` | 今の編集と、書き出した `.ugoemod` (本文のバイト列で送る) の違いの行 (`ModuleDifference[]`。足した・変えた要素ごとの詳細とツリーで選ぶ ID を持つ。同じなら空。[プレビューの着せ替え](#プレビューの着せ替え)) |

`ModuleStatus` は、本体のテクスチャ・物理のリグと衝突体・スロット・パラメータ・アバタープロファイルのレベルと、モジュールの条件 (`requires`) も返します
(画面が本体のものの欄を無効にし、設定の画面で選ばせるため)。`base_revision` は本体の版で、プレビューが本体だけの `.ugoe` を取り直すかを決めます。

### モジュールの設定

モジュールの名前・付けるスロット・本体に求める条件 (`Module::requires`) は、`Module:` の印を押して開く画面 (`ModuleSettingsDialog.svelte`) で
変えます (T0650)。「Apply」で `Command::SetModuleSettings { name, slot, requires, keep_look }` を 1 つ送り、1 回の Undo で戻ります。

- `Document::apply_with_report` がこのコマンドだけを先に受け、作業用のモデルを今の設定でモジュールに分け (`split`)、新しい設定の `ModuleContext`
  (`ModuleContext::reconfigured`。本体は同じ) で合成し直します (`workspace`)。本体がスロット・条件を満たさなければ (`compose` のエラー)、何も変えずに拒否します。
  `Batch` の中では使えません (作業用のモデルを丸ごと組み直すため)。設定が変わらなければ何もしません。
- **`ModuleContext` はスナップショットに入れる:** `Document` の `module` を `Snapshot` に移し、Undo / Redo で設定も戻るようにしました
  (以前は「編集中は変わらない」ものとして `Document` が直接持っていた)。
- **名前を変えると ID の接頭辞が変わる:** 作業用のモデルの要素の ID は `<新しい名前>/…` になります (新しく作って接頭辞のなかった要素も、合成し直すと付く)。
  画面は選択を新しい ID に付け替えます (`renamePrefix`、FileMenu の `onrenamemodule`。適用の後、ドキュメントを読み直す前に)。
- **ほかの編集と同じ道を通る (T0689):** 画面の「Apply」は `EditorDocument.applyWith` で適用し、適用の報告の警告 (`ApplyReport.warnings`) を
  ほかの編集と同じく出します。失敗は `rethrow` で設定の画面に返し、画面の中に出します。
- **スロットを変えると:** 描画順は新しいスロットの範囲に写し直されます。モジュールの最上位の要素は取り付け先の局所座標なので、
  取り付け先の違うスロットに移すとき:
  - `keep_look` (画面の「Keep where it appears」、既定で入) なら、合成し直した作業用のモデルで、モジュールの最上位のデフォーマとメッシュの親を
    いったん前の取り付け先に戻し、`ReparentPreservingLook` と同じ写し方 (`deformer_edit::reparent`。基準の姿で見た目を保つ) で新しい取り付け先の
    下へ移します (`ModuleContext::keep_look`、T0651)。推定のセルは作り直し (`inference::rederive`)、検証とモジュールへの分割を確かめてから採用します。
    ミラーし直せなかった左右の対 (`inference.mirror_failed`) は、ほかの編集と同じく拒否せずに `ApplyReport.warnings` で返し、画面がインスペクタの
    警告に出します (付け替えで親が変わった対の確かめ `check_reparented_pairs` も行う。名前を変えたときは作業用の ID の接頭辞 `<名前>/` が変わるので、モジュールの要素の接頭辞を前の名前に付け替えて前のモデルと
    突き合わせる (`ModuleContext::previous_target`)。T1609・T1621)。
    物理の衝突体のうち、モジュールのもので取り付け先に付くもの (モジュールの中では `attach` なし) は、形状の位置 (`center`・`a`・`b`) を同じく
    基準の姿で前の取り付け先の局所座標 → キャンバス → 新しい取り付け先の局所座標と写し直します (半径はピクセルなので変えない。T0653)。
    リグの粒子は写し直しません。モジュールのリグの根元はモジュールのデフォーマ (出力先か `frame`) の局所座標かキャンバス座標で表され
    ([physics.md](spec/physics.md#物理空間と根元の取り付け先))、どちらもデフォーマを写し直せば基準の姿のキャンバスでの位置が保たれるためです
    (本体のデフォーマを `frame` や出力先にするリグはモジュールとして表せない)。
  - `keep_look` でなければ局所座標のままで、見た目の位置が変わります (画面が注意を出す)。
  スロットが公開していないパラメータ・マスク・衝突体・ポーズのパーツを使っていれば拒否されます。
- **条件:** アバタープロファイルのレベル (本体が名乗るレベルまで)、ほかのスロット、パラメータを本体のものから選びます。付けるスロットは書きません。

### モジュールの適用のエラーの案内

モジュールのプロジェクトで適用を拒否されたとき、本体を直せば済むエラーには、エラーの文の下に直し方の案内を出します (T1609。
`ModuleErrorHelp.svelte`・`moduleErrorHelp` (`src/lib/moduleProject.ts`))。出す場所は、インスペクタの上のエラー (ほかの編集) と
[モジュールの設定](#モジュールの設定) の画面です。本体が `.ugoeproj` なら「Open base project」で本体のプロジェクトを開けます
(保存していない変更があれば先に確かめる。設定の画面は閉じる。開いた後の流れは本体の問題の「Open base project」と同じで、
[モジュールに戻る](#モジュールに戻る) 帯が出る)。
本体が書き出した `.ugoe` なら開けないので、本体のプロジェクトで変えて書き出し直すよう案内します。

| コード (`cause` のコード) | 案内 |
| --- | --- |
| `edit.read_only_base`・`module.read_only_base` | 本体の要素は本体のプロジェクトで変えて保存し、開き直すか本体を読み直す |
| `module.collider_needs_base_physics` | 本体のプロジェクトの物理のタブで物理を足して保存する |
| `module.compose_failed` (`compose.parameter_not_exposed`・`compose.unknown_mask`・`compose.unknown_collider`・`compose.pose_not_exposed`) | 公開しているスロットを選ぶ・モジュールで使うのをやめる・本体のスロットで公開する |
| `module.compose_failed` (`compose.missing_parameter`・`compose.missing_slot`・`compose.avatar_level`) | 条件やモジュールから外す・本体に足す |

ほかのコード (`module.invalid_name`・`module.unknown_slot`・`edit.module_shape`・`edit.not_module`・上に無い `compose.*` など) は、
エラーの文が直し方まで言うか、本体ではなくモジュールの側を直すものなので、案内を付けません。

### モジュールに戻る

「Open base project」(本体の問題の枠と、[適用のエラーの案内](#モジュールの適用のエラーの案内)) で本体を開いた後、直し終えたら元の
モジュールに戻れるよう、ヘッダーの下の知らせの帯に「Editing the base of module …」と「Return to module」を出します (T1622。
`ModuleReturnBar.svelte`・`FileMenuFlow.returnTo`・`FileMenuFlow.returnToModule`)。

- **出すとき:** 本体を開けたときで、モジュールのプロジェクトの場所が分かるとき (保存してある。開けなかったプロジェクトなら開こうとした場所)。
  名前は開いていたモジュールの名前 (本体の問題の枠からなら、フォルダー名から `.ugoeproj` を除いたもの)。
- **戻る:** 「Return to module」はモジュールのプロジェクトを `open_project` で開き直します。モジュールは保存した本体の上に組み直されるので、
  本体の直しは保存してから戻ります。本体に保存していない変更があれば帯に保存を促す文を出し、押すとほかの開く操作と同じく捨てるかを確かめます。
  開き直しも本体の問題で失敗すれば、「Open…」と同じく [本体の問題の直し方](#本体の問題の直し方) の枠を出し、帯は残ります。
- **適用し直す:** [モジュールの設定](#モジュールの設定) の画面で拒否された設定から本体を開いたときは、拒否された `set_module_settings` を覚え、
  戻った後に同じ設定を適用し直します (ほかの編集と同じく 1 回の Undo で戻る)。また拒否されたら、設定の画面をその設定とエラーを欄に入れて
  開きます (今の設定ではなく)。インスペクタの上のエラー (ほかの編集) から開いたときは、開き直すだけです (本体の要素の編集などは、
  本体を直せばモジュールで同じ編集をする必要がないことが多く、同じコマンドを送り直しても多くは同じ理由で拒否されるため)。
- **消すとき:** 戻ったとき・ほかのプロジェクトに替わったとき (新規・開く・見本など)・帯を閉じたとき。本体を外での書き換えで読み直しても残ります。
- **読めるパス:** モジュールのプロジェクトのフォルダーは閉じると「開いているプロジェクトの中」でなくなるので、`open_project` は閉じるのが
  保存したモジュールのプロジェクトなら、そのフォルダーを読めるパスに加えます ([読み書きできるパス](#読み書きできるパス))。
  本体の問題の枠から開いたときのモジュールは、開くダイアログで選んだので読めます。

### 本体のものの表示

モジュールのプロジェクトでは、本体のもの (読み取り専用) の欄を無効にし、「This belongs to the base model and can't be edited here. Edit it in the
base project.」と示します (`BaseLock.svelte`。`fieldset` の `disabled` で入力欄とボタンをまとめて無効にする)。

| 場所 | 無効にするもの |
| --- | --- |
| インスペクタ | 主な選択が本体の要素なら、プロパティ・キーフォーム・推定・深度・描画用の複製の欄 (見出しに `base`)。何も選んでいないときのアバタープロファイル |
| パラメータ | 「+」(モジュールはパラメータを足せない)、定義の編集欄。選んだ要素が本体のものなら ◆ (キーの追加)。スライダーはいつも動かせる |
| Physics | World (重力・風・頻度)、本体のリグの編集欄、本体の衝突体の編集欄。リグの ▲▼ は本体のリグとの入れ替えを無効にする (モジュールのリグは合成で本体のリグの後に並ぶので、モジュールのリグどうしでだけ並べ替える。`canMoveRig`)。「+ From selected deformers」は本体のデフォーマを鎖に入れない。テンプレート (`TemplateSection`) は「Create missing parameters」を無効にし、首の候補から本体のデフォーマを除き、本体にあるリグを作り直すテンプレートと、本体にない中継パラメータ (`BodyBounce`) を作るテンプレートを無効にする (T0651) |
| Lighting | モデル全体のライティングの有無、本体のテクスチャの法線マップ、「Lit meshes」の本体のメッシュ (`base` の印。「All on」「All off」・「Light selected」はモジュールのメッシュだけを変える。T0651) |
| Slots | タブ全体 (スロットは本体のもの) |
| ビューポート | 本体の要素を選ぶと、選択枠を灰色の破線にし、編集のツール (フォームの編集・ギズモ・メッシュの編集) を出さず、「Base model: can't be edited here…」と示す。「Module outline」(既定で入) でモジュールのメッシュすべての輪郭を重ね、本体と見分ける |

- **ID の接頭辞を隠す:** 合成で付いた `<モジュールの名前>/` を、表記 (`labels.ts` の `elementLabel`・`nameOf`・`textureLabel`) から外します
  (`setHiddenIdPrefix`。`EditorDocument.refresh` が設定する)。コマンドに渡す ID はそのままです。
- 無効にしていない欄から本体を変えようとしても、バックエンドが
  `` mesh `face` belongs to the base model and can't be edited here. Edit it in the base project `` で拒否します。

### 本体の変化

本体のファイルが、モジュールのプロジェクトを開いた (読み直した) 後で変わったことに気づき、本体を読み直すか尋ねます (T0650)。

- **指紋:** `ProjectFingerprint::read_base` (本体が `.ugoeproj` ならプロジェクトの指紋、`.ugoe` ならそのファイル 1 つ) を、開いたとき・読み直したときに
  `ProjectSession` が覚えます。確かめる機会は [外での書き換え](#外での書き換え) と同じ (ウィンドウに戻ったとき・5 秒ごと) で、保存していない
  プロジェクトでも確かめます。
- **確認 (`BaseChangeDialog.svelte`):** 変わっていれば「Reload the base」「Later」。「Later」を選んだ状態は二度尋ねません。本体が見つからなければ
  (動かした・消した)、「Locate base project…」「Locate base .ugoe…」で開き直さずに本体を選び直せます (`reload_base` に `base`。下の「付け替え」)。
- **自動の読み直し (T0651):** 外での書き換えの「Reload automatically」と **同じ設定** (`loadAutoReload`) を使い、確認の画面にも同じ欄を出します。
  入っていれば尋ねずに読み直し、「Reloaded the base model: it was changed outside the editor」と知らせます (`baseAction`)。本体の読み直しは保存していない
  モジュールの編集を保つので、プロジェクトの自動の読み直しと違い、未保存の変更があっても読み直します。本体が見つからないときは尋ねます。
  別の設定にしなかったのは、どちらも「外で変わったものを黙って取り込むか」という同じ好みで、欄が増えると分かりにくいためです。
- **読み直し (`reload_base`、`Document::rebase_module`):** 作業用のモデルを今の本体でモジュールに分け、読み直した本体で合成し直します。保存していない
  モジュールの編集は保ち、保存済みかどうかも保ちます (モジュールの中身は変わらない)。前の状態は古い本体のものなので、Undo の履歴は消します
  (状態の番号は続けるので、AI の提案は古くなったと分かる)。読み直せなければ ([本体の問題の直し方](#本体の問題の直し方))、ドキュメントはそのままです。

### 本体の問題の直し方

本体の問題でモジュールのプロジェクトを開けない・本体を読み直せないとき、問題と直し方を枠で出します (`ModuleProblemBar.svelte`、T0650)。
問題の種類はバックエンドが `ProjectError` から決めます (`ModuleProblemKind`)。

| 種類 | 何が起きたか | 直し方のボタン |
| --- | --- | --- |
| `base_not_found` | 本体が見つからない (動かした・名前を変えた) | 「Locate base project…」「Locate base .ugoe…」 |
| `base_unreadable` | 本体を読めない (壊れている・モジュールのプロジェクトを指している・PNG でない画像) | 同上 |
| `unknown_slot` | 本体にモジュールのスロットがない | スロットを選んで「Use this slot」、「Open base project」(スロットを戻す)、本体を選び直す |
| `does_not_fit` | 本体はあるが、モジュールが合わない (条件・公開されたパラメータ・マスクなど) | 「Open base project」(本体を直す)、本体を選び直す |
| `base_needs_physics` | モジュールのリグがスロットの公開する本体の衝突体に当たるが、本体に物理が無い (`ModuleError::ColliderNeedsBasePhysics`。T0660) | 「Open base project」(本体に物理と衝突体を戻す)、本体を選び直す、「Open without these colliders…」(参照を外して開く。下の「衝突体の参照を外して開く」) |

- **物理の無い本体が公開する衝突体 (T0660):** ランタイム (Puppet) は、本体に物理が無ければスロットの公開する衝突体への参照を外して
  付けます ([spec/modules.md#合成](spec/modules.md#合成))。作者のツールは、保存のときに参照が黙って消えないよう、作業用のモデルと書き出しの
  検査ではエラーのままにし、`ModuleContext::compose_physics` が `ComposeError::UnknownCollider` を `ModuleError::ColliderNeedsBasePhysics`
  (リグ・衝突体・スロットと、本体に物理を足すよう促す文) に置き換えます。開けたモジュールのプロジェクトでも、物理の画面の「Colliders」に
  当たれない衝突体を案内します。
- **衝突体の参照を外して開く (T0661):** `base_needs_physics` で開けなかったとき、`module_open_problem` は、本体に無い衝突体への
  リグの参照を外せば開けるなら、外す参照を `ModuleProblem.collider_references` (`ColliderReference { rig, collider }`。モジュールの中の ID)
  に入れます。枠の「Open without these colliders…」で、外す参照 (見本の TwinTails なら `TailL → Shoulders`・`TailR → Shoulders`) と、保存するまでファイルは変わらないこと、本体に
  物理と衝突体を足せば戻せることを並べて確かめ (`dropCollidersPrompt`)、「Remove and open」で `open_module_with_base` に `drop_colliders: true`
  を渡して開きます (`Project::load_module_without_unavailable_colliders`。`ModuleContext::drop_unavailable_colliders` がスロットの公開する
  衝突体のうち本体に無いものへの参照を外す。モジュール自身の衝突体への参照は残す)。開いたプロジェクトには未保存の印が付き、保存すると
  `module.json` から参照が消えます。T0655 の「黙って消さない」を守るため、確かめずに外す道は作りません。本体を読み直せなかったとき
  (`reload_base`) には出しません (開いているモジュールの物理を勝手に変えないため)。

- 付け替え (本体・スロット) は `open_module_with_base` で開き直し、保存するまでファイルに書きません (未保存の印)。選び直した本体でも開けなければ
  (スロットがないなど)、その本体で確かめた次の問題を出し、次の直し方でもその本体を使います。
- 「Open base project」は本体の `.ugoeproj` を開きます (モジュールは閉じる)。本体が `.ugoe` (書き出したもの) なら出しません。
  モジュールのプロジェクトの場所が分かれば (保存してある)、直した後に [モジュールに戻る](#モジュールに戻る) 帯から開き直せます。
- **付け替え (T0651):** 開いているモジュールの本体を読み直せなかったとき (本体が変わった・見つからない) の「Locate base…」「Use this slot」は、
  開き直さずに `reload_base(base, slot)` で付け替えます (`Document::rebase_module_on`。作業用のモデルを今の設定で分け、選んだ本体・スロットで合成し直す)。
  保存していない編集は残り、Undo の履歴は消えます。付け替えは保存するまでファイルに書かず、未保存の印が付きます。開けなかったプロジェクト
  (開く途中の問題) は、これまでどおり `open_module_with_base` で開き直します。
- **説明の文:** 本体のファイル名は引用符で囲みます (`The base model "moved.ugoeproj" no longer has slot "Hair".`)。ファイル名が文と続けて読めてしまうため。
- **読み取り専用で開く形は作らない:** 本体が無い・合わないと、モジュールの最上位の要素の座標 (取り付け先の局所座標) を表示できず、
  描画順も写せないため、モジュールだけを読み取り専用で見せても役に立ちません。代わりに直し方をすぐ選べるようにしました。

### 使えるもの・まだ無いもの

- **使える:** パーツ・デフォーマ・メッシュ・グルー・テクスチャ・法線マップ・物理 (リグと衝突体。重力などは本体のもの) の編集、PSD の取り込み
  (描画順をスロットの範囲に写す)、自動メッシュ・メッシュの編集、キーフォームの推定・ミラー。すべてモデルと同じ `Command` で、本体を変えない限り通ります。
  T0648 で確かめたのは、要素の追加・描画順・テクスチャの編集・PSD の取り込み・保存と開き直し・書き出し (テストとヘッドレスのブラウザ) です。
- **AI の提案** (PSD からのリギング) は、モジュールのプロジェクトではモジュール向けの提案になります
  ([モジュールのプロジェクトでの AI の提案](#モジュールのプロジェクトでの-ai-の提案)、T0517)。パラメータは足しません。
- **ポーズ:** `Module::pose` (グループと本体の要素への追従) は開いて保存すれば保たれます。「Poses」タブ ([ポーズの編集](#ポーズの編集)、T0521)
  と AI の提案 (衣装の差分 A / B、T0519) で作れます。
- **まだ無い:** 本体の中からモジュールを切り出す操作 (ADR 0024 で見送り)。名前・条件・スロットは [モジュールの設定](#モジュールの設定) で変えられます (T0650)。

## PSD の取り込み

`ugoe_editor_core::import::import_psd(model, bytes, options)` が、PSD (`ugoe-psd` の `LayerTree`) から
**1 つの `batch` コマンド** (Undo の 1 単位) と警告の一覧を作ります。モデルは書き換えず、コマンドを返すだけです
(エディタ・CLI・AI が同じ関数を使う)。読み込み済みの `LayerTree` からは `layer_tree_command` で作れます。

| PSD | ugoe |
| --- | --- |
| ドキュメント全体 | 最上位のパーツを 1 つ作り (名前は `options.name`。エディタと CLI はファイル名)、すべてをその下に入れる |
| キャンバス | モデルが空 (パーツ・メッシュ・デフォーマ・テクスチャがない) なら `set_canvas` で PSD の大きさにし、原点は中央。空でなければ変えず、大きさが違えば警告する (「モデルのキャンバスはそのままで、レイヤーは PSD のピクセル座標に置くので、キャンバスと揃わないことがある。PSD のキャンバスを使うには空のプロジェクトに取り込む」という文)。どちらでもレイヤーは PSD のピクセル座標に置く |
| グループ | パーツ (階層を保つ)。不透明度をパーツのキーフォームの不透明度にする。非表示なら不透明度 0。通過 (`pass`) と通常 (`norm`) 以外のブレンドモードは警告して無視する |
| ラスタレイヤー | テクスチャ 1 枚とメッシュ 1 つ。テクスチャはレイヤーの画像 (外接矩形で切り出したもの) の周りに `ceil(margin) + 1` px の透明な余白を足した PNG。メッシュは余白付きの画像から `ugoe-mesher` で作り、UV はそのテクスチャ上、頂点位置は「レイヤーの位置 − 余白 + メッシュの座標」(キャンバスのピクセル座標)。キーフォームは 1 つ (パラメータなし) |
| 不透明度 | メッシュの不透明度 = レイヤーの不透明度 × 塗りの不透明度 |
| 非表示のレイヤー | **取り込み、不透明度を 0 にする** (差分の絵 (閉じ目など) を後で使えるように)。元の不透明度は残らない |
| 描画順 | ドキュメント全体で下から上へ、`draw_order = 1000 × (i + 1) / (n + 1)` (`i` は下から数えた番号、`n` はメッシュの数)。`meshes` にも下から上の順に並べる。既存のモデルに取り込むと、既存のメッシュと描画順が混ざるので、必要なら後で並べ直す |
| ブレンドモード | `norm` → `normal`、`mul` → `multiply`、`lddg` (覆い焼き (リニア)) / `add` → `additive`。ほかは `normal` にして警告する |
| クリッピング | クリッピングされたレイヤーは、同じ並びの中で下にある最も近いクリッピングでない要素 (土台) を通常マスク (`MaskRef { invert: false }`) にする。土台がグループなら、その中のすべてのメッシュ (和集合)。土台のレイヤーの不透明度 (非表示なら 0) をクリッピングされた側の不透明度に掛ける (Photoshop では土台の不透明度が掛かるため)。下に何もなければ無視し (Photoshop と同じ)、土台を取り込まなかった (空のレイヤーなど) なら警告してマスクなしにする。グループのクリッピングは警告して無視する |
| 空のレイヤー・完全に透明なレイヤー・頂点が多すぎるレイヤー・大きすぎるレイヤー | 取り込まずに警告し、残りのレイヤーは取り込む。大きすぎるのは、自動メッシュの作業用の格子 (中身の外接矩形と余白) が `ugoe_mesher::MAX_GRID_CELLS` (2^28 セル) を超えるもの (`psd.layer_too_large`) |
| 画像が上限を超えるレイヤー | 余白を足したテクスチャの一辺が画像のデコードの上限 (`DEFAULT_MAX_IMAGE_SIDE`、16384 px。テクスチャの追加が拒む大きさ ([テクスチャ](#テクスチャ))) を超えるレイヤーは、取り込まずに警告し (`psd.layer_texture_too_large`)、残りのレイヤーは取り込む。上限ちょうどは取り込む。判定は画素を広げる前に寸法だけで行う。**縮小して取り込まない**のは、(1) 解像度が黙って変わり、差分の絵や後から差し替える画像と画素の対応がずれる、(2) 縮小した画像でメッシュと UV を作るとレイヤーの位置との写像が取り込みの規則 (テクスチャ上の 1 px = キャンバスの 1 px) から外れる、(3) 一辺が 16384 px を超えるレイヤーはまれで、利用者がペイントソフトで縮小すれば済み、警告でそれが分かるため (T1617) |
| 調整レイヤー・レイヤー効果など | `ugoe-psd` の警告をそのまま返す ([ugoe-psd](../crates/ugoe-psd/README.md#警告-layertreewarnings)) |

- ID は名前から `ids` の規則で作ります。名前が英数字を 1 つも含まなければ (日本語だけの名前など)、PSD のレイヤー ID から `Layer12` / `Group8` (ID がなければ `Layer` / `Group`) にします。表示名 (`name`) には元の名前を残します。
- テクスチャ名はメッシュの ID と同じにし、大文字小文字を区別せずに重なれば `_2`、… を付けます。
- 自動メッシュの設定 (`AutoMeshOptions`、JSON で省略したフィールドは既定値) は `ugoe_mesher::MeshOptions` と同じです。不正な値 (`MeshOptions::validate` と同じ規則。`margin` の上限 1024 を含む) なら、レイヤーを処理する前に取り込みをエラー (`PsdImportError::InvalidMeshOptions`。範囲の外のフィールドとその値を持つ) にします。

### 読み込みの上限

PSD は ZIP で圧縮したチャンネルが 1000 倍ほどに展開されうるので、`ugoe-psd` は展開する前に大きさを上限
(`ugoe_psd::ImportLimits`。チャンネル 1 枚が 1 GiB、1 つの PSD の合計が 4 GiB) と比べます ([ugoe-psd](../crates/ugoe-psd/README.md))。
`ugoe-psd` は上限の超過を `PsdError::LimitExceeded` (どの上限か (`ImportLimit`)・展開しようとした大きさ・上限の値) で返し (T0558)、
editor-core は `import::read_layer_tree(bytes, limits)` で読んで、それを変種で見分けて `PsdImportError::LimitExceeded` (どの上限か
(`PsdLimit`)・大きさ・上限の値) に分けます。コードは `psd.channel_limit_exceeded` (チャンネル 1 枚) と `psd.total_limit_exceeded` (合計) で、
引数 `size`・`max` (バイト) を持ちます。合計の `size` は超えたところまでの合計で、PSD 全体の合計より小さいことがあります
(超えたところで読むのをやめるため)。ほかの読み込みのエラー (壊れた PSD・対応しない機能) は `psd.read_failed` のままで、
元のエラーを原因 (`psd_read.*`。理由の種類ごとのコードと値。T0571) に載せます。エディタのバックエンドが AI の提案のために PSD を読むとき
(`psd.read_file_failed`) も同じく原因を載せます。

- **上限を変える設定は持たせません** (T0554)。エディタ (取り込み・新しいプロジェクト・AI の提案の PSD)・CLI・MCP サーバはどれも
  既定の上限で読みます。既定の値は PSD の最大のキャンバス (30000 × 30000) の 8 ビットのレイヤーの 1 チャンネルが収まる大きさで、
  上限に当たる PSD は、そのままではデスクトップのメモリでも扱いきれない大きさだからです。上げる設定は、メモリを使い果たす入力を
  通す手段になるので置きません。必要になったら、上限を超えた実例をもとに既定の値を見直します。ライブラリとして使う側は
  `read_layer_tree` に別の `ImportLimits` を渡せます。
- 展開の前に、PSD のファイルそのものの大きさも上限 `ugoe_psd::MAX_PSD_FILE_BYTES` (2 GiB。PSD の形式は長さを 32 ビットで持つ) までに
  抑えます。エディタ・MCP サーバ・CLI (`ugoe import-psd`) はどれもこの定数で `ugoe_fs::read_regular_file` を呼び、値は 1 か所にあります
  (T0568)。これも設定には出しません。

### メッシュの再生成

`regenerate_mesh { id, options }` は、メッシュのテクスチャの画像から自動メッシュで作り直します。

- すべてのキーフォームの頂点位置が同じ (キーフォームが 1 つの場合を含む) メッシュだけが対象です。キーフォームごとに頂点が動いていると、頂点が変わった後に変形を引き継げないので拒否します (`EditError::CannotRegenerateMesh`)。頂点番号を参照するグルーがあるメッシュ、頂点ごとの深度 (`depth`) を持つメッシュも拒否します。
- 頂点位置は UV のアフィン写像になっていなければなりません (取り込んだメッシュは「テクスチャの左上 + UV × 寸法」)。現在の UV と頂点位置からこの写像を最小二乗で求め (ずれが位置の広がりの 1e-4 倍を超えれば拒否)、新しい頂点の位置を決めます。親デフォーマの局所座標でも同じです。
- 各キーフォームの頂点位置以外の値 (不透明度・描画順・色など) と格子の軸は保ちます。
- `margin` を取り込み時より大きくすると、輪郭がテクスチャの余白の外 (UV が 0〜1 の外) に出ることがあります。

### メッシュの編集

メッシュの **トポロジーと基準の形** を編集するコマンドです (実装は `crates/ugoe-editor-core/src/mesh_edit.rs`)。キーフォームごとの形の編集 (パラメータのキーでの変形) は T0608 で扱います。

どのコマンドも、**既定値のパラメータで評価したメッシュ** (以下「基準の姿」。グルー適用後のキャンバス座標で、ビューポートに既定値で表示される形) を基準にします。評価はランタイム (`ugoe-runtime`) そのもので行います (モデルの複製を評価し、ランタイムが公開する局所座標の頂点 `mesh_local_positions` と親のワールド変換 `mesh_parent_world` を読む。[evaluation.md](spec/evaluation.md#デフォーマのワールド変換を読む))。モデルが評価できない (検証に通らない) ときは `EditError::MeshGeometry` で拒否します。

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
J       = ∂F/∂l (l での 2×2 のヤコビ行列。ランタイムが解析的に求める)
local   = J⁻¹ · delta                 // すべてのキーフォームの positions[i] に同じ量を足す
uv[i]   = 基準の姿のテクスチャの対応で、キャンバスの点 world[i] + delta に当たる UV
```

- 親なしは `J = I`。親が回転デフォーマ (と祖先の回転) ならワールド変換はアフィンなので、`J` は厳密にその線形部分 (回転・拡大率・反転) です。
- 親がワープなら、`J` はその点でのセル単位の双線形写像の微分です (格子の外では外挿のアフィン近似の微分。セルの境目では、写像と同じく `floor` で選んだセルの片側の微分)。双線形は一次式ではないので、大きく動かすと、評価後の位置は `delta` から交差項 (`∂²F/∂u∂v · Δu · Δv`) の分だけずれます。
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

### 描画用の複製

顔を大きく振ると、横髪や耳と輪郭の前後関係が入れ替わります。描画順をキーで切り替えると入れ替わる瞬間にちらつくので、
同じ絵を複製して輪郭を「サンドイッチ」し、マスクで片方ずつ見せるのが定番の作り方です
(参考: https://note.com/himono_vtuber/n/n19828b07b8a3 、https://note.com/himono_vtuber/n/ne6f7c0fdfdaa)。
複製に元と同じ変形を手で付け直すのは手間で、食い違いの元になるので、エディタは **形を元と共有する複製** を作ります
(実装は `crates/ugoe-editor-core/src/draw_clone.rs`)。モデルの形式は変えません。

| `op` | 内容 |
| --- | --- |
| `create_draw_clone { source, id?, name }` | `source` の複製を作る。`id` を省略すると `<元の ID>_Clone` (重なれば `_2`…)、`name` が空なら `<元の名前>の複製`。`source` が複製なら、その元の複製を作る |
| `sync_draw_clone { id }` | 複製を元に合わせ直す。`id` が複製ならその複製、元ならそのすべての複製。複製でも元でもなければ `EditError::NotDrawClone` |

**複製の中身。** 元のメッシュの値をすべて写し (パーツ・親デフォーマ・テクスチャ・UV・三角形・合成・カリング・マスク・`mask_only`)、
次の点だけを変えます。複製は `meshes` の中で元の直後 (同じ描画順なら元のすぐ手前) に置きます。

- キーフォームは、元の格子をパラメータの既定値で補間した **基準の形 1 つ** (軸なし)。描画順・不透明度・色・`mask_strength` もその値。
- `mirror`・`mirror_axis`・`depth` は持たない。

**形の共有。** 複製の全頂点 `i` を元の頂点 `i` に一致させるグルーを `glues` の末尾に加えます。

- ID は `DrawClone_<複製の ID>`、`mesh_a` が元、`mesh_b` が複製、ペアは全頂点の `(i, i, weight 0)`、強さ (`intensity`) は定数 1。
- [evaluation.md](spec/evaluation.md#5-グルー) の式で `weight = 0` なら `target = pa` なので、`pa' = pa`、`pb' = pb + (pa - pb) · 1 = pa`。
  複製の頂点は、元の頂点の **グルーの後** の位置 (浮動小数点の丸めで 1 ulp 程度の差はありうる) になり、元は動きません。
- 複製のキーフォームの頂点位置は描画に効きません。パラメータのキーを付けて描画順・不透明度だけを変えてかまいません。
- 動的物理の加算量 (デフォーマの角度・原点、ワープの制御点、メッシュの頂点) はすべてグルーの前に加わるので
  ([evaluation.md](spec/evaluation.md#動的物理を使う場合-2-段階の評価))、複製にもそのまま伝わります。複製自身に物理の出力を付ける必要はありません。
- 元が他のメッシュとグルーでつながっていても、複製のグルーはそれより後ろにあるので、複製はつながった後の位置に一致します。
- 不透明度は、複製自身のキーフォーム × 複製のパーツ × 複製の親デフォーマで決まります (元の不透明度は伝わりません)。
  親デフォーマは元と同じにそろえるので、デフォーマの不透明度は元と同じく掛かります。

**複製と元の対応** はグルーの ID の規則だけで表します (`draw_clone::original_of`・`clones_of`、フロントエンドは
`lib/panels/inspector/drawClone/drawClone.ts`)。ID が規則どおりでも、次のどれかを満たさない複製は「ずれている」とし
(`draw_clone::is_in_sync`)、インスペクタが合わせ直し (`sync_draw_clone`) を促します。

- UV・三角形・テクスチャ・親デフォーマが元と同じで、複製のすべてのセルの頂点数が元と同じ。
- グルーのペアが全頂点の `(i, i, weight 0)` で、強さがすべてのセルで 1。
- 元を使う他のグルーが、複製のグルーより後ろにない。

**元の編集への追従。** `Document::apply` は、コマンドの適用の後 (推定のやり直しと検証の前) に、UV・三角形・テクスチャ・親の
どれかが変わった元の複製を合わせ直します (`sync_draw_clone` と同じ処理)。コマンドの種類によらないので、頂点の移動・追加・削除、
三角形の作り直し、メッシュの再生成、親の付け替え (`reparent_preserving_look`・デフォーマの作成など) のどれにも追従します。
合わせ直しは同じ Undo の単位です。

- 合わせ直しは、複製の UV・三角形・テクスチャ・親を写し、すべてのセル (ブレンドシェイプ格子を含む) の頂点位置を元の基準の形にし、
  グルーのペアと強さを作り直して `glues` の末尾へ移します。複製の描画順・不透明度・色・マスク・パーツは変えません。
- 複製の形の編集 (`move_mesh_vertices`・`add_mesh_vertex`・`remove_mesh_vertices`・`retriangulate_mesh`・`regenerate_mesh`) は
  `EditError::DrawCloneShape` で拒否します。形は元を編集します。
- `regenerate_mesh` は頂点番号を参照するグルーのあるメッシュを拒否しますが、元の描画用の複製のグルーは例外です (作り直した後に複製ごと合わせ直すため)。

**画面。** インスペクタの「Draw clone」の節 (`DrawCloneSection.svelte`) で、メッシュの複製を作り (作った複製が選ばれる)、
複製から元へ、元から複製へ選択を移せます。複製を選ぶと、ずれていれば「Resync」と、今のセルの不透明度・描画順の欄 (`set_keyform_cell`。描画順のキーを作るため) が出ます。
マスクは「Mesh」の節で、普通のメッシュと同じく編集します。

## キーフォームの編集

Live2D のリギング作業の中心: パラメータを作り、要素にキーを追加し、各キー (セル) の上で形を編集します。

### コマンド

要素は `target` で種類と ID を指定します: `{"kind": "part" | "deformer" | "mesh" | "glue", "id": "..."}` (`ugoe_editor_core::KeyformTarget`)。デフォーマはワープ・回転のどちらでもよく、格子の種類はデフォーマから決まります。

| `op` | 内容 |
| --- | --- |
| `add_keyform_key { target, parameter, key }` | 格子の `parameter` の軸にキー `key` を加える。軸がなければキー 1 つの軸を末尾に加える (セルの並びは変わらない)。軸があれば、新しいセルを既存の格子のその値での補間 ([evaluation.md](spec/evaluation.md#1-キーフォームの重み) と同じ重み。キーの範囲の外は端のセルの値。曲線補間の格子では形の値を曲線の値にする) で埋めるので、**キーの位置の評価の結果 (見た目) は変わらない** (曲線補間の格子では、接線が変わるのでキーの間は少し変わる)。真偽値のフィールド (`reflect_x` など) は重みの大きい側 (同じなら小さいキーの側) の値。新しいセルの由来は [spec/keyform-inference.md#キーの追加とセルの由来](spec/keyform-inference.md#キーの追加とセルの由来): 組み合わせのセル (中立でない軸が 2 本以上) は、加算合成の格子ならその場で推定して、見た目が変わらなければ `additive` にする。それ以外のセルは、埋めた元のセルが推定で作ったセルなら `derived`、なければ `authored` (中立のキーがない軸がある格子では、格子に推定で作ったセルがあれば `derived`)。同じキーがすでにある (`DuplicateKey`)、有限でない、パラメータがない場合は拒否する |
| `remove_keyform_key { target, parameter, key }` | 軸からキーとその断面のセル (と由来) を取り除く。軸の最後のキーなら軸ごと取り除き、そのキーの断面が残る (`remove_parameter` と同じ畳み方) |
| `set_keyform_cell { target, cell, keyform }` | 1 セルを置き換え、そのセルを `authored` にする。`cell` は `set_mesh_keyform` と同じ「軸のパラメータ ID → キーの値」。`keyform` は `{"kind": "mesh" \| "warp" \| "rotation" \| "part" \| "glue", ...フィールド}` で、種類は格子と一致させる (`KeyformKindMismatch`)。頂点 (制御点) の数は変えられない (`ShapeMismatch`) |
| `set_mesh_draw_order { id, draw_order }` | メッシュの格子の **すべてのセル** の `draw_order` を同じ値にする (有限の値でなければ `InvalidDrawOrder`)。セルの由来と他の値は変えず、すべて同じ値なので作り直しもしない。描画順の違うメッシュどうしの前後を入れ替えるのに使う (`reorder_mesh` は描画順が同じメッシュの間でしか効かない)。セルごとに変えるなら `set_keyform_cell` |
| `set_keyform_interpolation { target, blend_shape, interpolation }` | 格子のキーの間の補間の種類 (`linear` / `smooth`、[spec/evaluation.md#曲線補間](spec/evaluation.md#曲線補間)) を設定する。`blend_shape` (パラメータ ID、省略可) を指定すると、そのパラメータを軸に持つブレンドシェイプ格子に設定する (なければ `NotFound`)。セルの値と由来は変えないので作り直しもしない。未知の値と、パーツ・グルーの格子への `smooth` は `UnsupportedInterpolation` で拒否する |

`set_keyform_cell` の後は、その格子の `derived` のセルを同じ Undo の単位の中で作り直します ([推定とミラー](#推定とミラー))。`add_keyform_key` と `remove_keyform_key` は既存のセルを作り直しません (追加は補間で見た目を保ち、削除は残ったセルの値を変えないため)。`add_keyform_key` がその場で推定するのは新しい組み合わせのセルだけです。

### ブレンドシェイプ

`kind = blend_shape` のパラメータ ([model.md#ブレンドシェイプ](spec/model.md#ブレンドシェイプ)) は `add_parameter` でふつうに作れます。キーとセルの編集は、同じコマンドがパラメータの種類で格子を振り分けます (T0307)。

- `add_keyform_key`: ブレンドシェイプのパラメータなら、要素の格子の `blend_shapes` のうち、そのパラメータを軸に持つ格子にキーを加えます。なければ、軸がそのパラメータ 1 本で、キーが既定値と `key` の 2 つ (同じなら 1 つ)、セルがすべて **基準の形** (通常の格子をパラメータの既定値で補間した形) のブレンドシェイプ格子を末尾に加えます。差分は 0 なので見た目は変わりません。その後 `key` のセルに目的の形を置けば、その形がそのまま差分になります。組み合わせのセルの推定はしません。
- `remove_keyform_key`: そのパラメータを軸に持つブレンドシェイプ格子からキーを取り除きます。軸がなくなった格子は取り除きます。
- `set_keyform_cell` と `set_mesh_keyform`: `cell` にブレンドシェイプのパラメータがあれば、そのパラメータを軸に持つブレンドシェイプ格子のセルを指します (ほかの軸もその格子の軸で指定する)。なければ通常の格子のセルです。
- `remove_parameter`: ブレンドシェイプ格子の軸も既定値の断面に畳み、軸がなくなった格子は取り除きます。
- 頂点数・座標系を変える編集 (`move_mesh_vertices`・`add_mesh_vertex`・`remove_mesh_vertices`・`regenerate_mesh`・デフォーマの作成と付け替え・`move_rotation_origin`) と `set_mesh_draw_order` は、ブレンドシェイプ格子のセルにも同じ操作をします。
- 推定 (`infer_keyforms`・`mirror_keyforms`・由来の変更) は通常の格子だけを扱い、ブレンドシェイプ格子は変えません。
- パーツとグルーはブレンドシェイプ格子を持てないので、それらへのブレンドシェイプのキーの追加は検証で拒否します。画面での操作 (ブレンドシェイプ格子の表示・編集) はまだありません。

### 座標の変換

キーフォームの頂点・制御点・原点は親の座標系 (親なしならキャンバス座標、ワープの子は UV 空間、回転デフォーマの子は原点を (0,0) とした座標) で表されます。ビューポートでの操作 (キャンバス座標) を書き込むには、`ugoe_editor_core::space` で変換します。

- `local_to_canvas(model, target, values, points)`: 親の座標系 → キャンバス座標。親の祖先のデフォーマだけを残したモデルをランタイム (`ugoe-runtime`) で評価し、ランタイムが公開する親のワールド変換 (`deformer_world`、[evaluation.md](spec/evaluation.md#デフォーマのワールド変換を読む)) で写すので、変換はランタイムの評価と必ず一致します。
- `canvas_to_local(model, target, values, points, guesses)`: 逆変換。ランタイムの解析的なヤコビ行列でニュートン法を回します (初期値はドラッグ前の位置)。アフィンな親 (回転デフォーマ) なら 1 回で、ワープでも数回で収束します。親の変換が潰れていれば `Singular`、収束しなければ `NotConverged` を返します。
- `values` はビューポートの表示中のパラメータ値です (パラメータ値はドキュメントの状態ではないので、フロントエンドが渡す)。
- Tauri コマンドは `local_to_canvas(target, parameters, points)` と `canvas_to_local(target, parameters, points, guesses)` (`src-tauri/src/space.rs`)。点は `[x, y]` の配列です。

### 画面での操作

| 場所 | 操作 |
| --- | --- |
| パラメータ一覧 (`ParametersPanel.svelte`) | 「+」でパラメータを作る (ID・名前・最小・既定・最大)。名前を押すと名前・値域・既定値の編集と削除 (`rename_parameter` と `set_parameter_range` を `batch` で 1 つの Undo に)、グループの「Group」(選ぶとすぐ `set_parameter_group`)、左右の対の「Mirror」(なし / 自分自身 / 相手。選ぶとすぐ `set_parameter_mirror`。標準の命名から対を提案する。`panels/parameters/`)、「Relay (internal)」(切り替えるとすぐ `set_parameter_internal`)。中継用のパラメータは一覧に残し、名前を斜体にして「relay」の印を付ける。スライダーの下に、主な選択の要素のキーを ◆ で示し (押すとそのキーへ。押せる範囲は 24px で、隣の印と重なる所では近い方のキー)、「Snap to keys」で要素のすべての軸を最も近いキーへ動かす。行の値の欄に数を入力して Enter (またはフォーカスを外す) でスライダーをその値にする (数でない・値域の外なら理由を出して拒否)。行の ◆ ボタンで、今の値にキーを追加 (今の値がキーならそのキーを削除。キーの上とみなす許容差の中に別のキーがあれば拒否)。グループに入ったパラメータがあれば一覧をグループの順に区切り (折りたためる見出し。グループの無いものは最後の「Other」)、一覧の下の「Groups」でグループを作る・名前を変える・消す・並べ替える (行のドラッグ・▲▼・Alt + ↑↓。[パラメータのグループ](#パラメータのグループ))。スライダーは表示の状態なので IPC を通らない |
| インスペクタ (`inspector/keyforms/KeyformInspector.svelte`) | 主な選択の要素の軸とキー (× で削除、押すとそのキーへ移動、「+ 値」で今の値にキーを追加)、軸のないパラメータへの軸の追加、セルの格子 (1 軸は 1 行、2 軸は表、3 軸目以降は今の値に最も近いキーの断面)。メッシュでは描画順を表示し、「Set for all cells」で `set_mesh_draw_order` を送る。回転デフォーマでは今のセルの「Angle (this cell)」(度) と「Scale (this cell)」の数値の欄を出し、確定でギズモと同じ `set_keyform_cell` を送る (ほかの値はそのセルのまま。拡大率は 0.001 以上。同じ欄の続けての確定は 1 つの Undo。キーの間では欄を空にして押せなくする)。メッシュ・ワープ・回転では「Smooth curve between keys」のチェックで `set_keyform_interpolation` (`smooth` / `linear`) を送る (未知の値なら線形として扱うことを添える。ブレンドシェイプ格子の切り替えはまだ画面にない)。セルは記号と色で由来を示し ([推定の画面での操作](#推定の画面での操作))、今の値がすべての軸でキーの上にあればそのセルを枠で示す。キーの上になければ「Move to nearest key」を出す |
| ビューポート (`viewport/tools/form/FormTool.svelte`) | 「Form edit」で切り替える。主な選択の要素のうち、メッシュは頂点、ワープは制御点、回転デフォーマは原点をハンドルで示す (主な選択がデフォーマのときは、代わりに [デフォーマのギズモ](#デフォーマの編集) を出す)。点のクリックで選び (Shift で追加・解除)、要素の内側 (メッシュの三角形・ワープの格子) の何もない所からのドラッグで矩形選択 (Shift で追加)、クリックで選択を解く。要素の外側のクリックは下の選択レイヤー (メッシュの選択) に通す。「All」/ Ctrl + A ですべて、Esc で解く。選んだ点のドラッグで選んだ点をまとめて同じだけ動かす (選んでいない点を押すと、その点だけを選んで動かす)。ドラッグ中はオーバーレイだけでプレビューし、離したときに `canvas_to_local` で親の座標系に直してから `set_keyform_cell` を 1 つ送る (Undo の 1 単位)。選択は要素ごとにフォーム編集の中だけで持つ (選択の補助は `viewport/tools/form/form.ts`) |

- **キーの上でだけ編集できる:** 今のパラメータ値がどれかの軸でキーの上にないと、どのセルに書くか決まらないので編集を拒否し (ハンドルを出さない)、「Move to nearest key」を出します。
- キーの上かどうかは、パラメータの値域の 1e-6 倍 (最小 1e-6) の差まで同じとみなして判定します (スライダーの値は wasm の `f32` を経由するので、`0.1` のようなキーと完全には一致しない)。送るセルの値はキーの値そのものです。
- 判定などの純粋な関数は `inspector/keyforms/keyforms.ts` にあり、Vitest で検査します (`pnpm --filter @ugoe/editor test`、`just web-check` に含まれる)。

## デフォーマの編集

デフォーマを作るコマンドと、見た目を保つ親の付け替えです (実装は `crates/ugoe-editor-core/src/deformer_edit.rs`、テストは `deformer_edit_tests.rs`)。
[メッシュの編集](#メッシュの編集) と同じく、**既定値のパラメータで評価した姿** (基準の姿) を基準にします。基準の姿での各デフォーマのワールド変換 `F_D` ([evaluation.md](spec/evaluation.md#3-デフォーマのワールド変換)) は、モデルの複製をランタイムで 1 回評価し、ランタイムが公開するワープの制御点 (`warp_world_points`) と回転デフォーマのアフィン変換 (`rotation_world_frame`) から作ります ([evaluation.md](spec/evaluation.md#デフォーマのワールド変換を読む))。写像・ヤコビ行列・ワープの子の回転デフォーマの変換は、作ろうとしているデフォーマの分もランタイムの `WorldTransform` で求めます (逆写像のニュートン法だけをエディタが持つ)。

| `op` | 内容 |
| --- | --- |
| `create_warp_deformer { id?, name, children?, parts?, rows, cols, parent? }` | `children` (`{"kind": "mesh" \| "deformer", "id"}` の配列) と `parts` のパーツの要素 (下) を包むワープ (`rows` × `cols` セル、1〜64) を作り、子を付け替える |
| `create_rotation_deformer { id?, name, children?, parts?, origin?, parent? }` | `children` を包む回転デフォーマ (角度 0・拡大率 1) を作り、子を付け替える。`origin` はキャンバス座標で、省略すると子の外接矩形の中心 |
| `reparent_preserving_look { element, new_parent }` | メッシュかデフォーマ (`element`) の親デフォーマを `new_parent` (`null` で最上位) にし、キーフォームを新しい親の座標系へ写し直す |
| `move_rotation_origin { id, delta }` | 回転デフォーマ `id` の格子の **すべてのセル** の原点に `delta` (親の座標系) を足し、直接の子を基準の姿で見た目が変わらないように写し直す (下の「回転の中心の移動」) |
| `unwrap_deformer { id }` | デフォーマ `id` を外す: 直接の子を `reparent_preserving_look` と同じ規則でデフォーマの親へ付け替え、デフォーマを `remove_deformer` と同じ規則で削除する (ミラーの対も外す)。1 つの Undo の単位 |

- **パーツを包む:** `parts` のパーツ (と子孫のパーツ) に属するメッシュ・デフォーマのうち、同じ集まりのデフォーマ (と `children` のデフォーマ) の下にない、いちばん外側のものを子に加えます (`children` と重なるものは 1 つにする)。パーツの中のデフォーマの下にある要素はそのデフォーマごと移るので、階層は保たれます。パーツの外のデフォーマの下にある要素も、パーツの中でいちばん外側なら子になります (新しいデフォーマは、子の親の最も近い共通の祖先の下にできる)。包むものがなければ `DeformerEdit`、存在しないパーツは `NotFound` で拒否します。
- **子の中の子:** `children` (とパーツから加えた子) に、ほかの子のデフォーマの下にある要素があれば、その要素は付け替えません (そのデフォーマごと移るので、階層は保たれる)。
- **名前と ID:** `name` が空なら、最初のパーツ (なければ最初の子) の名前から作ります。名前に ASCII 以外の文字 (日本語のレイヤー名など) があれば `顔のワープ` / `顔の回転`、なければ `Face warp` / `Face rotation` です。`id` を省略すると、名前に英数字があれば名前から (`Face_warp`)、なければ種類とその要素の ID から (`WarpLayer8`・`RotationHead`。要素がこの規則で作ったデフォーマなら種類の接頭辞を外して `RotationLayer8`) 作り、重なれば `_2`、`_3`… を付けます (`ids::new_deformer_id`)。
- **作るデフォーマ:** `parent` を省略すると、子の親の最も近い共通の祖先 (共通の祖先がなければ最上位)。パーツは子のパーツの最も近い共通の祖先 (なければ最初の子のパーツ)。格子は定数 (軸なし) です。
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

### 回転の中心の移動

キーを作った後で回転デフォーマの原点をギズモで動かすと、今のセルの原点だけが変わります (そのセルで子ごと平行移動する。動きを付けるための編集)。リガーが「回転の中心を置き直したい」ときに期待するのは、**絵を動かさずに中心だけを動かす** ことなので、`move_rotation_origin` を用意しました。

- すべてのセルの原点に同じ `delta` (親の座標系) を足します。セルどうしの原点の違い (動き) は保たれます。
- 直接の子は、回転デフォーマの基準の姿のワールド変換を `F_old` (動かす前)・`F_new` (動かした後) として、[子の写し直し](#デフォーマの編集) と同じく `p' = F_new⁻¹(F_old(p))` で写し直します (どちらもアフィンなので厳密)。孫は変えません。
- 子の格子は回転デフォーマの格子と軸が別なので、補正できるのは基準の姿だけです。基準の姿 (既定値のパラメータ) では見た目が変わらず、他のセルでは子が **新しい中心のまわりに** 回ります (基準の姿の角度が 0・拡大率 1 なら、角度 θ のセルでの子のずれは `(I - R(θ))·delta`)。これが中心を動かしたときに期待する動きです。
- 回転デフォーマでない (`DeformerKindMismatch`)、`delta` が有限でない (`DeformerEdit`) なら拒否します。
- 画面: ギズモのバーの「Origin: all cells」を付けるか、Shift を押しながら原点をドラッグすると、今のセルでの移動量 (`canvas_to_local` で親の座標系に直した原点の差) をこのコマンドで送ります。

### ギズモとデフォーマの作成 (画面)

| 場所 | 操作 |
| --- | --- |
| インスペクタ (`inspector/DeformerProperties.svelte`) | 選択中のメッシュ・デフォーマ・パーツ (複数可。パーツは `parts` で、その中のいちばん外側の要素) を「Wrap in deformer」で包む。「Name」(空なら上の規則で自動) と行数・列数を指定して「Warp」、または「Rotation」(原点は選択の中心)。作ったデフォーマを選択する。デフォーマを選ぶと種類・格子・キーフォームの数・親 (表示名と ID) を示し、「Unwrap」で `unwrap_deformer` を送って元の子を選択する |
| ビューポート (`viewport/tools/deformer/DeformerGizmo.svelte`) | 主な選択がデフォーマのとき、フォーム編集の代わりに出す。ワープは格子と制御点を示し、クリックで選択 (Shift で追加・解除)、格子の内側からのドラッグで矩形選択 (Shift で追加)、「All」/ Ctrl + A ですべて、選んだ点のドラッグでまとめて動かす。回転デフォーマは原点 (円)・回転の取っ手 (局所座標の上の向き)・拡大率の取っ手 (局所座標の右の向きの四角) を示し、原点の移動・回転 (Shift で 15 度刻み)・拡大率の変更 (原点からの距離の比) を行う。原点の移動は今のセルだけで、「Origin: all cells」か Shift + ドラッグなら [すべてのセル](#回転の中心の移動) |
| ツリー | デフォーマ・メッシュを別のデフォーマへドラッグすると `reparent_preserving_look` |

- ギズモの形は wasm の `WebPuppet.warp_points(id)` (キャンバス座標の制御点) と `rotation_frame(id)` (`[ox, oy, ax, ay, bx, by]`: 原点と局所座標の x 軸・y 軸の単位ベクトルの像) から読みます (今のパラメータ値。`deformer_ids()` で一覧)。wasm は表示中のインスタンスを評価し、ランタイムが公開するワールド変換 (`warp_world_points`・`rotation_world_frame`) をそのまま返します (`crates/ugoe-wasm/src/gizmo.rs`)。
- 編集は [フォーム編集](#キーフォームの編集) と同じく **今のパラメータ値のセル** に `set_keyform_cell` で書き込み (離したときに 1 つ、Undo の 1 単位)、キーの上にないときは編集を拒否して「Move to nearest key」を出します。角度と拡大率は、インスペクタの「Keyforms」の数値の欄 ([キーフォームの編集](#キーフォームの編集)) でも同じコマンドで入れられます。制御点と原点は `canvas_to_local` で親の座標系に直します。角度は画面上の回転角を足します (親や自分の反転で向きが裏返っていれば符号を変える)。拡大率は正の値に限ります。
- ドラッグ中はギズモだけをプレビューし、子の描画は離して読み直すまで変わりません。
- 幾何の補助 (格子の辺・外周・取っ手の位置・角度・拡大率) は `viewport/tools/deformer/gizmo.ts` にあり、Vitest で検査します。

### はみ出しの検証

ワープの子 (メッシュの頂点・子のワープの制御点・子の回転デフォーマの原点) は、親の UV 空間で表します。UV が [0,1]² の外の点は
[範囲外の外挿](spec/evaluation.md#範囲外の外挿) で親に追従するので壊れはしませんが、格子の中ほど思いどおりに曲がりません。
高可動域のリグでは、パラメータを動かしたときにこの「はみ出し」がよく起きるので、エディタで示します (T0632。Live2D の実践で言う
「親デフォーマからはみ出した頂点のハイライト」と「デフォーマの検証」に当たる)。読むだけで、コマンドは発行しません。

- **検出** (`ugoe_editor_core::overflow`): 子の局所座標 (= 親の UV) は子自身のキーフォームだけで決まり、親の変形には依らないので、
  子の格子とブレンドシェイプ格子を、ランタイムと同じ重み (`ugoe_runtime::keyform::weights`。格子の `interpolation` に従い、
  [曲線補間](spec/evaluation.md#曲線補間) の軸では曲線の重み) で補間して調べます (メッシュは `ModelInstance::mesh_local_positions`
  と、ワープの制御点は親のワールド変換で写した `warp_world_points` と一致することをテストで確かめている)。はみ出しの量は [0,1]² からの UV の距離
  (格子の幅・高さを 1 とした量) で、既定の許容距離 `DEFAULT_TOLERANCE` (0.001) を超える点を数えます。親が回転デフォーマ・親なしの
  要素は UV 空間を持たないので調べません。
  - `overflows_at(model, values, tolerance)`: 今のパラメータ値 (含まないものは既定値) で、はみ出した点を要素ごとに返す
    (`ElementOverflow { target, parent, kind, points: [{ index, uv, amount }] }`。`kind` は `vertex` / `control_point` / `origin`)。
  - `verify_overflows(model, tolerance)`: 親がワープのすべての要素について、格子とブレンドシェイプ格子の軸のパラメータの
    **キーのすべての組み合わせ** (セル) を調べ、はみ出したセルごとに `OverflowFinding { target, parent, kind, cell, count,
    worst_index, max_amount }` を返す。補間はセルの中で各軸について線形なので、凸関数である UV の距離の最大はセルの角 (キーの
    組み合わせ) で起きます。キーの外の値はクランプされるので、軸の端の組み合わせも両端のキーの組み合わせとして含まれます。
    曲線補間の軸はキーの間で行き過ぎる (オーバーシュート) ことがあるので、隣り合うキーの間の `SMOOTH_SAMPLES` (3) 点も値に
    加えます (間の点は近似。見つけたセルの値はその点になる)。
    1 要素の組み合わせが `MAX_COMBINATIONS` (4096) を超えるときは、各軸の両端のキーだけを調べ、`truncated` に入れます。
    サンプルのアバター (32 要素・269 セル) で 1 ms もかかりません。
- **Tauri コマンド** (`src-tauri/src/overflow.rs`): `find_overflows(parameters, tolerance?)` と `verify_overflows(tolerance?)`。
- **ビューポート** (`viewport/tools/overflow/OverflowOverlay.svelte`): 「Verify」のパネルの「Show overflows in viewport」
  (`VerifyView.showOverflows`) の間、今のパラメータ値ではみ出した点に赤い印を重ねます。どの点かは `find_overflows` に問い合わせ
  (同時に 1 つだけ。その間に変わった値は、終わってから最新のものだけを問い合わせる)、位置は wasm の評価結果 (メッシュは
  `drawable_positions`、ワープは `warp_points`、回転デフォーマは `rotation_frame` の原点) を読みます。操作は受けません。
- **「Verify」のパネル** (`panels/verify/VerifyPanel.svelte`、右の列のタブ): 「Verify all cells」で `verify_overflows` を呼び、
  要素ごとに (最大の量の大きい順) セル・点の数・量を並べます。行を押すと、その要素を選び、ビューポートのパラメータをセルの値に
  し (ほかのパラメータは変えない。物理のプレビュー中なら止める)、はみ出しの表示を出します。一度調べた後は、モデルが変わるたびに
  調べ直します。一覧の組み立て (まとめ方・表記・印の位置) は `panels/verify/overflow.ts` にあり、Vitest で検査します。

## テクスチャ

- `Document` はモデルと一緒に **テクスチャの画像** (`Textures` = テクスチャ名 → PNG のバイト列) を持ちます。モデルの `TextureRef` は名前と寸法だけで、画像はここにあります。画像はデコードせず、PNG ファイルのまま持ちます。
- 画像の追加・差し替え・削除もコマンド (`add_texture` など) で行うので、Undo/Redo の対象になります。[PSD の取り込み](#psd-の取り込み) はレイヤーごとに `add_texture` と `add_mesh` を `batch` にまとめて発行します。
- スナップショットでは画像を `Arc<[u8]>` で共有するので、Undo のたびに画像のバイト列が複製されることはありません。
- テクスチャ名は `textures/<name>.png` のファイル名になるので、`[A-Za-z0-9_.-]` だけからなり `.` で始まらないものに限り、大文字と小文字を区別せずに一意にします ([ファイル形式](spec/file-formats.md#名前と読み込み))。
- `png` は JSON では数値の配列になります。大きな画像をフロントエンドから送ることは想定しておらず、バックエンドの中 (取り込み処理) から使います。
- 画素が要る処理 (メッシュの作り直し・法線マップ・アトラス化) は、PNG を `ugoe_format::image` でデコードします (ランタイムと同じ展開と上限。`texture::decode_png`)。一辺が上限 (既定 16384 画素。[読み込みの上限](spec/file-formats.md#読み込みの上限)) を超える画像は、画素を確保する前に `*.texture_too_large` / `lighting.normal_map_too_large` のエラーで拒みます (T0162)。
- テクスチャと法線マップの画像を置くコマンド (`add_texture`・`set_texture_image`・`set_normal_map` の `png`) は、一辺が同じ上限を超える PNG を置く前に `edit.texture_too_large` で拒みます (T0560。IHDR の寸法だけを見るので画素は確保しません)。
  上限を超える画像はビューポート (`ugoe-wasm`)・ランタイム・画素が要る処理のどれも読めないので、置けても使えないからです。PSD の取り込みも `add_texture` を通るので、上限を超えるレイヤーがあると取り込み全体がこのエラーになります。
  プロジェクトを開くときは画像を確かめないので、以前に置いた上限を超える画像を持つプロジェクトも開けます (使う処理がそれぞれのエラーを返します)。

### 法線マップ

判断の経緯は [ADR 0015](adr/0015-lighting.md)、描き方は [rendering.md#ライティング](spec/rendering.md#ライティング)。

- 法線マップの画像はテクスチャの画像と同じ表 (`Textures`) に、`TextureRef::normal_map` の名前で置きます (プロジェクトでは `textures/<normal_map>.png`。T0640)。
- `set_normal_map` は `texture` (テクスチャの名前) の法線マップを設定・差し替え・外します。
  - `normal_map` と `png`: 画像を `normal_map` の名前で置きます。すでにその名前の画像があれば差し替えます (その画像を指すほかのテクスチャにも効く)。
  - `normal_map` だけ: ほかのテクスチャがすでに指している法線マップの画像を共有します (画像がなければ `NotFound`)。
  - どちらもなし (`null`): 法線マップを外します。`png` だけを渡すと拒否します (`InvalidNormalMap`)。
  - 名前はテクスチャの名前と同じ規則で、テクスチャの名前と (大文字小文字を区別せずに) 重なってはいけません (`DuplicateId`)。ほかの法線マップの名前とは、大文字小文字まで一致するか、まったく違う名前にします。
  - どのテクスチャからも指されなくなった法線マップの画像は表から除きます。Undo で画像も戻ります。
- `set_lighting` はモデル全体のライティングの有無 (`Model::lighting`) を変えます。メッシュごとの `lit` (既定 `true`) と両方が `true` のメッシュだけがライティングを受けます。
- `set_mesh_properties` の `lit` でメッシュごとの有無を変えます (目のハイライトなど、陰影を付けたくない部分を外す)。
- 法線マップの生成 (アルファの形・深度推定から) は `ugoe_ai::normal` が `set_normal_map` と `set_lighting` の列を提案します ([ai-integration.md#法線マップの生成](ai-integration.md#法線マップの生成))。
  エディタの画面からは、AI を使わないアルファの形からの生成 (部位のまとまりごと) だけを [ライティングの画面](#ライティングの画面) で行います。

### ライティングの画面

T0646。コードは `src/lib/panels/lighting/` (パネル)・`src/lib/viewport/tools/lighting/` (取っ手)・`src-tauri/src/lighting.rs`、
コマンドの組み立ては `ugoe_editor_core::lighting` (画像の割り当て) と `ugoe_ai::normal` (生成)。操作の手順は [manual/lighting.md](manual/lighting.md)。

- 右の列の「Lighting」タブ。モデルの「Receive lighting」(`set_lighting`) と、メッシュごとの「Lit」(`set_mesh_properties { lit }`。
  選んだメッシュか、選んでいなければすべてを「Light selected」「All on」などでまとめて `batch` で変える) を置きます。
  モデルがライティングを受けない (`Model::lighting` が偽か、`lit` のメッシュがない) ときは、光源を動かしても見た目が変わらない理由を出します。
- **法線マップ:** 主な選択のメッシュのテクスチャについて、テクスチャと法線マップの画像を並べて出し (`get_texture_png(name)` で PNG を読む。編集のたびに `get_texture_versions(names)` で画像の版 (中身のハッシュ) を問い合わせ、変わった画像だけを読み直す)、
  - 「Generate」(`generate_texture_normal_map(texture, settings, whole_group)`): 部位のまとまりの形のふくらみ (`ugoe_ai::normal::propose_bulge_normal_maps`。
    AI は使わない。[ai-integration.md#まとまりでふくらませる](ai-integration.md#まとまりでふくらませる))。まとまりは意味ラベルから決めます。ラベルはメッシュの名前の規則
    (`ugoe_ai::classify::label_meshes_by_name`) に、AI の提案のパネルで PSD を分類していればその結果 (`rig::mesh_parts`) を重ねたものです (T0518)。
    - `get_normal_map_group(texture)` (`NormalMapGroupView`) で、まとまりの名前・いっしょにふくらむテクスチャ・形にだけ加えるテクスチャ・向きだけを受けるテクスチャ
      (眼鏡。選んだテクスチャがそれなら「Takes the direction of group “face” … without bulging」)・既定の設定を出し、
      強さ (0〜2) と「Round edges only」の縁の丸みの幅・ならし (「Smoothing」) は、テクスチャを選ぶたびにまとまりの既定から始めます (「Defaults」で戻す)。
    - **長さの単位 (T0522):** `BulgeSettings` の `edge`・`smoothing` はまとまりの格子 (形に加わるテクスチャのうち最も細かいものの 1 画素) の画素ですが、
      画面は **キャンバスの px** で見せます (「Edge width (px)」「Smoothing (px)」)。`NormalMapGroupView::canvas_per_pixel` (格子の 1 画素のキャンバスの px) を掛けて見せ、
      送るときに割ります (`src/lib/panels/lighting/lighting.ts` の `bulgeForm`・`bulgeSettings`)。見本のアバターの顔では格子の 144 画素がキャンバスの 48 px
      (虹彩のテクスチャが細かいため、格子の 1 画素がキャンバスの約 0.33 px)。単独でふくらませるもの (`canvas_per_pixel` が `null`) はテクスチャの画素のままで、「(texture px)」と出します。
    - `settings` (`BulgeSettings { strength, edge, smoothing }`) はまとまり全体の設定です (`null` ならまとまりの既定)。「Regenerate」は選んだテクスチャの法線マップだけを、
      「Generate for the group」(`whole_group`) はまとまりのすべてのテクスチャの法線マップを作ります。どちらも形はまとまり全体で決まるので、1 枚ずつ作っても同じ向きになります。
    - `ugoe_editor_core::lighting::generate_normal_map` (テクスチャ単独のふくらみ) は画面からは使いません。
  - 「Assign PNG…」(`assign_texture_normal_map(texture, path, flip_green)`): 開くのダイアログで選んだ PNG を付けます。「Flip green (DirectX)」は G を反転します。
  - どちらも `set_normal_map` (モデルのライティングが無効なら `set_lighting { enabled: true }` も) を 1 つの `batch` にして適用するので、1 回の Undo で画像ごと戻ります。
    画像の名前は、今の法線マップをこのテクスチャだけが指していればその名前 (差し替え)、共有していれば `<texture>_normal` (重なれば `_normal2`…) です (`lighting::normal_map_name`)。
  - 「Remove normal map」は `set_normal_map { normal_map: null }`。
- **プレビューの光源:** 光源はモデルに入らない (ホストが渡す、[rendering.md#ライティング](spec/rendering.md#ライティング)) ので、コマンドではなく
  `project.json` の `editor.lighting` (`PreviewLighting`、[file-formats.md](spec/file-formats.md#プロジェクト-ugoeproj)) にプロジェクトの表示の設定として残します
  (`get_preview_lighting` / `set_preview_lighting`。Undo の対象ではなく、変えると未保存の印が付く。既定値なら書かない)。
  - 既定は、環境光 0.45 と左上の手前からの白い平行光源 1 つ (強さ 0.7)。色 (0〜1) と強さを別に持ち、`WebPuppet.set_lighting_json` には掛けた色を渡します (`lighting.ts` の `lightingJson`、Vitest)。
  - `LightingView` (`lightingView.svelte.ts`) が設定を持ち、`App.svelte` が `ViewportController.setLighting` でいつもビューポートに渡します
    (編集中もプレビュー中も。「Light the viewport」を外すと光源を渡さない)。`ViewportController` は WebPuppet を作り直しても渡し直し、
    読み込むたびに `uses_lighting` を `usesLighting` に写します。
  - 取っ手 (`LightingOverlay.svelte`) は「Lighting」タブを開いている間とプレビュー中に出します。平行光源はキャンバスの中央の円の上で、光源のある向き
    (`l = −normalize(direction)` の xy。中心が正面から、縁が真横から) に置き、ドラッグで向きを変えます (「From behind the canvas」で裏から)。
    点光源はキャンバスの上の位置に置き、ドラッグで動かします (奥行き z と届く距離 `range` はパネルの欄)。ドラッグ中は表示だけを変え、離したときに残します。
  - プレビューのパネルにも同じ光源の節を出します (モデルがライティングを受けるときだけ)。


## エフェクトの設定

- `set_effects` は目パチ・呼吸・リップシンクの設定 (`Model::effects`、[motion.md#エフェクト](spec/motion.md)) を丸ごと置き換えます。
  対象のパラメータがモデルにあること・時間の値の範囲は、適用後の検証 (`validate`) が調べ、通らなければ拒否してモデルを変えません。Undo で前の設定に戻ります。
- パラメータを削除すると、エフェクトの対象からも取り除きます (`remove_parameter`)。
- 自動リギングの提案 (`ugoe_ai::rig`) は、作ったパラメータに向けた標準の設定をこのコマンドで足します ([ai-integration.md#テンプレート-ugoe_airigpropose_rig](ai-integration.md#テンプレート-ugoe_airigpropose_rig))。
- 設定を編集する画面は後続 (今は提案か、読み込んだモデルの設定で動く)。

## グルーの編集

グルー (`Model::glues`、[model.md](spec/model.md)・[evaluation.md#5-グルー](spec/evaluation.md#5-グルー)) を作る・置き換える・消すコマンド
(T0659。`ugoe_editor_core::glue_edit`)。どれも Undo で戻ります。

| `op` | 内容 |
| --- | --- |
| `create_glue { id?, mesh_a, mesh_b, distance }` | `mesh_a` と `mesh_b` をつなぐグルーを `glues` の末尾に加える。頂点の組は、基準の姿 (パラメータが既定値) のキャンバス座標 (親のデフォーマで写した後・グルーの前) で、B の頂点ごとに `distance` ピクセル以内でいちばん近い A の頂点を選んで作る (B の頂点の順)。重みは 0.5、強さは 1 (軸の無い 1 つのキーフォーム)。`id` を省略すると `Glue_<A>_<B>` (モジュールのプロジェクトのメッシュは接頭辞を除いた ID。重なれば `_2`…) |
| `set_glue { glue, distance? }` | `glue.id` のグルーを丸ごと置き換える (両端のメッシュ・組と重み・強さのキーフォーム)。`distance` があれば組を `create_glue` と同じ方法で作り直す (重みは `glue.pairs` の先頭の重み)。ID は変えられない |
| `remove_glue { id }` | グルーを消す |

- 同じメッシュどうし・`distance` が負か有限でない・組が 1 つもできない・重みが有限でないときは `EditError::InvalidGlue`、メッシュや
  グルーが無ければ `NotFound`、`create_glue` の `id` が重なれば `DuplicateId`。参照するメッシュと頂点の番号は適用後の検証が調べます。
- モジュールのプロジェクトでは、本体のグルーは変えられず (作業用のモデルの本体の部分の検査で拒否する)、モジュールのグルーは
  モジュールのメッシュだけをつなげます (本体のメッシュをつなぐとモジュールに書き出せない)。
- 描画用の複製のグルー (`DrawClone_…`) は `sync_draw_clone` が作り直すので、画面では編集しません ([描画用の複製](#描画用の複製))。

### グルーの画面

右の列の「Glue」タブ (`lib/panels/glues/`、T0659。組み立ては `glues.ts`)。

- **一覧:** グルーごとに ID、両端のメッシュと組の数。押すと選び、下に編集の欄を出します。「Select meshes」で両端をツリーで選びます。
- **編集:** 「Swap」で両端を入れ替え (組の a と b を入れ替え、重みを `1 − w` にするので見た目は変わらない)、「Weight」で重み
  (0 で A に、1 で B に寄せる。すべての組に同じ値。組ごとに違えば注記する)、強さはキーフォームごと (軸が無ければ「Intensity」、
  あればセル `Glue = 1` ごと)。「Pair again within N px」で組を作り直し、「Remove glue」で消します。どれも `set_glue`・`remove_glue`。
- **作る:** 「New glue」で Mesh A・Mesh B と距離 (既定 8 ピクセル) を選んで「+ Glue」(`create_glue`)。ツリーで 2 つのメッシュを選んで
  いれば、それを既定にします。作ったグルーを選びます。
- **モジュールのプロジェクト:** 本体のグルー (どちらかの端が本体のメッシュ。`ModuleStatus.base_meshes`) は「base」の印を付け、編集の欄を
  無効にします (`BaseLock`)。Mesh A・B の候補はモジュールのメッシュだけです。描画用の複製のグルーは「clone」の印を付け、同じく無効にして
  インスペクタの「Sync」を案内します。
- **強さの軸 (T0661):** 編集の欄で、パラメータとキー (既定はパラメータの既定値) を選んで「+ Axis」で強さの格子に軸を足し、軸ごとに
  「+ Key」でキーを足し、「×」で外します (`add_keyform_key`・`remove_keyform_key` の `target: { kind: "glue", id }`。軸の最後のキーを外すと
  軸ごと消える)。キーは範囲の中の、軸にまだ無い値だけ (`keyValue`)。キーごとの強さは上の欄 (`AngleX = 30` など) で変えます。
- **組をビューポートで編集する (T0661):** 「Glue」タブで選んだグルーを、ビューポートに重ねます (`viewport/tools/glue/GlueOverlay.svelte`。
  ToolOverlay が出す)。両端のメッシュの輪郭 (A は緑、B は紫の破線)・組の線・組んだ頂点を描きます。位置は評価後 (グルーを掛けた後) なので、
  強さ 1 の組の頂点は重なります。下のバーの「Add pairs」で両端の頂点をすべて出し、A と B の頂点を 1 つずつ (どちらからでも) 押すと
  組を足します (`withPair`。B の頂点は 1 つの A の頂点とだけ組むので、同じ B の組があれば置き換える。重みはグルーの重み)。「Remove pairs」で
  組の線の中点に四角を出し、押すとその組を外します (`withoutPair`)。どちらも `set_glue` (Undo の 1 単位) で、Esc でモードを抜けます。
  本体のグルー・描画用の複製のグルーは重ねるだけで、モードを選べません。モードと先に押した頂点は `GlueView.pairMode`・`pending`。

## ポーズの編集

- `set_pose` はポーズ (`Model::pose`、[model.md#ポーズ](spec/model.md#ポーズ)) を丸ごと置き換えます (T0519)。参照するパーツがあること・
  同じパーツが 2 回現れないこと・`fade_duration` の範囲は、適用後の検証 (`validate`) が調べ、通らなければ拒否してモデルを変えません。Undo で戻ります。
- モジュールのプロジェクトでは、本体のグループと要素は変えられません (作業用のモデルの本体の部分の検査で拒否する)。衣装のパーツだけの
  グループを後ろに足すことと、本体の要素の `links` に衣装のパーツを加える (スロットの `pose_parts` にある要素だけ。合成が検査する) ことが
  できます ([modules.md#ポーズ](spec/modules.md#ポーズ))。
- AI の提案 (モジュールの自動リギングの差分のポーズ、[ai-integration.md#モジュールの自動リギング](ai-integration.md#モジュールの自動リギング))
  とエージェントも使います。

### ポーズの画面

右の列の「Poses」タブ (`lib/panels/poses/`、T0521)。操作はすべて、今のポーズを組み替えて `set_pose` を送るだけです (組み立ては
`poses.ts`)。

- **グループ:** グループごとに要素を並べます。表示されるのはグループの要素のうち 1 つで、先頭の要素 (「first」の印) が最初に
  表示されます。要素をドラッグするか ▲▼ で並べ替え (ドラッグは同じグループの中だけ。落とした要素の前か後ろに入る。
  `poses.ts` の `moveEntryTo`。T0523。ドラッグはツリーと同じくポインタのイベントで作り、離したときに 1 回だけ送る。
  Esc・ポインタの取り消しでは何も変えない。T0696)、✕ で要素を外します (最後の要素を外すとグループも消える)。「Remove group」はグループを消します
  (パーツは残る)。要素の名前を押すとそのパーツを選びます。
- **リンク:** 要素ごとの「Links」で、要素と一緒に切り替わるパーツ (腕の差分に付く袖など) をチェックで選びます。
- **足す:** 「+ Entry」はグループに、「+ Group」は新しいグループに、ポーズでまだ使っていないパーツを足します (同じパーツはポーズの
  中で 1 度しか使えない。候補から除く)。「+ Group from selection」はツリーで選んだ 2 つ以上のパーツでグループを作ります。
- **フェードの時間:** 「Fade time (s)」で `fade_duration` (0 以上) を変えます。
- **ビューポートで確かめる (T0523):** 「Show the pose in the viewport」を入れると、編集中のビューポートにポーズを効かせ、グループごとに
  1 つの要素だけを見せます (アプリでの見え方。モデルは変えない)。要素ごとの「Show」で表示する要素を切り替えます (クロスフェードは待たずに
  切り替わった姿を出す)。`ViewportController.setPosePreview` が WebPuppet のポーズの段を有効にし、`show_pose` で要素を選んで
  `update` で進めます (編集中はほかの段を止めてあるので、ポーズだけが進む)。モデルを読み直しても続け、タブを離れると効かせるのをやめます。
  プレビューモードと物理のプレビューの間は、毎フレームの `update` に任せます。
- **モジュールのプロジェクト:** 本体のグループ (作業用のモデルのポーズの先頭の `ModuleStatus.base_pose_groups` 個) は点線の枠と
  「base」の印で示し、並べ替え・外す・足す・消すを無効にします。本体の要素のうちスロットが公開するもの (`SlotSummary.pose_parts`)
  にだけ「Links」を出し、モジュールのパーツを選べます (本体のリンクは「Base links」として見せ、送るときにそのまま残す)。
  公開していない要素には理由を出します。要素・リンクの候補はモジュールのパーツだけで、フェードの時間は本体のものなので変えられません。
  モジュールのグループは本体のグループの後ろに足します。検査は editor-core の本体の部分の検査と合成が行います。

## プロジェクト

- 保存形式は [`.ugoeproj/`](spec/file-formats.md#プロジェクト-ugoeproj) です。`ugoe_editor_core::Project` がプロジェクト 1 つ分の中身 (`ProjectMeta`・モデル・テクスチャ・物理・モーション・表情) を持ち、`save(dir)` / `load(dir)` で読み書きし、`export_bundle(&Project)` で `.ugoe` のバイト列を作ります。
- 編集中は、モデル・テクスチャ・物理・モーション・表情を `Document` が持ちます (`Project::document()` で作り、`Project::from_document` で保存用に戻す。モーションと表情は T0630、物理は T0629 で移した)。
- 物理の設定は Tauri の `get_physics` (`PhysicsSettings | null`) で読みます。ビューポートは `get_model_bundle` のバンドルに入った物理を使います。
- **未保存の変更:** `Document` は状態ごとに番号を振り、保存した状態の番号を覚えます。`is_modified()` は現在の状態が保存した状態と違うかを返すので、Undo で保存時の状態に戻ると「変更なし」に戻ります。失敗したコマンドは状態を変えません。

### Tauri コマンド

| コマンド | 内容 |
| --- | --- |
| `project_status` | 名前・保存先・未保存の変更の有無 (`ProjectStatus`) を返す |
| `undo` / `redo` | 1 つ戻す / やり直す。できなければ `false`。`src-tauri/src/commands.rs` |
| `history_status` / `set_undo_limit(limit)` | Undo / Redo できるかと履歴の上限 (`HistoryStatus`) を返す / 上限を変えて覚え、変えた後の `HistoryStatus` を返す ([Undo](#undo))。`src-tauri/src/history.rs` |
| `get_animations` | ドキュメントのモーションと表情 (`AnimationData { motions, expressions }`) を返す。タイムラインの表示用 (編集は `apply_command`)。`src-tauri/src/commands.rs` |
| `new_project` | 空のモデル (1024×1024 のキャンバス) の新しいプロジェクトにする |
| `new_project_from_psd(path, mesh_options?)` | 空のプロジェクトを作って PSD を取り込み (キャンバスは PSD の大きさ、警告は `import_psd` と同じ)、それを今のプロジェクトにする。名前はファイル名 (拡張子を除く)、保存先はなし。取り込みは Undo の 1 単位で、未保存の状態から始まる (Undo すると空のプロジェクトに戻る)。失敗したら今のプロジェクトはそのまま |
| `open_project(path)` | `.ugoeproj` ディレクトリを開く (モジュールのプロジェクトなら本体も読む)。失敗したら今のプロジェクトはそのまま |
| `save_project(overwrite?)` | 開いた (前回保存した) 場所に保存する。保存先がなければエラー。開いた後で外で書き換えられていたら、`overwrite` が真でない限り書かずにエラー ([外での書き換え](#外での書き換え)) |
| `reload_project` | 開いているプロジェクトを保存先から開き直す。未保存の変更は破棄する (確認はフロントエンドで行う) |
| `check_external_change` | 開いた (最後に保存した) 後で保存先のファイルが外で変わったか (`ExternalChange \| null`) |
| `save_project_as(path)` | 別の場所に保存し、以後の保存先にする。拡張子 `.ugoeproj` がなければ付け、名前をディレクトリ名にする。ファイルはロックの外で書き、失敗したら名前と保存先は元のまま ([外での書き換え](#外での書き換え)) |
| `get_export_settings` / `set_export_settings(settings)` | 書き出しの設定 (`ExportSettings`) を読む / プロジェクトに残す ([書き出し](#書き出し))。範囲の外はエラー |
| `get_preview_lighting` / `set_preview_lighting(lighting)` | プレビューの光源 (`PreviewLighting`) を読む / プロジェクトに残す ([ライティングの画面](#ライティングの画面))。範囲の外はエラー |
| `get_normal_map_group(texture)` / `get_normal_map_groups` | 1 枚のテクスチャのまとまりと既定の設定 / すべての対象のまとまりの一覧 (AI タブ。[法線マップの提案](#法線マップの提案))。`NormalMapGroupView` (`with_normal_map` はすでに法線マップを持つテクスチャ)。`src-tauri/src/lighting.rs` |
| `generate_texture_normal_map(texture, settings)` / `assign_texture_normal_map(texture, path, flip_green)` / `get_texture_png(name)` / `get_texture_versions(names)` | 法線マップの生成・PNG の割り当て (どちらも 1 回の Undo) と、テクスチャか法線マップの画像の PNG と版 (中身のハッシュ。無い画像は null)。`src-tauri/src/lighting.rs` |
| `write_png_file` (本文 = PNG のバイト列、ヘッダー `x-ugoe-path` = パス) | ビューポートのスクリーンショットを書く ([スクリーンショット](#スクリーンショット))。`src-tauri/src/screenshot.rs` |
| `export_bundle(path)` | プロジェクトの書き出しの設定で `.ugoe` ファイルに書き出し (アトラス化は任意)、`ExportReport` を返す。保存状態は変えない |
| `open_sample_project(parent)` | 着せ替えの見本 (`assets/samples/dress-up/` をバイナリに埋め込んだもの) を、`parent` の中の新しいフォルダー `ugoe-dress-up-sample` (あれば `-2`・`-3`…。既にあるフォルダーには書かない) に本体 `dress-up.ugoeproj` とモジュールのプロジェクト 4 つとして書き出し、本体を開く (T0715)。変換は `ugoe sample dress-up --project` と同じ `ugoe_editor_core::bundle_project::save_bundle_projects` で、本体の物理・モーション・表情も入る (T0655)。`src-tauri/src/sample.rs` |
| `import_psd(path, mesh_options?)` | PSD ファイルを今のドキュメントに取り込み ([PSD の取り込み](#psd-の取り込み)、Undo の 1 単位)、警告 (`PsdImportWarning[]`) を返す。最上位のパーツの名前はファイル名。`src-tauri/src/psd.rs` |

- パスの引数は **絶対パスだけ** を受け付けます (相対パスは `editor.relative_path` (`path must be absolute: …`) のエラー)。相対パスはプロセスの作業ディレクトリ
  (開発時は `apps/editor/src-tauri`) を基準にしてしまい、思わぬ場所に保存されるためです (T0619)。パスは [ファイルダイアログ](#ファイルダイアログ) で選びます。
- ファイルメニューは「New」「New from PSD…」「New Module…」「Open…」「Open Sample…」「Save」「Save As…」「Export…」「Import PSD…」です
  (モジュールのプロジェクトでは「Export…」が「Export Module…」。[モジュールのプロジェクト](#モジュールのプロジェクト))。
- 未保存の変更がある状態で「New」「New from PSD…」「Open…」「Open Sample…」を押すと、もう一度押すまで実行しません (確認のダイアログの代わり)。
- 新規・開く・取り込みの後は、ビューポートをキャンバス全体の表示 (Fit) に戻します。
- 別のモデルに替わったとき (「New」「New from PSD…」「Open…」) は、パラメータのスライダーを新しいモデルの既定値に戻します
  (`ViewportController.resetParametersOnNextModel`。WebPuppet は同じ ID の値を引き継ぐので、読み込んだ後に既定値へ戻す)。
  「Import PSD…」は今のモデルに足すだけなので、値を保ちます。
- **起動時は空のプロジェクト** (`new_project` と同じ。名前は `Untitled`、1024×1024 のキャンバス、保存先なし) です。
  空のモデルへの「Import PSD…」はキャンバスを PSD の大きさにするので、起動直後の取り込みは「New from PSD…」と同じ結果になります
  (名前だけが `Untitled` のまま)。
- 空のプロジェクト (パーツ・メッシュ・デフォーマ・テクスチャがない。取り込みの規則と同じ判定、`src/lib/emptyState.ts`) では、
  ビューポートの中央とツリーに「File > New from PSD… で始める」案内を出します。
- 空のプロジェクトも保存・書き出しできます (メッシュのない `.ugoe` になる)。

### 読むファイルの種類と大きさ

プロジェクト (`Project::load*`)・モジュールのプロジェクトの本体 (`load_base*`)・指紋 (`ProjectFingerprint`) は、プロジェクトの中のファイルを
**通常のファイルのときだけ、大きさの上限までで** 読みます (T1607。`crates/ugoe-editor-core/src/project/read.rs`)。
プロジェクトの中のファイルは外 (`ugoe-mcp`・手での編集・受け取ったプロジェクト) が置くので、`model.json` という名前の FIFO や
`/dev/zero` へのリンクがありえます。FIFO は開くだけで書き手を待って止まり、装置は終わらずにメモリを使い果たします。

- 通常のファイルでないもの (FIFO・装置・ディレクトリ。リンクは辿って先を見る) は、開かずに `project.not_regular_file` で拒みます。
  ディレクトリの中から集めるもの (`motions/`・`expressions/`・`textures/`) は、前から通常のファイルだけを数えます。
- 上限を超えるファイルは、読まずに `project.file_too_large` で拒みます。見てから読むまでの間に大きくなっても、上限 + 1 バイトまでしか読みません。
- 上限は **読む側の設定** (`ugoe_editor_core::ProjectReadLimits`) で、既定の値を持ちます。`Project::load_with_limits`・`load_base_with_limits`・
  `ProjectFingerprint::reread_with_limits`・`reread_base_with_limits` で変えられ、ほかの関数は既定の値で読みます。エディタ・CLI・MCP サーバは既定の値を使います。

| 対象 | 既定の上限 | 理由 |
| --- | --- | --- |
| JSON 1 つ (`project.json`・`model.json`・`module.json`・`physics.json`・モーション・表情) | 256 MiB (`DEFAULT_MAX_JSON_BYTES`) | 大きなメッシュを持つモデルでも数十 MiB に収まる。JSON は読んだ後に何倍ものメモリで値に直すので、画像より小さくする |
| 画像 1 つ (`textures/*.png`) | 512 MiB (`DEFAULT_MAX_IMAGE_BYTES`) | 一辺の上限 (16384 画素、[読み込みの上限](spec/file-formats.md#読み込みの上限)) の RGBA は展開して 1 GiB。圧縮した PNG はふつうその半分を大きく下回る |
| 本体の `.ugoe` | 512 MiB (`DEFAULT_MAX_BUNDLE_BYTES`) | ビューア・スタジオが開く `.ugoe` の上限 (512 MiB) と揃える |

- 保存 (`Project::save`) で今のファイルと中身を比べるのも、通常のファイルで書く中身と同じ大きさのときだけです (比べられなければ置き換える)。
- 読み方は 3 つのアプリと同じ `ugoe_fs::read_regular_file` です (T0011・T0025)。
- 見てから開くまでの間に FIFO へ差し替えられると、開くところで止まりえます (その窓は小さいので受け入れています)。

### 外での書き換え

`ugoe-mcp` (ファイルを直接扱う形)・別のエディタ・手での編集が、開いているプロジェクトを書き換えたことに気づき、黙って上書きしない
([ADR 0023](adr/0023-editor-mcp-bridge.md) の 5、T0641)。操作は [マニュアル](manual/screen.md#外でプロジェクトが書き換えられたとき)。

- **検出の方法: ファイルの指紋を比べる (ファイルの監視は使わない)。** `ugoe_editor_core::ProjectFingerprint` が、`Project::load` の読むファイル
  (`project.json`・`model.json`・`module.json`・`physics.json`・`motions/`・`expressions/`・`textures/*.png`) の中身のハッシュと大きさを持つ。
  OS のファイルの通知 (`notify` クレートなど) は、WSL 越しのパス (`\\wsl.localhost\...`、9P) やネットワークのドライブ (SMB) では届かないことが多く、
  エディタを Windows で動かし、エージェントを WSL で動かす使い方 (ADR 0023 の 4) で働かない。指紋を比べる方式はどのファイルシステムでも同じに働き、
  依存も増えない。代わりに、確かめた時点でしか気づかない。
- **いつ確かめるか:** ウィンドウに戻ったとき (`focus`)、ウィンドウが見えている間は 5 秒ごと (`CHECK_INTERVAL_MS`)、保存の前。
  バックエンドの `save_project` も書く直前に確かめ、変わっていれば `overwrite` なしでは書かない (画面の確認とのあいだの書き換えも拾う)。
- **操作を重ねない (T0689):** ファイルメニューの流れは `src/lib/fileMenu.svelte.ts` の `FileMenuFlow` が持つ (`FileMenu.svelte` は表示と呼び出しだけ。
  `fileMenu.test.ts` で IPC を偽物にして確かめる)。利用者の操作 (保存・上書き・開き直し・本体の読み直し・ダイアログで選ぶ操作・モジュールの設定) は
  `busy` を立てて 1 つずつ行い、途中に来たもの (「Save」の連打など) は捨てる。確かめは操作の途中には行わず、ウィンドウに戻ったときと間隔の確かめが
  重なれば同じ確かめを待つ (同じ変化に 2 回応じない)。確かめの途中に来た利用者の操作は、確かめ (自動の開き直しまで) が終わってから行う。
- **比べ方:** 開いた (保存した) ときの指紋を `ProjectSession` が覚え、`ProjectFingerprint::reread` で取り直して比べる。中身が同じなら更新日時が変わっても
  変更としない (`touch`・同じ内容の書き直し)。JSON は毎回読んでハッシュし、PNG は大きさと更新日時が前と同じなら前のハッシュを使い回す
  (大きな画像を 5 秒ごとに読まないため。大きさも日時も同じまま PNG の中身だけを変えられると気づかない)。ハッシュは `std` の `DefaultHasher` で、ファイルには残さない。
- **自分の保存を外の変更と取り違えない:** 保存の直後に指紋を取り直す (`PendingSave::write`)。確かめる処理 (`check_external_change`)・開き直し・保存は
  保存の順番の印 (`AppState::saving`) を取ってから行うので、書きかけのファイルを外の変更として見ない。
- **保存はロックの外で書く (T0025):** `save_project` とブリッジの `save_project` は、ドキュメントのロックの中では写しを取るだけで
  (`ProjectSession::begin_save`・`Document::begin_save`)、ファイル (と `.ugoe`) はロックの外で書き、書けたらロックを取り直して書いた状態に保存の印を付ける
  (`Document::mark_saved_state`)。書いている間も画面とほかのツールは待たない。書いている間の編集・設定の変更は未保存のまま残り、Undo で保存した状態に
  戻れば変更なしになる。書いている間にプロジェクトを替えたら (開く・新規)、新しいプロジェクトには印を付けない。
- **「名前を付けて保存」もロックの外で書く (T1620):** `save_project_as` は、名前 (ディレクトリ名) と着せ替えの一覧 (新しい保存先からの相対パス) を
  付け替えたメタ情報を写しの側だけで作り (`ProjectSession::begin_save_as`)、ロックの外で書く。書けたらロックを取り直し、名前と一覧をセッションでも
  付け替えてから保存先・指紋・保存の印を戻す (`WrittenSave::finish`)。書いている間の編集と設定の変更は残り (一覧の変更も新しい保存先から指し直す)、
  保存の印は書いた版にだけ付く。書けなければセッションは変えないので、名前・保存先・保存済みの印は元のまま。
  開くときの指紋は読み込みの **前** に取る (`Project::load_tracked`)。読み込みの途中で書き換えられたら、次に確かめたときに変更として見える (見逃すより開き直しを促す)。
- **応じ方 (`src/lib/externalChange.ts`、Vitest):** 変わっていれば `ExternalChangeDialog` を出す。未保存の変更がなければ「Reload」・「Later」、
  未保存の変更がある (または保存しようとした) ときは「Reload and discard my changes」・「Overwrite with my version」・「Save As…」・「Later」、
  `project.json` がなければ (移された・消された)「Save here again」・「Save As…」・「Later」。「Later」を選んだ外の状態 (`ExternalChange.token`) は、
  間隔の確認では二度尋ねない (保存の前は尋ねる)。「Reload automatically when there are no unsaved changes」([覚える値](#覚える値)、既定は切) を入れると、
  未保存の変更がないときは尋ねずに開き直す。
- **開き直し (`reload_project`):** 保存先から `Project::load_tracked` で読み直す。AI への送信の「今後は確かめない」は同じプロジェクトなので残し、
  AI の提案は前のドキュメントに対するものなので消す。ビューポートの表示の位置とパラメータの値は保つ (新規・開くと違い Fit しない)。

### ファイルダイアログ

ファイルメニューのパスは OS のファイルダイアログ (`tauri-plugin-dialog`、MIT OR Apache-2.0) で選びます。ダイアログはバックエンドが開き
(`open_file_dialog`・`save_file_dialog`、`src-tauri/src/dialog.rs`)、選ばれたパスを覚えて、コマンドが読み書きできるパスをそれに絞ります
(下の「読み書きできるパス」。T1616)。
パスの入力欄はありません (ダイアログが使えない環境の予備も置かない。ヘッドレスの確認では下の差し替えを使う)。
取り込みの警告はメニューの横のボタン (押すと一覧の Popover) で、保存・書き出しの結果 (書いた場所の絶対パス) は Toast で、
失敗はヘッダーの下の知らせの帯 (Notice。閉じるまで残る) で出します。保存していない変更を捨てる操作 (New・New from PSD…・New Module…・
Open…・Open Sample…・本体の問題の枠の「Open base project」) は、変更があれば確かめのダイアログ (AlertDialog) を出し、
「Discard and continue」を選んだときだけ続けます (T0663。前は同じボタンを 2 度押す形だった)。

| 操作 | ダイアログ | フィルタ | 既定 |
| --- | --- | --- | --- |
| New from PSD… / Import PSD… | ファイルを開く | `.psd` | 最初の場所 |
| Open… | **フォルダーを選ぶ** (`.ugoeproj` はディレクトリ) | なし | 最初の場所 |
| Open Sample… | フォルダーを選ぶ (見本のフォルダーをこの中に作る)。開いた後の最初の場所は見本のフォルダー (続けて「Open…」でモジュールのプロジェクトを開ける) | なし | 最初の場所 |
| Save As… (保存先のない Save も) | 保存 (新しいフォルダーを作れる) | `.ugoeproj` | 最初の場所に `<プロジェクト名>.ugoeproj` |
| Export… の欄の「Export .ugoe…」 | 保存 | `.ugoe` | 最初の場所に `<プロジェクト名>.ugoe` |

- **最初の場所:** 最後にダイアログで選んで成功した場所 → 開いているプロジェクトの隣 → ドキュメント (`documentDir()`、なければホーム) の順。
  プロジェクト名のうちファイル名に使えない文字は `_` にする。
- 保存のダイアログで選んだ名前に拡張子 (`.ugoeproj` / `.ugoe` / `.ugoemod`) がなければ付け足す (`save_project_as` も `.ugoeproj` を付ける)。
  Linux のダイアログはフィルタの拡張子を付けずにパスを返すことがあるため。付けるのはバックエンドの `save_file_dialog` (最初のフィルタの
  最初の拡張子。大文字小文字は区別しない) と、下の差し替えの経路 (`fileDialogs()` が同じ規則で付ける) だけで、画面はダイアログの答えを
  そのまま使う (T1625)。ファイルメニューの外の保存のダイアログ (スクリーンショットの `.png`・トラッキングの記録の `.json`・
  Claude Desktop の拡張の `.mcpb`) も同じ。
- ダイアログを閉じた (取り消した) ときは何もしない。ダイアログを開いている間はメニューのボタンを押せない。
- 権限 (`capabilities/default.json`) に `dialog:` のものは無い (webview はダイアログを開けない。`acl.rs` のテストが確かめる)。
  ダイアログはアプリのコマンド `open_file_dialog`・`save_file_dialog` で開く。既定の場所を得る `@tauri-apps/api/path` は `core:default` に含まれる。
- コード: ダイアログの呼び出しは `src/lib/fileDialogs.ts` (`FileDialogs`。`ipc.openFileDialog`・`ipc.saveFileDialog`)、フィルタ・拡張子・
  パスの整形は `src/lib/filePaths.ts` (Vitest)。
- **差し替え (テスト用):** `window.__UGOE_FILE_DIALOGS__` に `FileDialogs` (`openFile`・`openDirectory`・`saveFile`・`defaultDirectory`) を置くと、
  ダイアログの代わりにそれを呼ぶ。ヘッドレスのブラウザでは、IPC の差し替え (`window.__TAURI_INTERNALS__`) と一緒に固定のパスを返すものを置く。
  差し替えの `saveFile` が拡張子の無い名前を返したら、`fileDialogs()` がバックエンドと同じ規則で拡張子を付ける (`fileDialogs.test.ts`)。
  本物のアプリではこの差し替えのパスをバックエンドが知らないので、コマンドは `file.not_chosen` で断る。e2e (tests/e2e) はデバッグビルドの
  バックエンドの差し替えを使う: 環境変数 `UGOE_EDITOR_DIALOG_ANSWERS` にディレクトリを渡すと、ダイアログは OS のものを開かず、そこの
  `open-file`・`open-directory`・`save-file` のファイルに書いたパスを選んだことにする (読んだら消す。無ければ取り消し)。環境変数はアプリを
  起動する側が決めるので、webview はこれで選ばせるパスを決められない。リリースビルドには入らない。

### 読み書きできるパス

webview で思わぬスクリプトが動いても、利用者が選んでいないファイルを読み書きできないよう、ファイルのパスを受けるコマンドは
利用者が選んだパスだけを受け付けます (T1616。ビューアの `FileAccess` (T0727)・スタジオの `access.rs` (T0838) と同じ考え。`src-tauri/src/access.rs`)。
ダイアログで選んだパスの覚え方と拡張子の補い方は 3 つのアプリで共通の `ugoe_fs::FileAccess`・`ugoe_fs::with_extension` です (T0037)。

- **読める:** 開くダイアログで選んだパス (ファイルとフォルダー)・ウィンドウにドロップしたパス・開いているプロジェクトのフォルダーの中
  (`..` を含むパスは除く)・開いているプロジェクトが指すファイル (モジュールの本体・プレビューの着せ替えの一覧・提案の元の PSD)・
  `open_project` で閉じたモジュールのプロジェクトのフォルダー ([モジュールに戻る](#モジュールに戻る) ため。T1622)。
  どれもこのセッションの間だけ覚える (エディタには最近のファイルの一覧が無いので、ファイルには残さない)。
- **書ける:** 保存のダイアログで選んだパス (拡張子を付けた後のもの。名前が最初のフィルタの最初の拡張子 (大文字小文字は区別しない) で終わっていなければ付ける。画面の `withExtension` と同じ規則。画面はダイアログが返したパスをそのまま渡す)。1 回書いたら忘れる (同じパスにもう 1 回書くには、またダイアログで選ぶ)。
  今の保存先への保存 (`save_project`・`reload_project`) はバックエンドが保存先を持つので、選び直さずに書ける。
- 選んでいないパスは、ファイルに触る前に `file.not_chosen` (読む) か `file.not_chosen_for_save` (書く) で断る (ビューア・スタジオと同じコード)。

| コマンド | パス | 受け付けるもの |
| --- | --- | --- |
| `open_project` | `path` | 読める (開くダイアログのフォルダー・本体の問題の枠の「Open base project」の本体・「Return to module」のモジュール)。閉じるのが保存したモジュールのプロジェクトなら、そのフォルダーを読めるパスに加える |
| `new_project_from_psd` / `import_psd` | `path` | 読める (開くダイアログ) |
| `open_sample_project` | `parent` | 読める (開くダイアログのフォルダー)。**例外:** 選んだフォルダーの中に見本のフォルダーを新しく作って書く (既にあるものには書かない) |
| `read_module_base` / `new_module_project` | `path` / `base` | 読める (開くダイアログ) |
| `open_module_with_base` / `module_open_problem` / `reload_base` | `path`・`base` | 読める。`module_open_problem` が返した本体 (選んだモジュールのプロジェクトが指すもの) は読めるパスに加える |
| `classify_psd_layers` | `psd` | 読める (開くダイアログ・取り込んだ PSD) |
| `assign_texture_normal_map` | `path` | 読める (開くダイアログ) |
| `set_preview_modules` / `read_module_file` | モジュールのパス | 読める (開くダイアログ・ドロップ・一覧に前からあるもの) |
| `save_project_as` | `path` | 書ける (保存のダイアログ)。`.ugoeproj` が無ければ付けたフォルダーに書く |
| `export_bundle` / `export_module` | `path` | 書ける (保存のダイアログ) |
| `write_png_file` / `write_json_file` / `save_claude_desktop_bundle` | `path` | 書ける (保存のダイアログ) |

パスを受けないコマンド (`save_project`・`reload_project`・`get_model_bundle` など) は、バックエンドが持つ保存先だけを読み書きする。
つないだエージェント (MCP のブリッジ) はファイルのパスを受けない (開いているドキュメントだけを扱う)。

### 書き出し

「Export…」は書き出しの欄を開きます。欄で設定を選び、「Export .ugoe…」で保存のダイアログを開いて書き出します (`src/lib/ExportPanel.svelte`)。
モジュールのプロジェクトでは、同じ欄を「Export Module…」で開き、「Export .ugoemod…」で書き出します (下の最後の項目)。

- **設定 (`ugoe_editor_core::ExportSettings`):** テクスチャのアトラス化の有無 (既定はしない)、アトラス 1 枚の最大の大きさ
  (512〜16384px から選ぶ。既定 2048)、余白 (端の画素の複製の幅、0〜64px。既定 2)。規則は
  [file-formats.md#テクスチャのアトラス化](spec/file-formats.md#テクスチャのアトラス化)。
- 設定は変えるたびにプロジェクトに残し (`set_export_settings`)、保存で `project.json` の `editor.export` に書きます
  ([file-formats.md#projectjson](spec/file-formats.md#projectjson))。すべて既定値なら書きません。設定を変えると未保存の印が付きます
  (`ProjectSession.meta_modified`。Undo の対象ではない。モデルを変えないので `Command` にしない)。
- 欄を続けて変えても重ねません (`src/lib/exportSettings.svelte.ts`、T0689): 変えた値はすぐ欄に出し、`set_export_settings` は 1 つずつ順に、
  前の変更まで残した設定に重ねて送ります (前の変更が終わる前の古い設定を元に送って戻さない)。断られた変更だけを欄から戻し、理由を出します。
- **結果:** 書き出した後、欄の下にテクスチャの枚数・描画の状態の切り替え回数・描画の回数の前 → 後と、アトラス化の詳細
  (詰めた枚数とアトラスの大きさ、同じ画像として 1 つにしたもの、まとめずに残したものと理由、使われないので取り除いたもの) を出します
  (`src/lib/exportSummary.ts`、Vitest)。別のプロジェクトに替わると欄を閉じて結果を消します。
- **ファイルの書き方:** 書き出し (`.ugoe`・`.ugoemod`)・画像 (`write_png_file`)・トラッキングの記録 (`write_json_file`)・設定
  (`ai-provider.json`・`mcp-bridge.json`) は、行き先と同じフォルダーの一時ファイル `.<名前>.<プロセス ID>.<連番>.tmp` に書いて
  `sync` し、閉じてから名前を変えて置き換えます (`ugoe_fs::write_atomic`、T0687・T0025。ビューア・スタジオ・MCP サーバ・`ugoe-editor-core` と同じ関数)。途中で失敗しても
  (ディスクがいっぱい・強制終了) 前のファイルは残り、一時ファイルは消します。`.ugoeproj` の保存も各ファイルを一時ファイル経由で書きます
  (`ugoe_editor_core::Project::save`)。
- **IPC で受けたパスの読み方:** 画面から受けたパスで読むファイル (PSD (`import_psd`・`new_project_from_psd`・AI の提案)・`.ugoemod`
  (`read_module_file`)・法線マップの PNG (`assign_texture_normal_map`)) は、通常のファイル (リンクは辿る) のときだけ、大きさの上限までで
  読みます (`ugoe_fs::read_regular_file`、T0011・T0025。PSD は `ugoe_psd::MAX_PSD_FILE_BYTES` の 2 GiB (PSD の形式の上限)、`.ugoemod` は 512 MiB、PNG は 64 MiB)。
  装置 (`/dev/zero`)・FIFO・ディレクトリは開かずに `editor.io` (`not a regular file`) にします (読み続けてメモリを使い果たす・開くだけで
  止まるため)。線引きは MCP サーバ・ugoe-headless・CLI の `import-psd` と同じです。CLI のモデル・バンドルの入力だけは、利用者が自分でパスを渡すので
  FIFO を読めるままにします (T0530)。
  `.ugoeproj` と本体の `.ugoe` は `ugoe_editor_core::Project` が読むので、ここには含みません。
- **読めなかった設定:** アプリの設定のファイル (`editor-settings.json`・`ai-provider.json`・`mcp-bridge.json`) が壊れている・読めない
  ときは、既定のまま起動し、起動しただけではファイルを書き換えません (ブリッジも始めない)。利用者が設定を変えたときに初めて書きます
  (スタジオの API の設定と同じ。T0011 で確かめた)。
- **切り替え回数の見積もり (`ugoe_editor_core::export::estimate_draws`):** エディタのバックエンドは `ugoe-render` (GPU) に依存しないので、
  参照レンダラの本描画のまとめ方 ([rendering.md#描画の最適化](spec/rendering.md#描画の最適化)) を CPU で真似ます。既定のパラメータで
  描画順に並べた、表示されていて三角形を持つメッシュについて、(テクスチャ・合成モード・カリング) が前と変わる回数を数えます。
  マスクアトラスの違いは数えないので、マスクを多く使うモデルでは `ugoe render --stats` の `state_changes` より少なく出ることがあります
  (`character.psd` では前後とも `ugoe render --stats` と一致した: 12 枚 → 1 枚、切り替え 10 → 2、描画 11 → 3)。
- 着せ替えのモジュール (T0111) のテクスチャはアトラスに含めません ([file-formats.md#テクスチャのアトラス化](spec/file-formats.md#テクスチャのアトラス化))。
- **モジュールの書き出し (T0654):** モジュールのプロジェクトでも同じ設定 (`editor.export`) を使い、アトラス化を選んでいれば、モジュールのテクスチャだけを
  モジュールの単位でまとめて書き出します (`ugoe_editor_core::export_module_with_settings`、`build_module_atlas`。`ugoe build <m>.ugoemod --atlas`
  と同じ規則。T0132)。書き出しの検査はアトラス化の前に行います。結果として、ファイルの大きさとアトラス化の詳細を出します (描画の見積もりは、
  本体に付けないと描画順が決まらないので出さない。`moduleExportSummary`)。

### ヘルプ

ヘッダーの「Help」は、操作マニュアル (`docs/manual/README.md` の GitHub の URL) を OS の既定のブラウザで開きます
(`tauri-plugin-opener` / `@tauri-apps/plugin-opener`、MIT OR Apache-2.0。Tauri v2 の WebView は `target="_blank"` のリンクを開かないため)。

- 権限は `opener:allow-open-url` を **`https://github.com/rosylilly/ugoe/*` と `https://www.anthropic.com/legal/*`** (AI の確認の画面の規約) だけに限っています (`capabilities/default.json`)。ほかの URL・ファイルは開けません。
- 開けなかったときは、理由と URL を選んで写せる形で、ボタンの下に知らせ (Notice) を出します (`src/lib/HelpButton.svelte`)。
- マニュアルは日本語だけです (ADR 0029 の 6)。英訳ができたら、今の言語のマニュアル (`docs/manual/en/`) があればそれを開きます。
- **差し替え (テスト用):** `window.__UGOE_OPEN_URL__` に `(url) => Promise<void>` を置くと、プラグインの代わりにそれを呼びます (`src/lib/help.ts`)。

### 言語

画面の言語は、ヘッダーの「Settings」の「General」で「OS に合わせる」(既定)・言語自身の名前で書いた言語 (「English」・「日本語」) から選びます
([ADR 0029](adr/0029-shared-ui-and-i18n.md) の 3、T0663)。選ぶとすぐに画面全体が切り替わり、`localStorage` (`ugoe.editor.locale`。[覚える値](#覚える値)) に覚えます
(`@ugoe/i18n` の `initLocale`・`setLocalePreference`。キーと OS の言語の取り方は `src/lib/locale.ts`、画面は `src/lib/LanguageSetting.svelte`。
選択肢が 4 つまでの間は SegmentedControl)。

- **OS の言語:** 起動時に `tauri-plugin-os` (`@tauri-apps/plugin-os`、MIT OR Apache-2.0) の `locale()` で OS の言語を取り、
  `@ugoe/i18n` の `initLocale()` に渡します (`setSystemLanguages()` を呼ぶ)。WebView の `navigator.languages` は OS の言語と違うことがあるため
  ([packages/i18n/README.md の「OS の言語」](../packages/i18n/README.md#os-の言語))。権限は `os:allow-locale` だけ (`capabilities/default.json`)。
  Tauri の外 (ヘッドレスのブラウザでの確認) では `navigator.languages` のまま。

「General」にはテーマ (「OS に合わせる」(既定)・ライト・ダーク) も置きます (`@ugoe/ui` の `ThemeSetting`、T0909)。`localStorage`
(`ugoe.editor.theme`) に覚え、ビューポートの背景もテーマに合わせて読み直します ([docs/ui.md](ui.md#名前と切り替え))。
- **文言:** 画面の文言はカタログ `apps/editor/locales/<言語>.json` (名前空間 `editor`。原文は英語) から `src/lib/i18n.ts` の `t()` で引きます
  ([docs/ui.md の「文言の書き方」](ui.md#文言の書き方))。バックエンドのエラーの訳は [エラーと警告のコード](#エラーと警告のコード)。
- MCP・CLI・ログの文言は英語のまま変えません。マニュアルは日本語だけです。
- 移行の途中: 外枠 (ヘッダー・ファイルの操作・ダイアログ・知らせ・タブ・ヘルプ) は T0663 で移しました。パネルの中 (`src/lib/panels/`・
  `viewport/` の道具・`ai/`・`agents/`・`ExportPanel`・`preview/`) は T0664・T0665 で移すまで英語のままです。

### AI の設定と送信の同意

AI のプロバイダを選び、外部の AI サービスを安全に使うための土台です (T0638、プロバイダの選択は T0512・[ADR 0022](adr/0022-multiple-ai-providers-and-agents.md))。
方針は [AI 連携のセキュリティとプライバシー](ai-integration.md#セキュリティとプライバシー)。バックエンドは `src-tauri/src/ai.rs`・
`ai_provider.rs`、画面は `src/lib/ai/` にあります。

- **プロバイダの選択:** Claude API (`ugoe_ai::claude::ClaudeProvider`) か OpenAI 互換の API (`ugoe_ai::openai::OpenAiProvider`。
  [OpenAI 互換の API](ai-integration.md#openai-互換の-api)) を選びます。既定は Claude (`claude-opus-5-5`) で、既存の利用者の設定は変わりません。
  - 選んだもの (`ProviderSettings`: プロバイダ・Claude のモデル・OpenAI 互換のベースの URL とモデルと JSON の出力の形) は
    エディタ全体の設定で、アプリの設定のディレクトリ (Tauri の `app_config_dir`) の `ai-provider.json` に置きます。プロジェクトには
    書かず、API キーも含めません。ファイルがなければ既定、読めなければ既定のまま起動します (知らない項目は無視し、欠けた項目は既定)。
  - OpenAI 互換は、よく使う送り先 (OpenAI・OpenRouter・Google Gemini・Ollama・LM Studio) を選ぶとベースの URL とモデルを埋めます
    (`src/lib/ai/providers.ts`)。ほかの送り先は URL を入れます。JSON の出力の形は、構造化出力 (既定)・JSON モード・プロンプトだけから
    選びます。後の 2 つは答えを検証して問い直すので、同じ内容を送り直すことがあります (確認の画面の「送り直す回数」に含める)。
  - 変えると `set_ai_provider` で検査 (ベースの URL は `http(s)://`、モデルは空でない) してから保存します。送り先が変わると
    「今後は確かめない」は効かなくなります (覚えた送り先と違うため)。

- **API キーの保存先:** OS の資格情報の保存先 (Linux は Secret Service、macOS はキーチェーン、Windows は資格情報マネージャー。
  [`keyring-core`](https://crates.io/crates/keyring-core) と OS ごとの保存先のクレート) に、サービス `ugoe-editor` で置きます。
  アカウントはプロバイダごと (`KeySlot`) で、Claude は前からの `anthropic-api-key`、OpenAI 互換は送り先のホストごとの
  `openai-compatible:<host>` です (OpenAI のキーを OpenRouter などの別の送り先に送らないため)。プロジェクト・設定ファイル・ログには書きません。
  - 読む順は保存先 → 環境変数 (Claude は `ANTHROPIC_API_KEY`、OpenAI 互換は送り先が `api.openai.com` のときだけ `OPENAI_API_KEY`)。
    読んだ値は `ugoe_ai::ApiKey::new` に渡します。
  - Claude はキーがないと送れません。OpenAI 互換はキーを省けます (手元のサーバ。キーがなければ `Authorization` を送りません)。
    キーがあっても、平文の `http://` で外部の送り先には送りません (`https://` にするよう断ります。ループバックは除く)。
  - 保存先は最初に使うときに開きます。開けない・読めない環境 (WSL・キーリングのない Linux など) では「使えない」と出し、
    環境変数だけを使います。普通のファイルへの代わりの保存はしません。
  - Linux の保存先は同期の D-Bus (`dbus-secret-service-keyring-store`、暗号は `crypto-rust`) で、OpenSSL は要りません。
    D-Bus の応答待ちで画面が止まらないよう、AI の IPC コマンドは別のスレッドで動かします。
- **画面に出すもの:** フロントエンドが知るのは、選んだプロバイダのキーの状態 (`KeyStatus`: `stored`・`not_stored`・`unavailable`)、
  キーが要るか、読む環境変数の名前とそこにキーがあるかだけです。キーは入力欄から `store_api_key` に 1 度渡すだけで、保存したら入力欄を空にします (Svelte の状態にも持たない)。
- **プロジェクトごとの「外部に送らない」:** `project.json` の `editor.ai.send_externally` (`ProjectMeta::ai_send_externally`)。
  選んだプロジェクトでは、外部に送るコマンドはすべて断ります (ローカルで完結するプロバイダと、手元のサーバ (ループバック) の
  OpenAI 互換の API は使える)。変えると未保存の変更になります。
- **送る前の確認:** AI に送る IPC コマンドは、プロバイダに関わらず `ai::prepare_requests` を通します。プロジェクトの設定 → API キー →
  同意の順に確かめ、
  同意がなければ **送らずに** `AiReply::NeedsConsent { disclosure }` を返します。フロントエンドは `withConsent(call, consentPrompt.ask)`
  (`src/lib/ai/consent.ts`) で確認のダイアログ (`ConsentDialog.svelte`) を出し、同意 (`ConsentAnswer`) を付けて呼び直します。
  - 送り先がループバック (`localhost`・`127.0.0.1`・`::1`) なら外部に出ない (`Destination::Local`) ので、確かめずに送ります。
  - ダイアログは `Provider::disclosure` の送り先 (サービス・URL・モデル)・画像の枚数とバイト数・テキストのバイト数と、
    送り先の規約へのリンクを示します (`termsFor`: Anthropic・OpenAI・OpenRouter・Google。知らない送り先には規約を確かめるよう
    促す文だけ)。Esc と「Cancel」は送りません。
  - 送り直すことがある回数 (`DisclosureView::retries`) には、OpenAI 互換で構造化出力を使わないときのプロバイダの問い直しも含めます
    (呼び出し側の回数 r、プロバイダの回数 j なら `(r + 1)(j + 1) − 1`)。
  - 同意は示した内容そのものに対するもので、呼び直したときに送る内容が違えば (画像が増えたなど) もう一度確かめます。
  - **「今後は確かめない」** は、このプロジェクトを開いている間 (エディタを終えるか、新規・開くで替えるまで) だけ、同じ送り先
    (サービス・URL・モデル) には確かめずに送ります。ファイルには残しません。画像を外部に送ることを永続的に黙って許す設定は
    置かない、という判断です。設定の画面の「Ask again」で取り消せ、「外部に送らない」にしても消えます。
- **キーの扱いの見せ方 (T0676):** 設定の API キーの欄には見えるラベルを付け、欄の上に、キーは OS の資格情報の保存先にだけ置き、
  送るのは選んだサービスへの問い合わせだけで、画面に二度と出さず、プロジェクト・設定のファイル・ログに書かないことを文で出します
  (環境変数のキーを使っているときは、起動したときに環境変数から読むこと)。欄はこの文と `aria-describedby` で結びます。キーの削除は
  取り消せないので `AlertDialog` で確かめます。送る前の確認には、送るのは表に挙げたものだけで、API キーはそのサービスへの認証にだけ
  一緒に送り、プロジェクトのファイルは手元に残ることを書きます。設定の失敗・接続の確認の失敗は閉じられる `Notice` (danger) です
  (送らないことを選んだ「Not sent.」は失敗ではないので文のまま)。
- **設定の画面:** ヘッダーの「Settings」(`SettingsDialog.svelte`)。プロバイダ・モデル・ベースの URL・JSON の出力の形の選択
  (「Apply」で保存)、選んだプロバイダのキーの保存・置き換え・削除 (プロバイダを変えたら先に Apply する)、「外部に送らない」、
  「Ask again」、「Test connection」(画像なしの小さなリクエストを、外部なら同意を得てから送ってプロバイダとキーを確かめる) があります。
  「1 回の提案の時間の上限 (分)」(`ProviderSettings::time_limit_minutes`。既定 15 分、1〜120 分。プロバイダに関わらない) も
  プロバイダの設定と一緒に Apply します。
- **取り消しと時間の上限 (T0541):** 送る IPC コマンドは `prepare_requests` が取り消しの印 (`ugoe_ai::CancelToken`。時間の上限付き) を
  作ってプロバイダに渡し、実行中の送信の一覧 (`AppState::ai_jobs`) に載せます。`Prepared::Ready` の `AiJob` で送り、結果は
  `AiJob::finish` を通します (取り消した後に届いた結果は捨てて `ai.cancelled`)。`cancel_ai_requests` が実行中の送信をすべて取り消し、
  送信は応答を待たずに終わります。上限を超えたら `ai.time_limit`。画面は AI の提案のパネルの進み具合の隣と「Test connection」の
  隣に「Cancel」を出し、取り消したらエラーではなく「Cancelled」と出します。詳しくは `docs/ai-integration.md#取り消しと時間の上限`。
- **AI の機能を足すとき (T0637 など):** 送る IPC コマンドは `consent: ConsentAnswer | null` を受け取り、`prepare_requests` の
  `Prepared::NeedsConsent` をそのまま `AiReply::NeedsConsent` で返します。画面は `aiSettings.canSend` (`src/lib/ai/ai.svelte.ts`) で
  ボタンの有効・無効を決めます (理由の文言は `aiBlockedReason`、プロバイダの名前は `providerLabel`。`src/lib/ai/providers.ts`)。

| コマンド | 内容 |
| --- | --- |
| `ai_settings` | `AiSettings { provider, service, local, key, key_required, key_env, env_key, send_externally, consent_remembered }`。キーの中身は含まない |
| `set_ai_provider(provider)` | プロバイダ・モデル・ベースの URL を検査して `ai-provider.json` に保存し、`AiSettings` を返す |
| `store_api_key(key)` / `delete_api_key` | 選んでいるプロバイダのキーを保存先に保存・削除し、`AiSettings` を返す |
| `set_ai_send_externally(allowed)` | 開いているプロジェクトの「外部に送ってよいか」を変える |
| `forget_ai_consent` | 「今後は確かめない」を取り消す |
| `test_ai_connection(consent?)` | 接続の確認。`AiReply<null>` (`done` か `needs_consent`) |
| `cancel_ai_requests` | 実行中の AI の送信 (分類・リグの提案・作り直し・接続の確認) をすべて取り消し、取り消した数を返す |

### AI の提案

AI などが作った提案 (`Command` の列、[ADR 0006](adr/0006-ai-proposals.md)) を、差分として確かめてから採用・却下する画面と、
PSD の取り込みから「レイヤーの分類 → リギングの提案」までを画面から行う流れです (T0637)。提案の置き場・プレビュー・採用の規則は
`ugoe_editor_core::proposal` (`Proposals`)、IPC は `src-tauri/src/proposals.rs`、画面は右の列の「AI」タブ
(`src/lib/ai/ProposalsPanel.svelte`・`proposals.svelte.ts`・`proposals.ts`) にあります。操作の手順は [マニュアル](manual/ai.md)。

- **流れ:** 「New from PSD…」「Import PSD…」で取り込んだ PSD を覚え (`ProposalSession::set_psd`。「Choose PSD…」で替えられる)、
  「Classify layers」で `ugoe_ai::classify` の分類をし、「Propose rig」で分類から `ugoe_ai::rig` のリグ (顔の角度・目・口) を
  提案します。分類していなければ「Propose rig」が先に分類します (2 段階で、段階ごとに何をしているかをパネルに出す)。
  - 「Use <プロバイダ>」(Claude・OpenAI・Ollama など、選んだプロバイダの名前) なら、分類は `classify_layers` (合成画像と
    レイヤーのサムネイル)、リグは `propose_rig_with_ai` (原画) で選んだプロバイダに送ります。使わなければ名前の規則 (`classify_by_name`) とテンプレートの既定値 (`propose_rig` と
    `RigParams::default`) で、何も送りません。キーがなくても使えます。
  - リグはドキュメント (モデルと物理の設定) に組みます。着せ替えの本体 (スロットが本体の物理の衝突体を公開する) に取り込んだ PSD
    でも、髪揺れのリグを本体の物理に足し、本体の衝突体とスロットの参照を残します (T1627。物理の設定が無いのにスロットが衝突体を
    公開しているときは髪揺れを組まず、理由に書く。[ai-integration.md](ai-integration.md#テンプレート-ugoe_airigpropose_rig))。
  - 深度推定 (T0505) の実際の推定器はまだないので、顔の振り向きは深度マップなし (顔の外接矩形に内接する楕円体。
    `TurnDepth::Rules`) で組みます。推定器ができたら、同じ流れに「深度を推定する」段階を足します。
  - 分類の答えとその PSD は、保存先のあるプロジェクトではキャッシュ (`.ugoe-cache/classification.json`) に残し、開き直したときに
    PSD が同じなら戻します (分類し直さず、AI も呼び直さない。[分類の答えのキャッシュ](#分類の答えのキャッシュ))。利用者が直したラベル・左右と
    「これでよい」と認めた答えは、ドキュメントの状態として持ち、`project.json` に残します ([分類の直し](#分類の直し))。確からしさの低いレイヤーと、
    AI の分類が名前の規則の確からしい答えと食い違うレイヤー (`needs_review`。食い違いは `LayerLabelView.name_disagreement` に
    名前の規則のラベルか左右。T0546、[ai-integration.md](ai-integration.md#レイヤーの意味分類)) は、パネルと提案の理由の末尾に
    出します (食い違いには「名前では eye_white / キャラクターの右」のように名前の規則の答えを並べる)。パネルの一覧ではその場で直せます。
    AI が同じ部品の別の絵 (差分) とした組 (`LayerLabelView.variant`。[ai-integration.md](ai-integration.md#レイヤーの意味分類))
    は、パネルの「Labels」の中に「Alternative drawings of the same part」として並べます (T0523)。
- **外部に送るとき:** 送る IPC コマンド (`classify_psd_layers`・`propose_rig_from_psd`) は、[送る前の確認](#ai-の設定と送信の同意)を
  `ai::prepare_requests` で通します (手元のサーバなら確かめずに送る)。
  - 分類はレイヤーが多いと複数のリクエストに分かれる (1 回 32 枚まで) ので、確認には **合計** (`DisclosureView::combined`) を示し、
    1 回の同意で送ります。送り先の違うリクエストは混ぜません。
  - リグは、答えが範囲の外か検証を通らなければ、同じ画像に前の答えと理由を添えて問い直します (既定で 3 回まで)。確認には
    1 回分の内容と、送り直すことがある回数 (`DisclosureView::retries`) を示します。
  - 「Use <プロバイダ>」は `aiSettings.canSend` が偽なら選べず、理由 (外部の送り先でプロジェクトが「外部に送らない」・
    キーの要るプロバイダでキーがない) と「Settings…」への案内を出します。キーがないときは、キーの保存か環境変数
    (`ANTHROPIC_API_KEY` など) を案内し、AI なしでも進められることを添えます。
- **提案の置き場 (`Proposals::add`):** 置く前に、ドキュメントの複製 (`Document::detached`。履歴なし) に全体を `Batch` として
  適用して検証を通すことを確かめ、通らなければ置きません。提案は作ったときのドキュメントの状態 (`Document::state_id`) に
  対するもので、ドキュメントが変わると **古い (stale)** とし、プレビューも採用もしません (承認した差分と違う結果に
  ならないように)。Undo で同じ状態に戻れば、また使えます。
- **プレビュー:** 選んだコマンドを複製に適用し (`preview_proposal`)、そのモデル (と `get_proposal_preview_bundle` のテクスチャ付きの
  バンドル) をビューポートに渡します。ドキュメントは変えません。プレビュー中はビューポートに「Previewing an AI proposal…」と出し、
  編集のツール (選択・ギズモ) を隠します。パネルには複製のパラメータ (提案が足したものを含む) のスライダーを出します。
  「Run physics」で提案の物理 (揺れ) を動かせます ([物理のプレビュー](#物理のプレビュー)と同じ `ViewportController.startPhysicsPreview`。
  スライダーを動かすと揺れ、採用の前に揺れ方を確かめられる。T0519)。プレビューをやめると物理も止まります (`ProposalsView.endPreview`)。
  ドキュメントが変わったら (編集・Undo・採用) プレビューをやめます。
- **採用:** 選んだコマンドを 1 つの `Batch` (ラベルは提案の題名。一部だけなら「(n of m commands)」を添える) として適用し、
  **1 回の Undo で戻ります**。採用した後に手で直したセルは、ほかの編集と同じく `authored` になります (推定に任せた `derived` の
  セルも)。採用した提案は一覧から隠しますが、Undo で提案を作った状態に戻ると再び出て、選び直して採用し直せます
  (AI にもう一度尋ねずに済むように)。却下 (「Reject」) は一覧から消します。
- **提案のカード (T0676):** 題名と印 (作り直しの元・エージェントの名前)・注文と反映していない注文・理由 (送らずに全体)・
  「採用で変わるもの」(選んだコマンドの種類ごとの数。`changeSummary`。種類は利用者の言葉で数を添えて出す: 「add 5 parameters」
  「パラメータを足す × 5」。訳はカタログの `proposals.changeKinds.*` で、`lib/ai/proposals.ts` の `CHANGE_KINDS` がコマンドの種類と
  キーを並べる。表に無い種類は、種類名のまま `add_parameter × 5` と出す (表が `ugoe_editor_core::Command` のすべての種類を
  持つことは proposals.test.ts が `command.rs` と照らし合わせて確かめるので、ふつうは起きない))・コマンドの一覧 (閉じて出す)・
  ボタンの順に並べます。ボタンは
  「Reject」「Regenerate with instructions」「Preview」「Adopt」で、決定の採用が並びの最後です。その下に「採用は選んだ n 個のコマンドを
  1 つの編集として適用し、1 回の Undo で戻る」ことを文で出します (採用のボタンの説明として読む)。古い提案は印でなく `Notice` (warning) で、
  プレビューも採用もできない理由と戻し方を出します。`primary` のボタンは画面に 1 つです: 提案が無ければ「Propose rig」、あれば
  プレビュー中の提案か、最初の採用できる提案の「Adopt」(法線マップの「Propose normal maps」は `secondary`)。
- **待ち・知らせ・失敗 (T0676):** 操作を始めた場所 (パネルの頭か提案のカード。`ProposalsView.scope`) に同じ形で出します。実行中は
  進み具合の文 (`role="status"`)、AI に送っている間は送り始めてからの秒数 (`ProposalsView.sentAt`。送る前の確認に答えて送り直すと
  数え直す) と「Cancel」、確認のダイアログに答えるのを待つ間は「送る内容の確認を待っています…」(取り消しは出さない)。取り消したら
  閉じられる `Notice` (info)、失敗は閉じられる `Notice` (danger、見出し「最後まで行えませんでした」)。カードの操作 (プレビュー・採用・却下・
  作り直し) の失敗はそのカードに、パネルの頭の操作 (分類・提案・法線マップ・分類の直し) の失敗はボタンの下に出し、読み直しなど場所の
  無い操作は場所を変えません。カードが一覧から消えていれば頭に出します。
- **一部だけの採用 (コマンドの単位):** コマンドごとにチェックを外せます。一覧は、同じ対象への続いたコマンドを 1 つのまとまりに
  まとめ (まとまりのチェックで全部を切り替える)、開くと 1 つずつ選べます。
  - 2 つ以上続いた分類の直し (`set_layer_correction`。エージェントがリグの提案に添えたもの。[分類の直し](#分類の直し)) は、対象 (レイヤー) が
    違っても 1 つのまとまり「Keep the classification of N layers」にし、開くと各行が出ます (T1631)。
    ほかの種類のコマンドとは同じ対象でもまとめません。直しはリグのコマンドが頼らない (`requires` に入らない) ので、まとまりを外してもリグは残ります。
  - 分類の直しの行は、レイヤーのパスと残すラベル・左右を `Face / Eye R → eye_white (right)` の形で出します (T1639)。ラベルと左右は
    コマンドの値のまま (`ProposalCommand::correction`。直しを消すコマンドでは省き、行は「→ back to the classification」)。まとめない 1 つだけの
    直しは `set_layer_correction Face / Eye R → eye_white (right)` の 1 行です。まとまりは閉じて出し、開いたときだけ中の行を描くので、
    レイヤーが多くても一覧は長くなりません (開いた中はすべての行を出し、長いパスやラベルは折り返します)。
  - 一覧に出す対象 (`ProposalCommand::subject`) は `id`・`target`/`element`・要素そのもの (`parameter` など) の ID・テクスチャの名前 (`set_normal_map` の `texture`。T0522)・レイヤーのパス (`set_layer_correction` の `path` を ` / ` でつなぐ)・`name` の順に探します。
  - 各コマンドの依存 (`ProposalCommand::requires`) は `proposal::analyze` が、コマンドの JSON が前のコマンドの **作った要素** か
    **主に変える要素** (`id`・`target`・`element`) の ID を指すかで決めます。同じ要素への後の編集は前の編集に依存するとみなします
    (推定の設定の後の推定、キーの追加の後のセルの置き換えなど、前の結果に頼るため)。種類ごとの規則を持たない近似で、
    外しすぎる側に倒れます。
  - チェックを外すと、それに (直接・間接に) 依存するコマンドも外れます。戻すと、それが依存するコマンドも戻ります
    (`src/lib/ai/proposals.ts`)。
  - それでも適用・検証に失敗する選び方なら、プレビューも採用もせず理由を出します (黙って直さない)。リグの提案では、どのコマンドを
    依存ごと外しても適用できることをテストで確かめています。

| コマンド | 内容 |
| --- | --- |
| `get_proposals` | `ProposalState { proposals, psd, classification, regenerable, sections, stale_rigs }` (一覧・元にする PSD・最後の分類 (分類の直しを重ねたもの。レイヤーごとの `LayerLabelView { path, label, side, confidence, needs_review, variant, name_disagreement, corrected }` と、直すときに選べるラベル `label_choices`)・作り直せる提案・コマンドの区切り・作り直しの知らせ) |
| `classify_psd_layers(psd?, use_ai, instructions?, revise, consent?)` | 分類して覚える。`AiReply<Classification>`。`psd` を省くと取り込んだ PSD。`revise` なら今の分類を前の答えとして送る |
| `propose_rig_from_psd(use_ai, instructions?, consent?, module_options?)` | 分類からリグを提案し、一覧に置く。`AiReply<ProposalView>`。`module_options` はモジュールのプロジェクトの組み方 (`ModuleRigChoice`。省略すると既定) |
| `regenerate_rig_proposal(id, instructions?, consent?, module_options?)` | 提案を前の答えと注文を添えて作り直し、後ろに並べる (いつも AI)。`AiReply<ProposalView>` |
| `propose_normal_maps(choices, replace_existing)` | 選んだまとまり (`NormalMapGroupChoice { group, settings }`) の法線マップを提案し、一覧に置く (AI なし)。`ProposalView`。[法線マップの提案](#法線マップの提案) |
| `preview_proposal(id, excluded)` | 外したコマンドを除いて複製に適用した姿。`ProposalPreview { model_json, report }` |
| `get_proposal_preview_bundle` / `clear_proposal_preview` | プレビューの `.ugoe` バンドル (生のバイト列) / プレビューの複製を捨てる |
| `adopt_proposal(id, excluded)` | 外したコマンドを除いて採用する (1 回の Undo で戻る)。`ApplyReport` |
| `reject_proposal(id)` | 提案を捨てる。`ProposalState` |
| `dismiss_stale_rigs()` | 本体の変形を写した後に形を編集した衣装のメッシュの知らせ (`ProposalState.stale_rigs`) を消す (今の形を覚え直し、`editor.rigged_meshes` に書く)。`ProposalState` |

### 分類の直し

AI タブの分類の確認の一覧で、レイヤーのラベル・左右を直し、名前の規則との食い違いを「これでよい」と認めて印を消します (T1615)。
操作の手順は [マニュアル](manual/ai.md#分類を直す)。

- **一覧:** 確かめたほうがよいレイヤー (`needs_review`) と、直したレイヤー (`LayerLabelView.corrected`) を並べます。各行でラベル
  (`Classification.label_choices`。[ai-integration.md](ai-integration.md#レイヤーの意味分類)の語彙) と左右 (キャラクター自身の左右) を選ぶと直し、
  「Looks right」で今の答えをそのまま利用者の答えにします。直した行は「checked by you」と出し、「Reset」で直しを消して分類の答えに戻します。
- **コマンド:** どれも `set_layer_correction { path, correction }` (`correction` は `{ label, side }` か、消すなら `null`) を 1 つ送り、
  **1 回の Undo で戻ります**。直しは `Document` のスナップショット (`LayerCorrections`) に入り、モデルは変えません (検証もしない)。
  同じ値なら何もしません (Undo を積まない)。`batch` の中でも使え、その `batch` と同じ 1 回の Undo で戻ります (T0570)。
  - **エージェントの提案から:** MCP でつないだエージェントのリグの提案は、エージェントが決めた分類をこのコマンドとして先頭に添えます
    ([ai-integration.md#エージェントに任せる自動リギング](ai-integration.md#エージェントに任せる自動リギング))。採用すると利用者の直しと同じ
    直しになり (一覧で「checked by you」)、外して採用すればリグだけが入ります。提案の一覧ではレイヤーのパスを ` / ` でつないで対象に出し、続いた直しは 1 つのまとまりにします
    ([AI の提案](#ai-の提案)の一部だけの採用。まとめて外す・戻すか、1 行ずつ外せます)。
  - 受け付ける形: パスは 1〜64 段で、名前は 1024 文字まで。ラベルは 1〜64 バイトの小文字の英数字と `_`。直しは 65536 件まで。
    外れれば `edit.invalid_layer_correction` で拒み、何も変えません。ラベルの語彙は ugoe-editor-core は知らない (`ugoe-ai` に依存しない) ので、
    語彙にないラベルの直しは受け付けますが、重ねる側 (エディタ) が無視します。
  - 直しもドキュメントの状態を変えるので、未保存の印が付き、置いてある提案は古く (stale) なります (直した分類でリグを提案し直す)。
- **重ね方 (`ugoe_ai::classify::corrected_labels`。MCP サーバと共有する):** 分類の答えのうち、パス (`LayerLabel::layer_path`) が同じレイヤーのラベルと
  左右を直しで置き換え、人が確かめたものとして確からしさを 1 にし、名前の規則との食い違いの印を消します (差分の組は保つ。
  直しは組を持たない)。
  画面の一覧 (`get_proposals`)・リグの提案 (`propose_rig_from_psd`・`regenerate_rig_proposal`)・分類の作り直しで送る前の答え (`revise`)・
  法線マップのまとまり (`get_normal_map_groups` など) は、どれも重ねた分類を使います。分類し直しても直しは残り、新しい答えの上に重ねます。
- **保存:** `project.json` の `editor.classification` (`{"corrections": [{"path": [...], "label": "...", "side": "left"}]}`。
  [file-formats.md](spec/file-formats.md#プロジェクト-ugoeproj)) に、保存したときに書きます。PSD のパスは持たず、レイヤーのパスだけで対応させるので、
  同じ構成で書き出し直した PSD にも効きます。直しがなければ項目を書きません。読めない件は捨てます。
  `.ugoe` には書き出しません。外部のエージェント (MCP サーバ) も同じ規則でこの直しを重ねます (T0566。
  [ai-integration.md#エージェントに任せる自動リギング](ai-integration.md#エージェントに任せる自動リギング))。
- **差分の組は残さない (T0573):** エージェントの提案を採用しても、エージェントが決めた差分の組 (`variant_sets`) は `project.json` に書きません。
  組を使うのはモジュールのリグのポーズだけで、採用したリグの組はポーズのグループとしてモデルに残ります。開き直した後に出すモジュールのリグは、
  名前の末尾の A / B と PSD で片方を隠した重なり (とエディタの AI の組。[分類の答えのキャッシュ](#分類の答えのキャッシュ)) で組を見つけます。

### 分類の答えのキャッシュ

分類の答え (名前の規則と AI の答えのラベル・左右・確からしさ・差分の組) を、プロジェクトの中のキャッシュ
`.ugoe-cache/classification.json` に残し、開き直しても分類し直さずに (AI を使ったなら呼び直さずに) 済むようにします (T0567)。
形は [file-formats.md#分類のキャッシュ](spec/file-formats.md#分類のキャッシュ)、読み書きは `ugoe_editor_core::classification_cache`。

- **置き場:** `project.json` は変えません。`.ugoe-cache/` はプロジェクトの `.gitignore` に入っていて、保存・読み込み・
  [外での書き換え](#外での書き換え)の指紋では触れません (キャッシュを書いても外の変更として知らせない)。
- **書くとき:** 保存先のあるプロジェクトで分類したとき (`classify_psd_layers`。ロックの外ですぐに書く) と、保存したとき
  (`save_project`・`save_project_as`。分類があれば保存先に書く)。分類していなければ前のキャッシュに触れません。直し
  ([分類の直し](#分類の直し)) は重ねずに書きます (直しは `project.json` に残り、開いたときに上に重ねる)。
  書けなくても分類や保存は失敗させません (標準エラーに書くだけ)。中身が同じなら書き直しません。
- **戻すとき:** 保存先のあるプロジェクトを開いた後 (`open_project`・`reload_project`・`open_module_with_base`・
  `open_sample_project`)。開く処理は PSD を読まずに終わり、戻すのはその後のバックグラウンド (`spawn_blocking`。ロックの外) です
  (`proposals::restore_in_background`。T1629)。キャッシュの PSD が今もあり、大きさと中身のハッシュ
  (FNV-1a 64 ビット) が同じで、レイヤーの数・パスが合い、確からしさが 0..=1 のときだけ戻し (`ugoe_ai::classify::cached_labels`)、
  その PSD を提案の元にします。名前の規則との食い違いの印は今の規則で付け直します。合わない・読めないときは何も戻さず
  (キャッシュは消さない)、分類し直します。大きな PSD では、開いてから分類が AI タブに出るまでに PSD の大きさに応じた時間がかかります。
  - **古い戻しは捨てる:** 戻し始めたときのセッションの番号 (`ProjectSession::id`。開く・閉じるたびに変わる) と、提案の元の PSD と
    分類の世代の番号 (`ProposalSession` の `epoch`。分類した・PSD を取り込んだ・提案の元の PSD を選んだら増える) を覚え、
    PSD を読み終えた後でどちらかが変わっていたら置きません (遅れて終わった戻しが、ほかのプロジェクトや分類し直した結果を上書きしない)。
  - **画面への知らせ:** 戻している間は `ProposalState.restoring` が真で、AI タブの PSD の行の下に「このプロジェクトに残した
    レイヤーの分類を読んでいます…」と出します (T0676)。印は開いたときに立て (キャッシュのファイルがあるときだけ。無ければ戻す仕事も作らない)、
    戻し終えた・戻せなかった・その間に分類し直した・PSD を選び直したときに下ろします。終えたら、戻せなくても
    イベント `classification-restore-finished` (中身なし。`ipc.onClassificationRestoreFinished`) を送り、画面は `get_proposals` を
    読み直します (AI タブの分類と PSD が出て、戻している印が消える)。
  上限を超える注文 (`Instructions::MAX_CHARS`) は捨てます (分類は戻す)。
- **Undo:** キャッシュなので Undo の対象にせず、`Document` にも入れません。未保存の印も付けません。
- **MCP サーバ:** 同じファイルを読み、同じ PSD の分類なら名前の規則の案の代わりに使います (書くのはエディタだけ。
  [ai-integration.md#エージェントに任せる自動リギング](ai-integration.md#エージェントに任せる自動リギング)の「エディタが残した分類」)。

### 法線マップの提案

AI タブの「Normal maps from the shapes」で、すべてのテクスチャの法線マップを部位のまとまりごとに設定して 1 つの提案にし、プレビューしてから採用します (T0520)。
作り方は Lighting タブの「Generate」と同じ `ugoe_ai::normal` (アルファの形のふくらみ。[ai-integration.md#まとまりでふくらませる](ai-integration.md#まとまりでふくらませる)) で、
**AI は使わず、何も送りません**。操作の手順は [manual/ai.md](manual/ai.md#法線マップを提案する)。

- **まとまりの一覧:** `get_normal_map_groups` が、法線マップを作る対象 (マスク専用と背景を除くメッシュのテクスチャ) のまとまりを返します。ラベルは Lighting タブと同じく
  メッシュの名前の規則に、パネルで分類していればその結果を重ねたもので、分類し直すと一覧も読み直します。単独でふくらませるもの (`alone`・ラベルなし) は 1 行にまとめます。
  行ごとにテクスチャの数・法線マップを持つ数・向きだけを受ける数 (眼鏡) を出します。
- **選ぶ:** 行ごとに含めるか (既定はすべて) と強さ (0〜2) を選び、行の下の開閉 (縁とならしの今の値を出す) で「Round edges only」・縁の幅・ならしを選びます
  (T0522。既定はまとまりの既定、「Defaults」で戻す)。長さは Lighting タブと同じくキャンバスの px で見せ、送るときにまとまりの格子の画素に直します
  ([ライティングの画面](#ライティングの画面)の「長さの単位」)。キャンバスの px で覚えるので、分類し直してまとまりの格子が変わっても見た目の幅は変わりません。
  選び方はエディタを開いている間だけ覚えます。「Replace existing normal maps」を付けなければ、法線マップを持つテクスチャは飛ばします。
- **提案:** `propose_normal_maps` が `set_normal_map` の列 (ライティングが無効なら最後に `set_lighting`) を作り、一覧に置きます (題名の末尾は「— no AI」)。
  作る間はドキュメントのロックを持たず、その間にドキュメントが変わったら置きません。モジュールのプロジェクトでは断ります (本体のテクスチャとライティングは変えられない。
  パネルにも出さない)。
- **区切り:** 提案のコマンドはまとまりごとの区切り (`ProposalState::sections`、`CommandSection { label, indices }`) にし、一覧はその単位でまとめます (区切りの中のコマンドは `ProposalCommand::subject` のテクスチャの名前付きで出す。T0520 の `subjects` は
  `proposal::analyze` がテクスチャの名前を拾うようにしたので T0522 で除いた)
  (まとまりのチェックで、そのまとまりの法線マップを外せる)。区切りのない提案 (リグなど) は今までどおり、同じ対象への続いたコマンドをまとめます。
- **プレビュー:** 提案の複製はライティングが有効なので、ビューポートはプレビューの光源 (Lighting タブの Preview lights) で照らした姿を出します。

### モジュールのプロジェクトでの AI の提案

モジュールのプロジェクト (T0648) では、同じ AI タブの「Propose rig」が `ugoe_ai::module_rig` の提案を作ります (T0517)。
モジュールはパラメータを足せず、本体の要素を変えられないので、モデルの自動リギング (パラメータを作る) は使いません。

- **分類:** 取り込んだ衣装の PSD のレイヤーを、モデルと同じ規則・AI で分類します (`topwear`・`headwear` など)。ラベルは
  モジュールのメッシュだけに対応付けます (`module_mesh_parts`。本体に同じ名前のメッシュがあっても付く)。
- **提案の中身:** 衣装を本体の変形 (ワープと、目の開閉のような本体のメッシュのキーフォーム) に追従させるキーフォーム、
  揺れるラベルの衣装の物理 (スロットが公開する本体の衝突体に当てる)、衣装の差分 (A / B) のポーズ (規則は
  [ai-integration.md#モジュールの自動リギング](ai-integration.md#モジュールの自動リギング))。キーの軸と物理の入力は、本体にある
  標準パラメータとスロットが公開するパラメータだけです。提案は作業用のドキュメントの複製に適用し、モジュールとして分けられる
  (`Document::split_module`) ことを確かめてから置きます。
- **AI が決める値:** 揺れの大きさと振り子の長さ (`ModuleRigParams`)。注文・作り直し・問い直しの流れはモデルと同じで、
  作り直しには前の答えとしてモジュールの値を送ります (`proposals.rs` の `PreviousAnswer`。プロジェクトの種類が違う前の答えは断る)。
- **画面:** AI タブの見出しと説明、注文の欄の説明がモジュール向けになります (`ProposalsPanel` の `module`)。新規作成の画面と
  `Module:` の印のツールチップでは、できることに出します (`MODULE_CAN`)。
- **組み方 (「How to rig」。AI には送らない。`ModuleRigChoice` → `ugoe_ai::module_rig::ModuleRigOptions`、T0519):**
  - 「Follow the base warps exactly」: 本体のワープの変形を、衣装を包む追従のワープ (既定。衣装を描き直しても組み直さずに済むが、
    格子で近似するので振り向きの移動量の 1 割ほどずれる) ではなく、衣装のメッシュのキーフォームに直接写します (キーの上でずれない。
    メッシュの頂点を編集したら提案し直す。下の「作り直しの知らせ」)。
  - 「Keep swaying parts out of the base's colliders」(既定で入): 揺れのリグを、スロットが公開する本体の衝突体に当てます。
  - 「Turn A / B variants into poses」(既定で入): 名前の末尾の文字だけが違う衣装と、絵が重なり意味ラベルが同じで PSD で片方を
    隠してある衣装をポーズにします (T0521)。
  - 選択は提案と作り直しに渡し (`propose_rig_from_psd`・`regenerate_rig_proposal` の `module_options`)、エディタを開いている間だけ覚えます。
- 取り付け先の下に衣装を覆う本体のワープも、衣装が重なる形の変わる本体のメッシュもなく、揺れるラベルの衣装も差分もなければ、組むものが
  ないと断ります (衣装は取り付け先の動きだけに従う)。
- **作り直しの知らせ (T0521):** モジュールのリグの提案は、衣装のメッシュのキーフォームに本体の変形を頂点ごとに写すことがあります
  (直接写す形と、本体のメッシュへの追従)。採用した後に、そのメッシュの形 (頂点・三角形・親。`MoveMeshVertices` は UV も変える) を
  編集すると、写した変形が合わなくなります。バックエンド (`ProposalSession`) は、採用した提案のうちメッシュのキーフォームを書く
  コマンド (`add_keyform_key`・`set_keyform_cell`。外したコマンドは除く) の対象のメッシュと、採用したときの形を覚え、形が変わった
  ものを `ProposalState.stale_rigs` に出します。AI タブの上に「Propose the rig again」とメッシュの名前を出し、タブにも印を付けます。
  「Dismiss」(`dismiss_stale_rigs`) は今の形を覚え直して知らせを消します。今キーフォームの軸を持たないメッシュ (採用を Undo した
  もの) と消えたメッシュは知らせません。インスペクタでも、選んだメッシュが知らせの対象なら上に「Propose the rig again」と
  「Open AI tab」を出します (`panels/inspector/StaleRigNotice.svelte`、T0523)。
  - **プロジェクトに残す (T0523):** 覚えた形は、形 (親のデフォーマの ID・UV・三角形) の指紋 (FNV-1a の 64 ビット。16 桁の 16 進) にして
    `project.json` の `editor.rigged_meshes` (`{"<メッシュの ID>": "<指紋>"}`) に置き、開き直しても知らせます
    ([file-formats.md](spec/file-formats.md#projectjson))。ID と親の ID はファイルの中の ID (モジュールのプロジェクトでは作業用のモデルの
    接頭辞を外したもの。取り込んだばかりのメッシュと開き直したメッシュで同じになる)。採用と「Dismiss」で書き換え、未保存の印を付けます
    (保存するまでファイルには書かない)。項目のない前からのプロジェクトは何も覚えていない状態で開き、形の違う値は捨てます。

### AI への注文と作り直し

AI タブの「Instructions for <プロバイダ>」(複数行) に書いた利用者の注文を、分類・提案に添えて送り、できた提案を注文を添えて
作り直せます (T0644)。プロンプトへの入れ方と、反映できない注文の示し方は [ai-integration.md#利用者の注文](ai-integration.md#利用者の注文)。

- **送るとき:** 注文は「Use <プロバイダ>」のときだけ、「Classify layers」「Propose rig」「Regenerate with instructions」に添えます
  (前後の空白を除いて 2000 文字まで。超えるとボタンを無効にし、バックエンドも断る)。AI を使わないときは送らず、欄の下に
  「Only sent when "Use …" is on」と出します。送る前の確認 ([AI の設定と送信の同意](#ai-の設定と送信の同意)) には、テキストのバイト数に
  加えて注文の文字数 (`DisclosureView::instruction_chars`) を示します。注文を変えると確認の内容が変わるので、もう一度確かめます
  (「今後は確かめない」を選んだ送り先を除く)。「外部に送らない」のプロジェクトでは、作り直しも外部には送りません。
- **作り直し (「Regenerate with instructions」):** このパネルで作ったリグの提案 (`ProposalState::regenerable`。つないだエージェントの
  提案は除く) にだけ出ます。前の提案の値と理由 (AI を使わなかった提案はテンプレートの既定値) と今の注文を送り、**新しい提案を
  一覧の後ろに並べます。前の提案は置き換えません。** 両方をプレビューで見比べ、要らない方を「Reject」します。新しい提案には
  「revision of #n」の印を付けます (`ProposalOrigin::revises`)。作り直しはいつも AI を使うので、「Use <プロバイダ>」が外れているか
  注文が空ならボタンを無効にします。
  - 分類は今の分類を使います。「When regenerating, also reclassify the layers…」に印を付けると、先に今の分類を前の答えとして
    注文を添えて分類し直します (`classify_psd_layers` の `revise`。レイヤーの画像をもう一度送るので、確認は 2 回)。同じ注文で
    分類し直したときは、ラベルに関わる注文を「反映できない」に数えさせません。
  - 置き方・プレビュー・採用 (1 回の Undo) は、ふつうの提案と同じです。作り直しの間にドキュメントが変わったら置きません。
- **提案の表示:** 提案の番号 (`#n`)、添えた注文 (`Instructions:`)、AI が 4 つの値では反映できないと答えた注文 (`Not applied`。
  `ProposalView::unapplied`、AI の言葉のまま) を理由の上に出します。分類の要約には「with instructions」と出します。
- **保存しない:** 注文の欄の中身はエディタを開いている間だけ持ち、プロジェクトには保存しません (`project.json` の形を変えず、
  利用者の注文が意図せずファイルに残らないように)。提案に添えた注文も、提案と同じくプロジェクトを開いている間だけです。

### エージェントをつなぐ

実行中のエディタに外部のエージェント (Claude Code・Claude Desktop・OpenCode・ChatGPT) をつなぎ、その提案をエディタの画面で確かめて採用する
([ADR 0023](adr/0023-editor-mcp-bridge.md) の 1・2・4・6、T0642)。`ugoe-mcp` の側 (持ち主の抽象・`EditorBridge`・`--editor` の中継・WSL) は
T0516 ([crates/ugoe-mcp/README.md](../crates/ugoe-mcp/README.md#実行中のエディタにつなぐ---editor))。バックエンドは `src-tauri/src/mcp.rs`、
画面は `src/lib/agents/` (設定の「Connect agents」・確認のダイアログ・登録の手順)。操作は [マニュアル](manual/mcp.md)。

- **持ち主 (`EditorHost`):** `AppState` の文書 (`document`) と `ProjectSession` のメタ情報・提案 (`ProposalSession` の `Proposals`。AI タブと
  同じ置き場) を `ugoe_mcp::DocumentHost` として貸す。錠の順は `document` → `project` (IPC と同じ)。ツールは HTTP の接続のスレッドで
  動き、Tauri のメインスレッドは止めない。
  - **指せるのは開いているプロジェクトだけ:** パス (正規化して比べる) が `ProjectSession::path` と同じときだけ貸す。未保存 (パスなし) なら
    保存を促すエラー。`open_project` は開いているプロジェクトなら状態を返すだけ (ディスクの内容で置き換えない)、作る・`import_psd`・
    閉じるは断る (利用者の作業を奪わない)。`instructions` で、エディタにつながっていることと開いているプロジェクトのパスを伝える。
  - **確認:** `apply_proposal`・`undo`・`redo`・`save_project` は `ugoe_mcp::host::Approvals` で確認を出し、画面の答えか **120 秒** の
    時間切れまで待つ (拒否・時間切れはツールのエラー)。確認は `ApprovalRequest` (操作・最後に握手したクライアントの名前・提案の番号と題名・
    外すコマンド・書き出し先) を Tauri のイベントで画面に送る。確認の間も錠は握らないので、利用者は編集を続けられる (その場合は提案が
    古いというエラーになり、承認した差分と違う結果にしない)。**画面から「Adopt」するときは確認しない** (利用者自身の操作)。
  - **保存:** エディタの保存と同じ `ProjectSession::begin_save` (ロックの外で書き、指紋を取り直すので、自分の保存を外の変更と取り違えない)。書き出し先 (`bundle`)
    はプロジェクトの書き出しの設定 (アトラス化) で書く。`ugoe-mcp` と同じく、`.ugoe` は先に一時ファイルへ書き、プロジェクトを保存できて
    から置き換える (どこで失敗しても前の `.ugoe` は残る)。開いた後で外で書き換えられていたら、エージェントの保存は断る (利用者が
    [外での書き換え](#外での書き換え)のダイアログで決める)。
  - **外での書き換えとぶつからない:** ブリッジの編集はエディタの文書に入り、ディスクには保存するまで書かないので、外の変更としては見えない。
- **提案の出どころ:** `Change::Proposed` の名前を `Proposals::set_source` で提案に記し、`ProposalView::source` として画面に出す
  (「from claude-code」。エディタの中で作った提案は `null`)。HTTP はセッションを持たないので、名前は最後に握手したクライアント。
- **画面へのイベント (`mcp-bridge`、`BridgeEvent`):** `status` (始めた・止めた・握手した)・`approval` / `approval_closed`
  (終わり方 `ApprovalOutcome` を添える。エージェントの取り消し (`cancelled`) なら画面は Toast で知らせる。T1603。取り消しは `HostError::Cancelled` の種類で見分ける。T0557)・`document`
  (適用・Undo・Redo・保存。画面は `EditorDocument.refresh`)・`proposals` (一覧を取り直し、AI タブを開いていなければタブのアイコンの右上に点)。
- **設定 (`mcp-bridge.json`):** アプリの設定のディレクトリに `enabled` (既定は偽)・`port` (無ければ空いているポート)・`token` を残す。
  トークンは初めて有効にしたときに OS の乱数で作り、起動し直しても同じ (HTTP で直接つなぐクライアントの設定を保つ)。Unix では一時ファイルも
  0600 で作り、名前を変えて置き換える (書いている途中で失敗してもトークンと有効の設定を失わない)。
  「Regenerate token」で作り直す。起動時に有効なら始め、終了時 (`RunEvent::Exit`) に止めて発見用のファイルを消す (待っている確認は拒否)。
- **登録の手順 (`src/lib/agents/setup.ts`、Vitest):** 同梱の `ugoe-mcp` (エディタの実行ファイルの隣。Windows では WSL から指すパス
  `/mnt/c/…` も) を埋めた、Claude Code (`claude mcp add … -- <ugoe-mcp> --editor`)・OpenCode (`opencode.json`)・ChatGPT
  (`tunnel-client` の `--mcp-command "<ugoe-mcp> --editor"`)・WSL (`ugoe-mcp.exe --editor --wsl-distro`、ミラーのときは Linux 版)・HTTP の
  手順。パスは OS に合わせて引用する。
- **Claude Desktop (MCP Bundle、T0643):** 「Claude Desktop」のタブで、同梱の `ugoe-mcp` を `--editor` で動かす `.mcpb` をその場で作る
  (`ugoe_mcp::mcpb::Bundle`。束の OS はエディタの OS、アイコンはエディタの `icons/icon.png`)。「Install in Claude Desktop…」は
  アプリのキャッシュのディレクトリに `ugoe-editor.mcpb` を書いて OS に開かせ (tauri-plugin-opener の `open_path`。Claude Desktop が
  インストールの画面を出す)、「Save .mcpb…」は保存のダイアログで選んだ場所に書く。圧縮に数秒かかるので Tauri の非同期のコマンドで
  `spawn_blocking` する。インストーラには `.mcpb` を別に入れない (同じ実行ファイルを 2 つ持たずに済み、束の版と OS がエディタと必ず
  そろう。開発中も `target/` の `ugoe-mcp` から同じように作れる)。束は `ugoe-mcp` の複製を持つので、エディタを更新したら入れ直す
  (画面とマニュアルに書いた)。`.mcpb` を使わない形として `claude_desktop_config.json` の `mcpServers` の JSON も写せる。
  エディタを WSL の中で動かしているときは、Windows の Claude Desktop から届かない旨を出す。
- **ugoe-mcp の同梱 (サイドカー):** Tauri の `externalBin` (`binaries/ugoe-mcp-<ターゲットの三つ組>[.exe]`)。`tauri.conf.json` には書かず、
  `src-tauri/tauri.bundle.conf.json` を `just editor-build` が `--config` で重ねる (tauri-build は `externalBin` のファイルが無いと
  `cargo check`・`tauri dev` も失敗させるため)。重ねた設定の `beforeBuildCommand` が `pnpm sidecar` (`scripts/sidecar.mjs`) で
  ターゲット向けに `ugoe-mcp` をリリースでビルドして置く (Linux から Windows 向けは cargo-xwin、`universal-apple-darwin` は lipo)。
  開発中 (`just editor`) は同梱の `ugoe-mcp` が無いので、画面は PATH の `ugoe-mcp` を使うコマンドを出す。

| コマンド | 内容 |
| --- | --- |
| `mcp_bridge_status` | `McpBridgeStatus` (有効・待ち受けの URL・ポート・発見用のファイル・エラー・握手したクライアント・待っている確認・エージェントが使うパス・同梱の `ugoe-mcp`)。トークンは含めない |
| `set_mcp_bridge_enabled(enabled)` | 有効・無効を設定に残し、始める・止める |
| `set_mcp_bridge_port(port?)` | 待ち受けるポート (`null` なら空いているポート)。動いていれば始め直す |
| `regenerate_mcp_bridge_token` / `get_mcp_bridge_token` | トークンを作り直す (動いていれば始め直す) / 写すために読む |
| `answer_mcp_approval(id, approved)` | 確認に答える。待っていなければ (時間切れの後など) `false` |
| `open_claude_desktop_bundle` | Claude Desktop 用の `.mcpb` をアプリのキャッシュに作って OS に開かせ、そのパスを返す (非同期) |
| `save_claude_desktop_bundle(path)` | Claude Desktop 用の `.mcpb` を `path` (拡張子は `.mcpb`) に書き、そのパスを返す (非同期) |

## ビューポート

[ADR 0003](adr/0003-editor-viewport-in-webview.md) のとおり、ビューポートは WebView 内の wasm で描画します。

- バックエンドはモデルが変わるたびに、モデルのバイト列をフロントエンドへ送ります (`tauri::ipc::Response` で生のバイト列を送る)。
- パラメータスライダーの操作はフロントエンドの wasm の中だけで完結させます (IPC を往復しない)。これにより 60fps を保ちます。スライダーの値はドキュメントの状態ではなく、表示の状態です。
- WebGPU が使えない WebView (Linux の WebKitGTK など) では、wgpu の WebGL2 バックエンドにフォールバックします。
- GPU を失ったら canvas ごと作り直します。WebGPU のデバイスの喪失も WebGL2 のコンテキストの喪失も `WebPuppet.is_lost()` で分かり
  (WebGL2 は wasm が canvas の `webglcontextlost` で印を立てる)、WebGL2 は `webglcontextrestored` を待ってから、次に描くのを待たずに作り直します。
  復元を待つ間はビューポートの右上に「GPU の復元を待っています」と出し、5 秒待っても復元が届かなければ新しい canvas で作り直しを試します
  (作れなければ 5 秒ごとに試し直す。T0938)。前の作り直しから 1 分以内に失う・作れないことが続き 4 回目になったら、作り直さずに
  ビューポートに「GPU を続けて失った」と「作り直す」ボタンを出します (T0935・T0938)。ボタンを押すと、同じ場所に作り直して表示中のモデルと
  パラメータの値を渡し直します。
  devicePixelRatio だけが変わったとき (倍率の違う画面へ移したなど) も、描画バッファの大きさを合わせ直します。
  WebPuppet と canvas の寿命はビューアと同じ `@ugoe/player` の `PuppetSurface` が持ちます ([ui.md](ui.md#webpuppet-の寿命)。T0932)。
- モデルの読み込み (`load_bundle`・`load_json`) の失敗は、wasm の `LoadError` の `code` と補足の値から今の言語の文にします
  (ビューアと同じ `@ugoe/player` の `describeLoadError`。訳はそのカタログ (名前空間 `player`) の `loadError`。T1611)。
  編集中もプレビューモードも同じ `ViewportController` で読むので、どちらにも出ます。知らない `code` と `LoadError` でない例外は
  wasm の文 (英語) のままです。読み込みの途中に GPU を失った (`device_lost`。`is_lost()` が真) ときは読み込めなかったとはせず、文を
  出さずに作り直し (WebGL2 は復元を待つ) を始め、作り直した WebPuppet に今のモデルを読みます (T0955。ビューア・`<ugoe-avatar>` と同じ。
  [ui.md](ui.md#webpuppet-の寿命))。`is_lost()` が偽なのに `device_lost` が来たときは、ビューアの「アプリを起動し直して」ではなく
  エディタのカタログの文 (`viewport.deviceLost`。エディタを起動し直す案内。スクリーンショットと同じ文) にします (`viewportErrors.ts` の
  `loadErrorText`。T0941)。
- WebPuppet を GPU やブラウザの都合で作れなかった (`CreateError`。T0475) ときは、`code` ごとのエディタのカタログの文
  (`viewport.createError.*`) をビューポートに出し、「作り直す」ボタンも出します (`viewportErrors.ts` の `createErrorText`。T0941)。
  知らない `code` と、`code` を持たない例外は例外の文のままで、ボタンは出しません ([ui.md](ui.md#webpuppet-の寿命))。
  作り直し (「作り直す」ボタンや GPU を失った後) でも作れずに諦めたときは、「GPU を続けて失った」ではなく作れなかった理由の文に
  諦めたことを添えて出します (`GpuLostRepeatedlyError` の `cause`。`viewport.recreateFailed`・`surfaceFailureText`。T0946)。
- 大きなモデルで、モデル全体を送るのが遅くなった場合は、差分パッチを送る方式を検討します。

### ビューポートの並び

ビューポートの上に重ねる道具と知らせの置き場です (T0675。[ui.md の「エディタのビューポートとタイムライン (T0675)」](ui.md#エディタのビューポートとタイムライン-t0675))。

| 場所 | 置くもの |
| --- | --- |
| 左上 (`ToolOverlay` の `.strip`) | 1 行目は道具のボタン (「Stop physics」・「Edit mesh」・「Form edit」・「Module outline」、メッシュの編集中は「Retriangulate」・「Delete vertices」・「Done」)。その下に今の道具の **説明の帯** (`.vp-band`): 対象とセル (`Warp: HeadWarp AngleX=0`)・「All」と選んだ数・キーの上にいない注意 (三角の印と文と「Move to nearest key」)・操作の説明・失敗の文。帯はテーマの地と枠に載せ (モデルの上でも読める)、並びは右上の HUD の幅を空けて折り返す (重ならない) |
| 右上 (HUD) | 「Screenshot」と、GPU を待つ・失ったときの知らせと「Try again」 |
| 右下 | 表示の道具 (縮小・拡大・全体を表示。ビューアと同じ位置と形)。拡大と縮小はビューの中央を中心に 1.25 倍ずつ |
| 左下 | グルーの頂点の組の帯、プレビューの違いの知らせ (右下の表示の道具の幅を空ける) |
| 中央 | 空のプロジェクトの次の操作 (T0673) |

- 描画の方式 (`webgpu`・`webgl2`) は開発者向けの情報なので、ビューポートではなく設定の「一般」の「ugoe のエディタについて」に出します。
- **図の色:** ギズモ・メッシュの点・選択の輪郭・物理・グルー・光源の色はテーマに依らない CSS の変数 (`--viewport-*`。`Viewport.svelte`) です。
  線はライトとダークのビューポートの地のどちらにも 3:1 以上の色、点は白の中と線の色の縁、選んだ点は琥珀の中と濃い縁で描きます
  (値と理由は [ui.md の「ビューポートの上の図」](ui.md#ビューポートの上の図))。

### ビューポートの操作と代わりの手段

ビューポート (モデルの描画) は WCAG の対象の外ですが、そこで行う操作にはパネルかキーボードでの代わりを用意します
([ui.md の「アクセシビリティの基準」](ui.md#アクセシビリティの基準))。T0675 で操作ごとに確かめた結果です。

| ビューポートの操作 | 代わりの手段 |
| --- | --- |
| ホイールで拡大・縮小 | 右下の「縮小」「拡大」(ビューの中央を中心に) |
| 中ボタン・Alt + ドラッグで移動 | Alt + 矢印キー (40px、Shift も押して 200px。絵が矢印の向きに動く。`tools/keys.ts` の `panOf`。T1635)。右下の「全体を表示」で全体に戻せる |
| クリックでメッシュを選ぶ・Shift / Ctrl で足す・Alt で重なりを順に | ツリーの選択 (矢印・Shift・Ctrl。T0674) |
| ダブルクリックでメッシュの編集に入る | 「Edit mesh」 |
| メッシュの編集: 頂点を選ぶ (クリック・矩形) | `]` `[` で次・前の頂点を 1 つ (ビューの外なら見える所まで表示を動かす。T1635・T1647)、Ctrl + A ですべて |
| メッシュの編集: 頂点のドラッグ | 矢印キー (1px、Shift で 10px。画面のピクセル) |
| メッシュの編集: Ctrl + クリックで頂点を足す | **無し** (位置を指す操作。インスペクタのメッシュの作り直しで形を作り直せる) |
| メッシュの編集: 頂点を消す・作り直す・抜ける | Delete・「Delete vertices」、「Retriangulate」、Esc・「Done」 |
| フォーム編集・ワープ: 点を選ぶ (クリック・矩形) | `]` `[` で次・前の点を 1 つ (ビューの外なら見える所まで表示を動かす。T1635・T1647)、「All」・Ctrl + A ですべて |
| フォーム編集・ワープ: 点のドラッグ | 選んだ点を矢印キーで動かす (1 回が Undo の 1 単位。`set_keyform_cell`) |
| 回転のギズモ: 原点のドラッグ | 矢印キー (「Origin: all cells」に従う) |
| 回転のギズモ: 回転・拡大率の取っ手 | インスペクタの「Keyforms」の「Angle (this cell)」「Scale (this cell)」(今のセルの値を数値で入れる。T1636) |
| 物理: 衝突体のドラッグ・粒子の鎖を選ぶ | 物理のパネルの衝突体の欄 (位置・半径)、リグの一覧 |
| 光源の取っ手のドラッグ | ライティングのパネルの光源の欄 (向き・距離・範囲)。取っ手もフォーカスして矢印で動く |
| グルー: 頂点を押して組を足す・外す | グルーのパネルの「距離で組み直す」 (1 組ずつは無い) |
| はみ出し・深度の表示 | 表示だけ (検証のパネル・インスペクタの深度の節で切り替え) |

矢印キー・Alt + 矢印キー・`]` `[` は、フォーカスがどこにも無い (ビューポートを押した後) か、ビューポートの中のボタンにあるときだけ効きます
(`tools/keys.ts` の `acceptsNudgeKey`。ツリー・スライダー・タイムラインの矢印は奪わない)。入力欄とダイアログの中でも効きません。

- **点を 1 つずつ選ぶ (`]` `[`。T1635):** `]` で次、`[` で前の点を 1 つだけ選びます (`keys.ts` の `pointStepOf`・`steppedPoint`)。
  順は点の番号 (メッシュの頂点の順・ワープの格子の行ごとの順)。選んだ点が無ければ `]` は最初・`[` は最後から、あれば最後に選んだ点
  (選択の末尾) から数え、端で反対の端に戻ります。Tab はフォーカスの移動に残します (ビューポートから出られなくしない)。
  1 つだけ選んだ点は輪で囲み (`.ring`)、帯の `role="status"` の領域に「Point 3 of 12」(`viewport.point.current`) を出して読み上げます。
  フォーム編集・ワープのギズモ・メッシュ編集で同じです (メッシュ編集のキーは ToolOverlay が受けて `MeshEditTool.step` を呼ぶ)。
- **選んだ点を見える所へ (T1647):** `]` `[` で選んだ点がビューの外か、端から 48px (`REVEAL_MARGIN`) の余白の中にあれば、
  端から 48px の所まで表示を動かします (`ViewportController.reveal`。量は `keys.ts` の `revealPanOf` で、軸ごとに足りない分だけ。
  ビューが余白の 2 倍より狭い軸は真ん中へ)。拡大率は変えず、余白より内側の点では動かしません。ポインタで選んだときは動かしません
  (押した点は見えている)。
- **パン (Alt + 矢印。T1635):** `Viewport.svelte` が受け、`ViewportController.panBy` を呼びます。Alt + ドラッグと同じく絵を矢印の向きに
  動かします。Alt の無い矢印は点を動かす操作なので重なりません。

### スクリーンショット

ビューポートの右上の「Screenshot」で、今の表示を PNG で保存します (T0420)。

- 画像は canvas の読み戻しではなく `WebPuppet.read_pixels()` で取ります (`ViewportController.capture`)。canvas と同じ大きさ
  (描画バッファの画素 = CSS ピクセル × devicePixelRatio)・ビュー・背景 (`--viewport-bg`)・光源で canvas とは別のテクスチャに描いた、
  ストレートアルファの RGBA です。WebGL2・WebGPU のどちらでも同じに取れます。
- **写るもの:** WebPuppet が描くモデルだけ。編集中はスライダーの値の姿、プレビューモードでは押したときのフレーム (物理・目パチ・
  モーションなどを含む)、AI の提案のプレビュー中は提案を適用した姿。**写らないもの:** オーバーレイ (ギズモ・選択の輪郭・
  メッシュ編集の頂点・深度の点・光源の取っ手・違いの強調・提案の帯) と HUD。
- 読み戻してから保存のダイアログ (`fileDialogs().saveFile`、フィルタ `.png`) を開くので、写るのは押したときの姿です。
  最初の場所はプロジェクトの隣 (保存していなければドキュメント、なければホーム)、既定の名前は `<プロジェクト名>-<年月日>-<時分秒>.png`。
  拡張子が無ければ `.png` を付けます (保存のダイアログが付ける。[ファイルダイアログ](#ファイルダイアログ))。保存した場所は Toast で知らせ、失敗の理由はビューポートの右上に閉じるまで出します (`Notice`。T0664)。
  読み戻しで GPU が止まった (`RenderError` の `device_lost`。T0465) ときの理由は、エディタのカタログの文 (`viewport.deviceLost`。
  起動し直す案内。読み込みの `device_lost` と同じ) にします (`saveScreenshot.ts` の `screenshotErrorText`。T0940)。
- PNG への変換はフロントエンドの 2D canvas (`toBlob`) で行い、バックエンドの `write_png_file` (`src-tauri/src/screenshot.rs`) が
  書きます。バイト列は本文でそのまま送り、パスはヘッダー `x-ugoe-path` (`encodeURIComponent`) で渡します。絶対パス・拡張子 `.png`・
  中身が PNG (先頭の署名) のものだけを書きます。
- コード: `src/lib/viewport/screenshot.ts` (既定のファイル名。Vitest)、`src/lib/viewport/saveScreenshot.ts` (保存の流れ)。
  画素の検査と PNG への変換 (`encodePng`)・`Screenshot` の型・ヘッダーの名前は、ビューアと共有する `@ugoe/player` の
  `src/screenshot.ts` にあり、変換の失敗の文はそのカタログ (名前空間 `player`) で訳します (T0730)。バックエンドの `write_png_file` は
  ビューアにも同じ形のものがあり、共有しない理由は [apps/viewer/README.md](../apps/viewer/README.md#画像の保存-t0420) に書いています。

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

## 画面の配置

よく使う操作を近くに、まれな操作をメニュー・設定の奥に置きます ([docs/ui.md の「UI の品質の基準」](ui.md#ui-の品質の基準)。T0673 で見直し)。

| 領域 | 内容 |
| --- | --- |
| ヘッダー | 左から「File」のメニュー (新規・PSD から新規・新しいモジュール・開く・見本を開く・保存・別名で保存・PSD の取り込み)、保存、書き出し (モジュールのプロジェクトでは「Export Module…」)、Undo・Redo、プロジェクトの名前と未保存の印 (●)・モジュールの印・取り込みの警告・読み書きしている操作の文。右端にプレビュー・エージェント (つないでいるとき)・設定・ヘルプ。1 行に収め、幅が足りなければプロジェクトの名前を省略する (`FileMenu.svelte`・`App.svelte`) |
| 左 | パーツツリー、デフォーマツリー (ドラッグで親を付け替え) |
| 中央 | ビューポート (メッシュ編集、デフォーマのギズモ、キーフォームの編集)。空のプロジェクトでは中央に次の操作 (PSD から新規・開く・見本を開く) のボタン |
| 右 | インスペクタと物理などのパネル (上。タブで切り替える) とパラメータ一覧 (下。スライダー、キーの追加と削除)。別々にスクロールする。上のタブは 8 つ (Inspector・Physics・Glue・Lighting・Slots・Poses・Verify・AI) で、既定の幅 (320px)・200% の拡大でも 1 行に収まるようアイコンだけにする (T1632。下の「右の列のタブ」)。パネルの見出し (`<h2>`) はタブと同じ名前なので、見た目では隠して読み上げと見出しの移動に残す |
| 下 | タイムライン (モーションと表情。ビューポートの下、畳める。[タイムライン](#タイムライン)) |

- **右の列のタブ (T1632):** 名前の文字を並べると英語で約 470px・日本語で約 500px 要り、既定の幅 (使える幅 296px) で 2 行、200% の拡大
  (列は窓の幅の 36% の 259px) で 3 行になっていたので、アイコン (`lib/ui/PaneIcon.svelte`) だけのタブにしました (`TabStrip` の `icons`)。
  名前は各タブの読み上げの名前 (`aria-label`) と、ポインタを乗せる・キーボードでフォーカスすると出る Tooltip (「名前: 何をするタブか」。
  `app.tabs.tooltip`) で伝えます。AI の提案の数はアイコンの右の数字、見てほしい変化 (新しい提案・作り直しが要るリグ) はアイコンの右上の点と
  強調色で示し、読み上げの名前にも足します (「AI (3) 要確認」)。キーボードは前と同じく並び全体で 1 つのタブ止まり、左右の矢印・Home・End で選びます。
  絵の選び方は [docs/ui.md の「アイコン」](ui.md#アイコン)。
- **大きさの変更:** 左右の列の幅 (左 180〜420px・既定 240px、右 280〜560px・既定 320px)、タイムラインの高さ (160〜800px。既定は窓の高さに
  合わせて 280〜520px。ビューポートを 120px は残す)、右の列の上下の配分 (15〜85%・既定 60%) を、間の区切り (`role="separator"`。
  `src/lib/ui/Splitter.svelte`) で変えます。ポインタのドラッグか、区切りにフォーカスして矢印 (列の間は ← / →、上下の間は ↑ / ↓)・Home (最小)・
  End (最大)。ダブルクリックか Enter で既定に戻します。大きさは表示の状態で、`localStorage` に覚えます (`src/lib/layout.ts`。[覚える値](#覚える値))。
  狭い窓でビューポートが潰れないよう、左右の列は窓の幅の 28%・36% でも抑えます。タイムラインの開閉も覚えます。
- **タイムラインを自動で畳む (T1637):** 空のプロジェクトと、幅 900px 未満の窓 (1440px の窓の 200% の拡大など) では、タイムラインを畳んで
  ビューポート (空の状態の次の操作のボタン・モデル) に高さを渡します。1024px 以上の窓で中身のあるプロジェクトなら、覚えた開閉のままです。
  利用者がこの画面で開閉のボタンを押した後は、押した開閉 (覚えた値) に従います。自動で畳んだことは覚えません (覚えた開閉と列の幅は上書きしない)。
  判定は `src/lib/layout.ts` の `isNarrowWindow`・`timelineOpen` (layout.test.ts)。左右の列は狭い窓でも変えません
  (200% の拡大では右の列が窓の幅の 36% で既に下限より狭く、狭めてもタイムラインの表はほとんど広がらないため。T1637 で比べた)。
- **空の状態:** 空のプロジェクトはビューポートの中央に「何が無いか」と次の操作のボタン、パラメータの無いモデルはパラメータ一覧に
  「パラメータを追加」を出します ([docs/ui.md の「空の状態」](ui.md#デザイン))。
- **ショートカット:** Undo / Redo は Ctrl + Z / Ctrl + Shift + Z / Ctrl + Y (macOS は Cmd + Z / Cmd + Shift + Z)、保存は Ctrl + S、
  別名で保存は Ctrl + Shift + S、開くは Ctrl + O (macOS は Cmd) で、`App.svelte` がウィンドウで受けます。入力欄・選択欄・編集可能な要素の中では、
  その要素自身の取り消しに任せて何もしません (保存は欄を確定してから)。ダイアログ (設定・確認・エージェントの求め) の中でも、後ろの文書を変えないよう
  何もしません。判定は `src/lib/shortcuts.ts` (Vitest)。「File」のメニューの項目にもショートカットを出します。
- **部品と文言:** 画面の部品は `@ugoe/ui`、文言は `@ugoe/i18n` を使います (ADR 0029)。エディタの中の共通の形 (パネルの見出し・行のレイアウト・
  タブの並び・説明付きのボタン) は [docs/ui.md の「エディタの中の共通の形」](ui.md#エディタの中の共通の形) にあります。
- **要素の表記:** 要素を選ぶ選択欄 (ミラーの相手・ミラー元・マスク・テクスチャ) と見出し (インスペクタの先頭・デフォーマの親) は、表示名と ID を `顔 (Layer8)` の形で出します (名前が ID と同じなら ID だけ。テクスチャは使っているメッシュの名前を添える)。`src/lib/labels.ts` (Vitest)。

### 覚える値

表示と設定のうちプロジェクトに入れないものは、WebView の `localStorage` に覚えます。キーはすべて `ugoe.editor.<名前>` の形で、
一覧 (`src/lib/storageKeys.ts` の `STORAGE_KEYS`) から使います (形と重なりは `storageKeys.test.ts` で検査する。T1633)。
読み書きは `@ugoe/i18n` の `readStoredItem` などを通し、使えない localStorage では覚えずに既定値で動きます ([docs/ui.md](ui.md#名前と切り替え))。

| キー | 値 | 既定 | 場所 |
| --- | --- | --- | --- |
| `ugoe.editor.locale` | `system`・`en`・`ja` | `system` | [言語](#言語) (`@ugoe/i18n` の `initLocale`) |
| `ugoe.editor.theme` | `system`・`light`・`dark` | `system` | [言語](#言語)の「General」のテーマ (`@ugoe/ui` の `initTheme`。`index.html` の `<head>` も読む) |
| `ugoe.editor.autoReload` | `true`・`false` | 切 | [外での書き換え](#外での書き換え) (`src/lib/externalChange.ts`) |
| `ugoe.editor.timelineOpen` | `open`・`closed` | 開く | [タイムライン](#タイムライン)の開閉 (`src/lib/panels/timeline/timeline.ts`)。空のプロジェクトと狭い窓で自動で畳んだことは覚えない |
| `ugoe.editor.parameterSectionsClosed` | 閉じた区切りのグループの ID の JSON の配列 (「Other」は `""`) | すべて開く | [パラメータのグループ](#パラメータのグループ)の一覧の区切り (`src/lib/panels/parameters/sections.ts`)。絞り込み中の開閉は覚えない |
| `ugoe.editor.leftWidth` | 左の列の幅 (px) | 240 | 上の「大きさの変更」(`src/lib/layout.ts`) |
| `ugoe.editor.rightWidth` | 右の列の幅 (px) | 320 | 同上 |
| `ugoe.editor.timelineHeight` | タイムラインの高さ (px) | 窓の高さに合わせる (覚えない) | 同上 |
| `ugoe.editor.rightSplit` | 右の列のインスペクタの高さの割合 | 0.6 | 同上 |

### ダイアログ

ダイアログは `@ugoe/ui` の `Dialog`・`AlertDialog` で、次の形にそろえます (T0673)。

| ダイアログ | 決定 (既定のボタン) | ほかのボタン (左から) | Esc・閉じるボタン |
| --- | --- | --- | --- |
| 新しいモジュール (`NewModuleDialog`) | 作る (`primary`。名前の欄の Enter) | 取り消す | 取り消す |
| モジュールの設定 (`ModuleSettingsDialog`) | 適用 (`primary`。名前の欄の Enter) | 取り消す | 取り消す |
| 設定 (`SettingsDialog`) | なし (欄ごとに適用) | 閉じる | 閉じる |
| ライセンス (`LicenseDialog`) | なし | 閉じる | 閉じる |
| 外での書き換え (`ExternalChangeDialog`) | 開き直す (`primary`)。消されたときは「この場所に保存」(`primary`)。保存していない変更があれば `primary` は無く、「捨てて開き直す」も `danger` | 後で (開いたときのフォーカス)・別名で保存・上書き (`danger`) | 後で |
| 本体の変化 (`BaseChangeDialog`) | 読み直す・本体のプロジェクトを選ぶ (`primary`) | 後で (開いたときのフォーカス)・本体のバンドルを選ぶ | 後で |
| 保存していない変更を捨てる (`AlertDialog`) | 捨てて続ける (`danger`) | 取り消す (開いたときのフォーカス) | 取り消す |
| 外部の AI に送る (`ConsentDialog`。`AlertDialog`) | 送る (`primary`) | 取り消す (開いたときのフォーカス) | 取り消す |
| エージェントの求め (`ApprovalDialog`) | 認める (`primary`) | AI タブで見る・断る | 効かない (モーダルでなく、画面の操作をふさがない。答えずに時間が過ぎると断ったことになる) |

- **並び:** 取り消し・「後で」を先に、決定を並びの最後 (右端) に置きます。`primary` は 1 つまで。取り消せない操作は `danger` にし、
  `danger` が決定のときは `primary` を置きません (Enter で取り返しのつかない操作をさせない)。
- **フォーカス:** 開くと中の最初の操作できる部品へ、確かめ (`AlertDialog`) は取り消しへ移ります。利用者の操作でなく不意に出るもの
  (外での書き換え・本体の変化) は「後で」へ移します (`Dialog` の `initialFocus`。打っていた Enter・Space で開き直しや設定の切り替えにならないように)。
  閉じると開いた元へ戻ります。
- **Enter:** 文字の欄が 1 つのダイアログ (新しいモジュール・モジュールの設定) は、欄の Enter で決定します (`<form>` の送信)。
  決定のボタンは操作の並びから `Button` の `type="submit"`・`form` でその `<form>` を送り、Enter と同じ 1 つの道で送ります (`onclick` で別に呼ばない。T1645)。
  決定のボタンが押せない (名前が不正・スロットが無い・送っている途中) あいだは Enter でも送りません。
- **Esc:** いちばん上のダイアログだけを閉じ、表の「Esc」の意味 (取り消す・後で) になります。重なったダイアログ (設定の上のライセンス・
  送る前の確認) は上から 1 つずつ閉じます ([docs/ui.md の「重なるダイアログ」](ui.md#エディタの中の共通の形))。

### 読み込み中と失敗の見せ方

| 状態 | 出す場所と形 |
| --- | --- |
| ファイルの読み書き (開く・保存・書き出し・PSD の取り込み) | ヘッダーのプロジェクトの名前の後ろに「保存しています…」などの文 (`role="status"`。`FileMenuFlow.activity`)。その間はファイルの操作のボタンとメニューを押せない (`busy`) |
| ビューポートの準備・GPU の復元 | ビューポートの右上に「読み込んでいます…」・「GPU の復元を待っています」と、諦めたときの「作り直す」 |
| AI の送信 | 始めた場所 (AI タブの頭か提案のカード) に進み具合と経った秒数・取り消し。確認を待つ間はそのことを出す。分類のキャッシュを戻している間は PSD の行の下に文 ([AI の提案](#ai-の提案)) |
| 結果 (保存した・書き出した・開き直した) | Toast (`showToast`。すぐ消えてよい知らせ) |
| ファイルの操作の失敗 | ヘッダーの下の知らせの帯 (`Notice` の `danger`。閉じられる) |
| 編集・読み直しの失敗 (`EditorDocument.error`) | 右の列の上に `Notice` の `danger` (見出し「モデルを更新できませんでした」・本文・直し方。閉じられる) |
| パネルの中の操作の失敗・欄の誤り | その場所に `Notice` の `danger`、欄の下の文 ([docs/ui.md の「エラー」](ui.md#デザイン)) |

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
- 名前の変更は行の中の `TextField` で、開くと名前の全体を選びます。Enter かフォーカスを外すと確定、Esc でやめます。欄の中の押下は行に渡しません
  (選択・ドラッグにしない)。行の高さ (24px) は変えません (`tree/TreeItem.svelte`)。
- 行が無いときは木 (`role="tree"`) を出さず、何が無いかの 1 文と次の操作のボタンを出します (空のプロジェクトは「PSD から新規…」、
  パーツ・デフォーマの無いモデルは「PSD の取り込み…」。`emptyState.ts` の `treeEmptyState`)。

### 選択とホバーの見せ方

ツリーとビューポートで、同じ要素の状態を対応する形で示します (T0674。状態の見せ方の約束は [docs/ui.md の「デザイン」](ui.md#デザイン))。

| 状態 | ツリーの行 | ビューポート |
| --- | --- | --- |
| 選択 | `accent-soft` の地 | 強調色の輪郭 (メッシュ) |
| 主な選択 (インスペクタに出すもの) | 名前を太字 | 太い輪郭・編集の道具 (ギズモ・フォーム) |
| キーボードの位置 | 強調色の枠 | (無し) |
| ホバー | 地に重ね (`--ugoe-overlay-hover`)。ビューポートでポインタが乗っているメッシュの行も同じ。その行が畳んだ祖先の下にあるときは、いちばん近い見えている祖先の行に地の重ねと左端の強調色の縦線 (3px) | 細い破線の輪郭。ツリーでポインタが乗っている行のメッシュも同じで、パーツ・デフォーマの行ならその中のメッシュすべて (選んでいるものには重ねない) |
| 本体の要素 (モジュールのプロジェクト) | 名前を控えめな色・「本体」の札。選ぶと灰色の破線の枠 | 灰色の破線の輪郭、編集の道具を出さない |
| 非表示 (既定のパラメータで不透明度 0)・マスク専用 | 名前を控えめな色 (マスクは斜体)・「hidden」「mask」の札 | 描かない (マスクはマスクとしてだけ効く) |

- ホバーは `Selection.hover` (表示だけの状態。Undo の対象ではない) に置き、ツリーの行 (`pointerenter`・`pointerleave`) とビューポートの
  選択の層 (`pointermove` でいちばん手前のメッシュ。ボタンを押している間は変えない) が書きます (T1638)。
  - パーツ・デフォーマの行に乗ると、その中のメッシュの輪郭を出します。パーツは子孫のパーツを含めて属するメッシュ、デフォーマは子孫のデフォーマを
    含めて親に持つメッシュです (表示の種類に依らない。パーツの表示ではデフォーマの行に子が無くても、そのデフォーマが動かすメッシュを示す)。
    `panels/tree/build.ts` の `meshesWithin`。
  - 数が多いとき (最上位のパーツなど) も数で打ち切らず、すべての輪郭を 1 本のパス (本体の要素は別の 1 本) にまとめて描きます。メッシュの境界の辺は
    三角形の列ごとに覚えておき、ポインタの移動や再生のたびに数え直さない (描くのは頂点の位置を辿るだけ)。
  - ビューポートで乗ったメッシュの行が畳んだ祖先の下にあると、いちばん近い見えている祖先の行に印を付けます (`nearestVisible`)。祖先は開きません
    (ポインタを動かすたびにツリーが開いて行がずれないように。選べば開く)。
- 警告 (静止姿勢の衝突・はみ出しなど) は、それを調べるパネル (物理・検証) の行と、ビューポートの重ね (`PhysicsOverlay`・`OverflowOverlay`) で示します。
  ツリーの行には出しません。

### インスペクタの並び

インスペクタは、選んだものの種類に依らず、よく触る欄を上に並べます (`panels/InspectorPanel.svelte`。T0674):

1. 見出し (種類と名前)・本体の札・作り直しの知らせ
2. デフォーマの概要と「Unwrap」(デフォーマを選んだとき)
3. キーフォーム (軸とキーの格子・フォームの編集) と推定 (リギングの間にいちばん触る)
4. メッシュの属性 (テクスチャ・合成・カリング・マスク。メッシュを選んだとき。一度決めればあまり触らない)
5. 選んだものをデフォーマで包む (作る操作)
6. 描画用の複製・深度 (まれな設定)

何も選んでいないときは、アバタープロファイルを出します。

## メインのスレッドの外で動くコマンド

Tauri v2 では `async` でない IPC コマンドはメインのスレッド (ウィンドウのイベントのループ) で動き、その間は画面が止まります。
ファイルを読み書きする・大きなバイト列を作る・計算が重い・応答を待つコマンドは `async fn` にし、本体を
`commands::blocking` (アプリの状態を使う) か `commands::off_main` (使わない) で `spawn_blocking` のスレッドへ渡します (`.mcpb` のコマンドは `spawn_blocking` を直接呼ぶ)
(T0685。ビューア T0726・スタジオ T0819 と同じ形)。スレッドが終わらなければ `editor.task_failed`。

| まとまり | コマンド |
| --- | --- |
| プロジェクトのファイル | `open_project`・`reload_project`・`save_project`・`save_project_as`・`new_project_from_psd`・`open_sample_project`・`check_external_change` |
| モジュールのプロジェクト | `open_module_with_base`・`module_open_problem`・`read_module_base`・`new_module_project`・`reload_base`・`check_base_change`・`export_module`・`get_preview_base_bundle`・`get_preview_module` |
| 取り込み・書き出し・画像 | `import_psd`・`export_bundle`・`get_model_bundle`・`generate_texture_normal_map`・`assign_texture_normal_map`・`read_module_file`・`write_png_file`・`write_json_file` |
| ファイルのダイアログ | `open_file_dialog`・`save_file_dialog` (ダイアログを閉じるまで待つ) |
| AI の提案 | `classify_psd_layers`・`propose_rig_from_psd`・`regenerate_rig_proposal`・`propose_normal_maps`・`preview_proposal`・`get_proposal_preview_bundle` |
| 設定・連携 | `ai_settings`・`set_ai_provider`・`store_api_key`・`delete_api_key`・`set_ai_send_externally`・`forget_ai_consent`・`test_ai_connection`・`cancel_ai_requests`・`open_claude_desktop_bundle`・`save_claude_desktop_bundle` |

ほかのコマンド (`apply_command`・`get_model_json`・`undo` など) はロックを取って値を写すだけなので、同期のままメインのスレッドで動きます。

- **ロック:** 錠の順は `document` → `project` のまま。バンドルを作るコマンド (`get_model_bundle`・`get_preview_module`・
  `get_proposal_preview_bundle`・`export_bundle`・`export_module`) は、ロックの中では `Project` の写しを取るだけにし、PNG を含む
  バイト列はロックの外で作ります。`import_psd` と `generate_texture_normal_map` も、モデル (ドキュメント) の写しから編集を作り、
  適用だけをロックの中で行います。作る間にプロジェクトかドキュメントの状態が変わっていたら (つないだエージェントの編集・
  別のプロジェクトを開いたなど) 適用せず `editor.model_changed` を返します (提案の `proposal.model_changed` と同じ確かめ方)。
  開く・保存は、ファイルを読み終えてから (保存は書く間も) ロックを持ちます。
- **呼ぶ順:** メインのスレッドの外のコマンドは、ほかのコマンドと並んで動き、呼んだ順に終わるとは限りません。メインのスレッドの
  コマンド (`apply_command` など) は届いた順に終わってから次が始まるので、`await` した適用の後の読み直し (`get_model_json`・
  `get_model_bundle`) は適用の後の姿を読みます。重なった読み直しは最後に始めたものだけが結果を置き (`EditorDocument.refresh`)、
  提案のプレビューも最後に始めたものだけを出します (`ProposalsView.showPreview`)。編集は `EditorDocument` が列に並べて
  1 つずつ送ります ([パネルからの編集](#パネルからの編集))。フロントエンドで、メインのスレッドの外の
  コマンドの結果に頼る別のコマンドを `await` せずに続けて呼ばないでください。
- **モデルの JSON の解析:** 読み直したモデルの JSON は `EditorDocument.parsedModel` が 1 回だけ解析し、ビューポート・プレビュー・
  パネルはそれを共有して読みます (部品ごとに `JSON.parse(doc.model.json)` しない。共有するので書き換えない)。読めない JSON なら
  null で、部品は何も出さずに止まりません。読み直しで JSON が同じ文字列なら同じオブジェクトのままなので、それに依存する `$effect` は
  関係のない読み直しでは走りません。ローカルの入力の状態を作り直す `$effect` は、それでも ID などの変わらない鍵に依存させます。

## WebView の CSP とコマンドの許可

ページは同梱したものだけで、PSD のレイヤー名・モデルの名前・エージェントの提案の題名と理由は Svelte が文字として出すので、
ページに何かを差し込まれる穴は今は無いつもりです。それでも差し込まれたときに、MCP のトークン (`get_mcp_bridge_token`)・
書き込み (`write_json_file`・`write_png_file`。書けるのは保存のダイアログで選んだパスだけ。[読み書きできるパス](#読み書きできるパス))・API キー (`store_api_key`) などを呼ばれないよう、二重に絞ります
(多層の防御。T0686。ビューア T0727・スタジオ T0820 と同じ形)。

- **CSP** (`src-tauri/tauri.conf.json` の `app.security.csp`): 同梱したものだけを読みます (`default-src 'self'`)。許すのは、
  wasm (`ugoe-wasm` と MediaPipe。`script-src 'wasm-unsafe-eval'`)・Tauri の IPC (`connect-src ipc: http://ipc.localhost`)・
  同梱した MediaPipe のスクリプト・wasm・モデル (`public/mediapipe/`。`'self'`)・blob と data の画像 (法線マップの表示・トラッキングの画像)・
  blob の動画 (`media-src blob:`。カメラの `srcObject` は CSP の対象外) です。`eval` と外への `fetch` は拒まれます
  (AI のプロバイダへの通信はバックエンドが行うので、ページからは要らない。MediaPipe の利用状況の送信も止まる。上の「フェイストラッキング」)。値は 3 つのアプリで同じです (T0024。
  [architecture.md](architecture.md#webview-の-csp-とコマンドの許可))。
  - `index.html` のテーマを先に付けるインラインのスクリプトは、Tauri がビルド時に hash を `script-src` に足すので動きます。
  - Svelte が書く `style` 属性のため `style-src` は `'unsafe-inline'` にし、Tauri が `style-src` に hash を足さないようにします
    (`dangerousDisableAssetCspModification`。hash があると `'unsafe-inline'` が効かなくなる)。
  - 開発時 (`just editor`) は、デスクトップの Tauri は `devUrl` (Vite) のページをそのまま読み、CSP を付けません
    (Tauri が CSP を差し込むのは同梱したページを返すときだけ)。CSP の下での確かめは、同梱したページで動く `just e2e --only editor` で行います。
- **コマンドの許可:** `src-tauri/build.rs` が `src-tauri/src/acl.rs` の `COMMANDS` を `tauri_build::AppManifest::commands` に渡し、
  コマンドごとに `allow-<コマンド>` の許可を作ります (`src-tauri/permissions/autogenerated/`。ルートの `.gitignore` で無視)。アプリの許可の表があると、
  どの capability にも載っていないコマンドはどのウィンドウからも呼べません。今のウィンドウは `main` の 1 つだけなので、
  `capabilities/default.json` (`windows: ["main"]`) がすべてのコマンドを許します。ウィンドウを足すときは、そのウィンドウ用の
  capability に要るコマンドだけを書きます (スタジオの出力ウィンドウと同じ。[docs/studio.md](studio.md))。
- コマンドを足したら `generate_handler!` (`src/lib.rs`)・`COMMANDS` (`src/acl.rs`)・capability の 3 か所に書きます。
  `src/acl.rs` のテストが食い違いと CSP の要の値・3 つのアプリの CSP が同じことを検査します。

## フロントエンドの規約

- Svelte 5 の runes (`$state`、`$derived`、`$effect`) を使います。ストアのライブラリは入れません。
- バックエンドとの通信は `src/lib/ipc.ts` に集約します。コンポーネントから `invoke` を直接呼びません。
- IPC のエラーは `Message` (コード・引数・英語の文) です。`String(e)` で文字列にせず、`src/lib/errors.ts` の `errorText` / `errorCode` で読みます ([エラーと警告のコード](#エラーと警告のコード))。
- IPC の引数・戻り値の型は Rust から生成したもの (`src/lib/generated/types.ts`) を使い、手で書きません。`ipc.ts` が再エクスポートするので、コンポーネントは `./ipc` から import します。
- lint と整形は Biome、型検査は `svelte-check` で行います (`just web-check`)。
- 表示や操作のロジックのうち DOM に依存しないもの (ツリーの組み立て、ドロップの可否など) は素の `.ts` に置き、
  Vitest で単体テストします (`*.test.ts`、`pnpm --filter @ugoe/editor test`。`just web-check` に含まれる)。

## エラーと警告のコード

バックエンド (editor-core と Tauri のコマンド) が画面に渡すエラー・警告・問題は、コードと引数と英語の文の形
(`ugoe_editor_core::Message`) で渡します ([ADR 0029](adr/0029-shared-ui-and-i18n.md) の「Rust 側のメッセージ」。T0666)。

```json
{ "code": "project.base_not_found", "args": { "path": "/work/body.ugoeproj" }, "message": "base model not found: /work/body.ugoeproj" }
```

- `code` は安定した識別子 (小文字の `snake_case` を `.` でつなぐ)。**公開の約束なので名前を変えません。** 意味が変わるなら新しいコードを作ります。
- `args` は文に差し込む値 (文字列か数値)。パス・名前・ID はそのまま渡し、単位や書式は画面で付けます。NaN と無限大は文字列 (`NaN`・`inf`) です。
- `message` は英語の文で、エラーの `Display` と同じです。ログ・MCP・CLI・開発者向けの表示で使い、**文言は前と変えていません** (MCP と CLI は今も文を返す)。
- `cause` (省略できる) は原因になったエラーです (`proposal.rejected` の中の `edit.*`、`edit.invalid_cell` の中の `cell.*` など)。`message` は原因の文も含みます。
- `items` (省略できる) は一覧の中身です。検証のエラーの列 (引数 `count`・`details` を持つコード) で、`details` に並べたのと同じ先頭の 10 件の
  `validation.*` を入れます (T0163)。
- **IPC コマンドの失敗:** Tauri のコマンドは失敗すると `Message` で reject します (`ipc.ts` の `IpcError`)。画面は `src/lib/errors.ts` の `errorText`・`errorCode` で読みます。
- **画面の文 (T0663):** `errorText` (`Message` には `messageText`) は、カタログ (`apps/editor/locales/<言語>.json`) の `errors.<code>` を `args` で引きます。
  カタログに無いコード (新しいバックエンドと古い画面の組み合わせなど) は `message` を出します。値が英語の識別子になる引数 (`kind`・`what`・`expected`・
  `actual`・`target_kind`。`mesh`・`parameter` など) は、`errorArgs.<値>` があれば訳してから差し込みます。`cause` があれば `errorCause` の形
  (「… 原因: …」) で原因の文を後ろに付けます。ただし引数 `detail` を持つコードは、原因の訳を `detail` の代わりに差し込み、後ろには付けません。
  `items` があれば、各項目の訳を `; ` でつないで `details` の代わりに差し込みます (`count` が項目より多ければ `errorListMore` で残りの件数を足す)。`count` を持つコードは複数形のキー (`_one`・`_other`) です。
  **コードを足したら、`en.json` と `ja.json` の `errors` にも足します** (無くても `message` が出るので壊れはしない)。
- **カタログの検査 (T0681):** コードと引数の名前の一覧は、各コードの見本の値 (editor-core は `src/message_catalog.rs`、
  エディタのバックエンドは `src-tauri/src/error.rs` の `message_samples`) から `just gen` で `src/lib/generated/messageCodes.ts` に
  書き出します。見本は本物のエラーの `to_message()` なので、引数の名前は `Coded::args` と一致します (その場でコードと引数を
  書く警告は、テストがソースの引数と照らし合わせる)。ソースに書いたコードに見本が無いと Rust のテスト
  (`every_code_has_a_sample`) が失敗し、エディタの Vitest (`src/lib/messageCodes.test.ts`) は、すべてのコードの文が
  `en.json` と `ja.json` の `errors` にあり、文の引数 (`{{name}}`) がそのコードの引数の名前に含まれることを確かめます。
  カタログは MIT のクレートからは読みません。
- **警告・問題:** `PsdImportWarning`・`RederiveWarning` は `code`・`args`・`message` のフィールドを持ちます。`ModuleProblem` は `Message` のフィールドを平らに含みます (`kind` は直し方を選ぶ分類、`code` は元のエラー)。`AvatarCheck` の `errors`・`warnings`、`.ugoemod` の書き出しの `warnings`、エージェントの接続の状態の `error` は `Message` です。
- **実装:** editor-core のエラーは `ugoe_editor_core::Coded` を実装し (`src/message_codes.rs`)、`?` で `Message` に変わります。Tauri だけで起きるエラーは `src-tauri/src/error.rs` の `EditorError` にまとめます。ほかのクレートのエラー (検証・合成・推定・自動メッシュ・PSD の読み込み・AI のプロバイダなど) を含むものは、その英語の文を `detail` (検証の一覧は `count` と `details`。`details` は先頭の 10 件 (`DESCRIBED_ERRORS_MAX`) までを `; ` でつなぎ、残りは `; and {残り} more` にまとめる) で渡します。
- **元のエラーのコード (T0163・T0167・T1618・T0558・T0571):** 検証 (`ValidationError::code`)・推定 (`InferError::code`・`WorldError::code`)・法線マップ (`NormalMapError::code`)・
  合成 (`ComposeError::code`)・PSD の読み込み (`PsdError::code`)・自動メッシュ (`MeshError::code`) は元のクレートが機械の読むコードを持ち、
  editor-core はそれに領域の名前を付けたコード (`validation.*`・`infer.*`・`world.*`・`normal_map.*`・`compose.*`・`psd_read.*`・`automesh.*`)
  で `cause` (検証の一覧は `items`) に載せます。
  editor-core はこれらのコードを一覧に持たず、元のコードから組み立てます (`Coded::code` は `Cow<'static, str>`)。
  `source_error_codes_follow_the_crate_codes` が、元のクレートのすべてのコードに見本があり組み立てた値と一致することを、
  `every_code_is_documented` が組み立てたコードがどれも下の一覧にあることを確かめます (元のクレートにコードを足したら、一覧と訳と見本にも足す)。合成のコードは SDK の `DressUpError::code` とも同じです。`detail`・`details` の英語の文は互換のために残します (MCP・CLI・古い画面)。
- 自由な文の引数のうち、editor-core が自分で組む理由 (`edit.invalid_physics`・`edit.invalid_motion`・`edit.deformer_edit_failed` などの `reason`)
  と AI のプロバイダの `detail` はまだ英語のままです。PSD の読み込みの理由は種類と値で訳します (`psd_read.*`。T0571) が、
  途中で切れた区画の名前 (`psd_read.truncated` の `section`) とカラーモードの名前 (`psd_read.unsupported_color_mode` の `name`) は
  PSD の形式の用語 (英語) のまま渡します。

### コードの一覧

引数の名前は `args` のキーです。`kind` は要素の種類 (`parameter`・`part`・`deformer`・`mesh` など)、`target`・`element` は ``mesh `EyeL` `` のような表記です。

**編集 (`EditError`。`apply_command` など)**

| コード | 引数 |
| --- | --- |
| `edit.duplicate_id` / `edit.not_found` | `kind`・`id` |
| `edit.invalid_range` | `min`・`default`・`max` |
| `edit.has_children` | `kind`・`id`・`children` (`, ` でつないだ表記) |
| `edit.self_reference` | `kind`・`id`・`field` |
| `edit.deformer_kind_mismatch` | `id`・`expected` |
| `edit.invalid_cell` | `kind`・`id` (原因は `cell.*`) |
| `edit.invalid_texture_name` / `edit.texture_not_png` | `name` |
| `edit.invalid_canvas` | `width`・`height` |
| `edit.cannot_regenerate_mesh` / `edit.mesh_geometry` | `id`・`reason` |
| `edit.automesh_failed` | `id`・`detail` (原因は `automesh.*`) |
| `edit.vertex_out_of_range` | `id`・`index`・`vertices` |
| `edit.too_few_vertices` | `id`・`remaining` |
| `edit.invalid_draw_order` | `id`・`draw_order` |
| `edit.keyform_kind_mismatch` | `kind`・`id`・`expected`・`actual` |
| `edit.invalid_model` | `count`・`details` (一覧の中身 `items` は `validation.*`) |
| `edit.inference_failed` | `target`・`detail` (原因は `infer.*`) |
| `edit.unsupported_inference` / `edit.mirror_pair_not_supported` | `target` |
| `edit.unsupported_interpolation` | `target`・`interpolation` (JSON の名前。`linear`・`smooth`・未知の値は `unknown`) |
| `edit.deformer_edit_failed` / `edit.invalid_physics` / `edit.invalid_avatar_profile` / `edit.module_shape` | `reason` |
| `edit.depth_length` | `target`・`expected`・`actual` |
| `edit.non_finite_depth` | `target`・`value` |
| `edit.depth_point_out_of_range` | `target`・`index`・`points` |
| `edit.invalid_name` | `kind` (`motion`・`expression`)・`name` |
| `edit.invalid_motion` | `motion`・`reason` |
| `edit.invalid_expression` | `expression`・`reason` |
| `edit.draw_clone_shape` | `id`・`original` |
| `edit.not_draw_clone` | `id` |
| `edit.invalid_normal_map` | `texture`・`reason` |
| `edit.invalid_slot` / `edit.invalid_glue` | `id`・`reason` |
| `edit.read_only_base` | `element` |
| `edit.not_module` | なし |
| `module.*` (モジュールのプロジェクトで作業用のモデルを作り直せない。`EditError::Module`) | 中の `ModuleError` のコードと引数 (下の「プロジェクト・モジュール」の表。本体の要素の編集は `edit.read_only_base`、モジュールとして表せないものは `edit.module_shape`、モジュールでないものは `edit.not_module`) |
| `edit.texture_too_large` (テクスチャ・法線マップの追加・差し替えとメッシュの作り直しで、画像の一辺がデコードの上限を超える。`texture` は法線マップならその画像の名前) | `texture`・`width`・`height`・`max` (画素) |
| `edit.conflicting_texture` (`set_mesh_properties` でテクスチャを `texture_name` と非推奨の `texture` の両方で指定した) | `id` (メッシュ) |
| `edit.invalid_layer_correction` (分類の直し `set_layer_correction` が正しくない。[分類の直し](#分類の直し)) | `reason` |

**キーフォームのセル (`CellError`)**

| コード | 引数 |
| --- | --- |
| `cell.missing_axis` / `cell.unknown_axis` | `parameter` |
| `cell.unknown_key` / `cell.duplicate_key` / `cell.non_finite_key` | `parameter`・`value` |
| `cell.out_of_range` | `index`・`len` |
| `cell.keyform_count` / `cell.shape_mismatch` | `expected`・`actual` |

**プロジェクト (`ProjectError`)・モジュール (`ModuleError`)**

| コード | 引数 |
| --- | --- |
| `project.io` / `project.json` | `path`・`detail` |
| `project.format` | `path` (原因は `format.*`) |
| `project.not_regular_file` (プロジェクト・本体の中の読むファイルが通常のファイルでない。[読むファイルの種類と大きさ](#読むファイルの種類と大きさ)) | `path` |
| `project.file_too_large` (読むファイルが上限より大きい) | `path`・`size`・`max` (バイト) |
| `project.not_a_project` / `project.not_empty` / `project.not_png` / `project.base_not_found` / `project.base_is_module` | `path` |
| `project.base_unreadable` | `path` (原因は本体を読んだときの `project.*` など) |
| `project.base_refused` | `path`・`reason` |
| `project.unsupported_version` | `found`・`supported` |
| `project.invalid_name` / `project.duplicate_name` | `kind` (`texture`・`motion`・`expression`)・`name` |
| `project.missing_texture` / `project.unsupported_base_texture` / `project.unsupported_texture` | `name` |
| `project.export_failed` | なし (原因は `format.*`) |
| `project.module_file_unreadable` (比べる `.ugoemod` を読めない。書き出しの失敗と区別する) | なし (原因は `format.*`) |
| `project.missing_module_link` / `project.not_a_module` | なし |
| `project.invalid_module` | `count`・`details` |
| `module.invalid_name` | `name` |
| `module.unknown_slot` | `slot` |
| `module.compose_failed` | `detail` (原因は `compose.*`) |
| `module.collider_needs_base_physics` | `rig`・`collider`・`slot` |
| `module.read_only_base` | `element` |
| `module.shape` | `reason` |
| `module.not_module` (`Document::rebase_module_on`。編集では `edit.not_module`、プロジェクトでは `project.not_a_module` に変わる) | なし |
| `module.draw_order_outside_slot` (書き出しの警告) | `mesh` |

**ファイル形式 (`FormatError`)・書き出し・アトラス・評価**

| コード | 引数 |
| --- | --- |
| `format.not_ugoe_file` / `format.too_large` | なし |
| `format.unsupported_version` | `found`・`supported` |
| `format.invalid_model` / `format.invalid_module` / `format.invalid_physics` | `count`・`details` |
| `format.invalid_motion` / `format.invalid_expression` (モーション・表情の数値が検証を通らない。`.ugoe` と `.ugoeproj` の読み込み) | `name`・`count`・`details` |
| `format.unsupported_features` | `features` |
| `format.unknown_values` | `values` |
| `format.chunk_limit_exceeded` / `format.total_limit_exceeded` (読み込みの上限。1 チャンクの展開後の大きさ / 展開の合計) | `chunk`・`max` (バイト) |
| `format.corrupt` (チャンク・圧縮・テクスチャ・MessagePack・JSON の不正) | `detail` |
| `export.invalid_max_size` / `export.invalid_padding` | `value`・`max` |
| `export.invalid_settings` | `detail` |
| `atlas.missing_texture` / `atlas.not_png` | `name` |
| `atlas.texture_out_of_range` | `mesh`・`texture` |
| `atlas.invalid_max_size` | なし |
| `atlas.texture_too_large` (画像の一辺がデコードの上限を超える) | `name`・`width`・`height`・`max` (画素) |
| `compile.invalid_model` | `count`・`details` |
| `compile.unknown_parameter` | `parameter` |
| `compile.unsupported` | `feature` |

**提案・ライティング・座標・はみ出し**

| コード | 引数 |
| --- | --- |
| `proposal.not_found` | `id` |
| `proposal.stale` / `proposal.nothing_selected` | なし |
| `proposal.invalid_index` | `index` |
| `proposal.rejected` | なし (原因は `edit.*`) |
| `proposal.model_changed` / `proposal.no_psd` / `proposal.not_classified` / `proposal.disappeared` / `proposal.no_preview` | なし |
| `proposal.not_regenerable` | `id` |
| `lighting.texture_not_found` / `lighting.unreadable_texture` | `texture` |
| `lighting.unreadable_normal_map` / `lighting.normal_maps_model_only` / `lighting.no_group_chosen` | なし |
| `lighting.generate_failed` | `detail` (原因は `normal_map.*`) |
| `lighting.invalid_settings` | `detail` |
| `lighting.invalid_preview` | `reason` |
| `lighting.unknown_group` / `lighting.empty_group` | `group` |
| `lighting.image_not_found` | `name` |
| `lighting.texture_too_large` (画像の一辺がデコードの上限を超える) | `texture`・`width`・`height`・`max` (画素) |
| `lighting.normal_map_too_large` | `width`・`height`・`max` (画素) |
| `space.not_found` / `space.no_space` | `kind`・`id` |
| `space.evaluate_failed` | `detail` |
| `space.length_mismatch` | `points`・`guesses` |
| `space.singular` | `index` |
| `space.not_converged` | `index`・`residual` |
| `overflow.unknown_parameter` | `target`・`parameter` |

**PSD の取り込み (エラーと、`PsdImportWarning` の警告)**

| コード | 引数 |
| --- | --- |
| `psd.read_failed` | `detail` (原因は `psd_read.*`) |
| `psd.channel_limit_exceeded` / `psd.total_limit_exceeded` (読み込みの上限。チャンネル 1 枚の展開後の大きさ / 展開の合計。[読み込みの上限](#読み込みの上限)) | `size` (展開しようとした大きさ。合計は超えたところまで)・`max` (上限)。どちらもバイト |
| `psd.read_file_failed` | `path`・`detail` (原因は `psd_read.*`) |
| `psd.invalid_mesh_options` | `option`・`value`・`range` (`automesh.invalid_options` と同じ) |
| `psd.automesh_failed` | `layer`・`detail` (原因は `automesh.*`) |
| `psd.canvas_mismatch` | `psd_width`・`psd_height`・`model_width`・`model_height` |
| `psd.group_blend_mode_ignored` / `psd.blend_mode_unsupported` | `blend_mode` |
| `psd.adjustment_layer` | `key` |
| `psd.unsupported_compression` | `compression` |
| `psd.layer_too_large` (自動メッシュの作業用の格子が大きすぎるので飛ばした) | `width`・`height` (格子のセル数)・`max` (セル数の上限) |
| `psd.layer_texture_too_large` (余白を足した画像の一辺がテクスチャの上限を超えるので飛ばした) | `layer` (レイヤーの名前)・`width`・`height` (余白を含む画像の画素数)・`max` (一辺の上限の画素数) |
| `psd.group_clipping_unsupported` / `psd.empty_layer` / `psd.transparent_layer` / `psd.layer_too_complex` / `psd.clipping_base_missing` / `psd.smart_object_without_pixels` / `psd.layer_effects_ignored` / `psd.layer_mask_ignored` / `psd.missing_pixels` / `psd.flattened_image` | なし (レイヤーは警告の `path`) |

**推定の警告 (`RederiveWarning`)・アバタープロファイルの検査 (`AvatarCheck`)**

| コード | 引数 |
| --- | --- |
| `inference.mirror_failed` | `target`・`source`・`detail` (原因は警告の `cause` の `infer.*`) |
| `avatar.missing_profile` | なし |
| `avatar.newer_profile_version` | `version`・`supported` |
| `avatar.invalid_level` | `level`・`max` |
| `avatar.missing_metadata` | `field` |
| `avatar.missing_parameter` | `level`・`id` |
| `avatar.too_few_extended_parameters` | `required`・`found` |
| `avatar.parameter_range` | `id`・`min`・`default`・`max`・`expected_min`・`expected_default`・`expected_max` |
| `avatar.unknown_reference` | `field`・`target_kind`・`target` |
| `avatar.unknown_tracking_input` | `field`・`kind`・`input` |
| `avatar.missing_tracking_target` / `avatar.unknown_value` | `field` |
| `avatar.duplicate_hit_area_name` | `name` |
| `avatar.invalid_value` | `field`・`value`・`expected` |
| `avatar.internal_tracking_target` | `field`・`id` |

**元のエラー (T0163。原因 `cause` と一覧の中身 `items` に載る)**

`count`・`details` を持つコード (`edit.invalid_model`・`project.invalid_module`・`format.invalid_*`・`compile.invalid_model`・
`compose.invalid_composition`) の `items` は `validation.*` です。数の並び (`keys`・`vertices`) と ID の並び (`cycle`) は `, ` でつないだ文です。

| コード | 引数 |
| --- | --- |
| `validation.duplicate_id` | `kind`・`id` |
| `validation.keyform_count` / `validation.origin_count` / `validation.depth_count` | `kind`・`id`・`expected`・`actual` |
| `validation.keys_not_ascending` | `kind`・`id`・`parameter`・`keys` |
| `validation.empty_keys` / `validation.duplicate_binding` / `validation.unbound_inference_parameter` | `kind`・`id`・`parameter` |
| `validation.indices_not_triangles` | `id`・`len` |
| `validation.index_out_of_range` | `id`・`index`・`vertices` |
| `validation.vertex_count` / `validation.warp_point_count` | `id`・`expected`・`actual` |
| `validation.empty_warp_grid` | `id`・`rows`・`cols` |
| `validation.unknown_reference` | `kind`・`id`・`field`・`target_kind`・`target` |
| `validation.texture_out_of_range` | `id`・`texture`・`textures` |
| `validation.glue_vertex_out_of_range` | `id`・`mesh`・`index`・`vertices` |
| `validation.parent_cycle` | `kind`・`cycle` |
| `validation.mask_cycle` | `cycle` |
| `validation.parameter_range` | `id`・`min`・`default`・`max` |
| `validation.duplicate_pose_part` | `part` |
| `validation.invalid_module_name` | `name` |
| `validation.asymmetric_mirror` | `kind`・`id`・`mirror`・`back` (指し返す先。無ければ無い) |
| `validation.deformer_kind_mismatch` | `kind`・`id`・`field`・`deformer`・`expected` |
| `validation.weight_count` | `rig`・`output`・`expected`・`actual` |
| `validation.parameter_kind_mismatch` | `kind`・`id`・`parameter`・`expected` (`normal`・`blend_shape`) |
| `validation.invalid_blend_shape` | `kind`・`id`・`index`・`reason` (英語の短い文) |
| `validation.invalid_value` | `field`・`value`・`expected` (`finite`・`finite and >= 0` などの条件) |
| `infer.unknown_parameter` / `infer.duplicate_axis` / `infer.mirror_axis_mismatch` / `infer.missing_rotation_axis` | `parameter` |
| `infer.missing_neutral_key` | `parameter`・`default` |
| `infer.non_finite_key` / `infer.missing_mirror_key` | `parameter`・`key` |
| `infer.keys_not_ascending` | `parameter`・`keys` |
| `infer.too_many_axes` | `axes`・`max` |
| `infer.grid_too_large` | なし |
| `infer.grid_size_mismatch` / `infer.origins_length_mismatch` / `infer.depth_length_mismatch` | `expected`・`found` |
| `infer.base_not_authored` | `cell` |
| `infer.shape_mismatch` | `cell`・`expected`・`found` |
| `infer.unknown_target` / `infer.unsupported_target` / `infer.not_depth_rotation` / `infer.missing_depth` / `infer.non_affine_parent` | `target` |
| `infer.unknown_parent` / `infer.unsupported_parent` / `infer.parent_cycle` | `deformer` |
| `infer.not_mirror_pair` / `infer.incompatible_mirror` | `from`・`to` |
| `infer.unmatched_vertices` | `vertices`・`tolerance` |
| `infer.parent_transform` | なし (原因は `world.*`) |
| `world.evaluate` | `detail` |
| `world.length_mismatch` | `points`・`guesses` |
| `world.singular` | `index` |
| `world.not_converged` | `index`・`residual` |
| `normal_map.empty_image` | `width`・`height` |
| `normal_map.size_mismatch` | `width`・`height`・`expected`・`found` |
| `normal_map.invalid_option` | `field`・`value` |
| `normal_map.grid_too_large` | `width`・`height` (升目の数)・`max` |
| `compose.invalid_module_name` / `compose.duplicate_module` | `module` |
| `compose.unknown_slot` / `compose.missing_slot` | `module`・`slot` |
| `compose.slot_occupied` | `slot`・`module` (付けようとしたモジュール)・`attached` (付いていたモジュール) |
| `compose.avatar_level` | `module`・`required`・`actual` (本体のレベル。プロファイルが無ければ無い) |
| `compose.missing_parameter` | `module`・`parameter` |
| `compose.parameter_not_exposed` | `module`・`slot`・`parameter` |
| `compose.unknown_mask` | `module`・`mesh`・`mask` |
| `compose.texture_out_of_range` | `module`・`mesh`・`texture` |
| `compose.unknown_collider` | `module`・`rig`・`collider` |
| `compose.pose_not_exposed` | `module`・`slot`・`part` |
| `compose.invalid_composition` | `count`・`details` |
| `psd_read.not_psd` (先頭の印が PSD でない。以下 `psd_read.*` は `psd.read_failed`・`psd.read_file_failed` の原因) | なし |
| `psd_read.unknown_version` (ヘッダの版が PSD・PSB のどちらでもない) | `version` |
| `psd_read.truncated` (途中で切れている) | `section` (読んでいた区画の名前。PSD の形式の用語で英語) |
| `psd_read.invalid_channel_count` (ヘッダのチャンネル数が 1〜56 の外) | `channels` |
| `psd_read.invalid_image_size` (ヘッダの幅・高さが 1〜30000 の外) | `width`・`height` |
| `psd_read.layer_bounds_out_of_range` (レイヤー・マスクの矩形が負か大きすぎる) | `width`・`height`・`left`・`top` |
| `psd_read.too_many_layer_channels` (レイヤーのチャンネル数が上限を超える) | `channels`・`max` |
| `psd_read.bad_blend_mode_signature` (レイヤーの記録が壊れている) | なし |
| `psd_read.bad_zip_data` (ZIP のチャンネルを展開できない) | なし |
| `psd_read.zip_length_mismatch` (ZIP のチャンネルの展開した大きさがレイヤーと合わない) | `len`・`expected` (バイト) |
| `psd_read.rle_row_too_short` / `psd_read.rle_row_wrong_length` (PackBits の行が短すぎる / 展開した長さが合わない) | なし |
| `psd_read.too_few_composite_channels` (合成画像のチャンネルが RGB に足りない) | `channels` |
| `psd_read.unbalanced_groups` (グループの始まりに対応する終わりがない) | なし |
| `psd_read.unsupported_psb` (大きなドキュメント形式) | なし |
| `psd_read.unsupported_depth` (8・16 ビット以外) | `depth` (ビット数) |
| `psd_read.unsupported_color_mode` (RGB 以外) | `mode` (番号)・`name` (`cmyk` などの名前。知らない番号は `unknown`) |
| `psd_read.unsupported_composite_compression` (仕様にない合成画像の圧縮) | `compression` |
| `psd_read.groups_too_deep` (グループの入れ子が深すぎる) | `max` (段数) |
| `psd_read.limit_exceeded` (読み込みの上限。editor-core は `psd.channel_limit_exceeded`・`psd.total_limit_exceeded` で返すので、ふつうは画面に出ない) | `limit` (`channel_decompressed`・`total_decompressed`)・`size`・`max` (バイト) |
| `automesh.empty` (中身の画素が無い) | なし |
| `automesh.bad_image` (画像の長さが大きさと合わない) | `width`・`height`・`len` (バイト) |
| `automesh.invalid_options` (範囲の外の設定。`MeshError::InvalidOptions`) | `option` (フィールドの名前。`margin`・`edge_spacing`・`interior_spacing`)・`value` (その値。NaN・無限大は文字列)・`range` (受け付ける範囲を区間の記法で書いたもの。`[0, 1024]`・`(0, inf)`・`[0, inf)`。言語によらない) |
| `automesh.too_complex` (頂点数が上限に収まらない) | `max` (頂点数の上限) |
| `automesh.too_large` (作業用の格子が大きすぎる) | `width`・`height` (格子のセル数)・`max` (セル数の上限) |

**エディタのバックエンド (`EditorError`。`src-tauri/src/error.rs`)**

| コード | 引数 |
| --- | --- |
| `editor.lock_poisoned` | `what` (`document`・`project`・`preview cache`) |
| `editor.io` / `editor.create_folder` | `path`・`detail` |
| `editor.internal` / `editor.task_failed` / `editor.invalid_percent_encoding` | `detail` |
| `editor.model_changed` / `editor.empty_path` / `editor.not_saved` / `editor.not_saved_use_save_as` / `editor.external_change` / `editor.not_a_module` / `editor.expected_module_bytes` / `editor.expected_png_bytes` / `editor.not_png_data` | なし |
| `editor.relative_path` / `editor.not_module_file` / `editor.not_png_file` / `editor.not_json_file` / `editor.too_many_sample_folders` | `path` |
| `file.not_chosen` / `file.not_chosen_for_save` (利用者が選んでいないパス。[読み書きできるパス](#読み書きできるパス)。ビューア・スタジオと同じコード) | `path` |
| `dialog.failed` (OS のファイルのダイアログが失敗した) | `reason` |
| `editor.missing_header` | `header` |
| `editor.invalid_header` | `header`・`detail` |
| `editor.invalid_undo_limit` | `min`・`max` (選べる Undo の上限の範囲) |
| `ai.claude_model_missing` / `ai.invalid_base_url` / `ai.model_missing` / `ai.api_key_required` / `ai.plain_http_api_key` / `ai.empty_api_key` / `ai.mixed_destinations` / `ai.send_disabled` / `ai.truncated` / `ai.cancelled` | なし |
| `ai.credential_store` / `ai.provider` / `ai.invalid_proposal` / `ai.missing_api_key` / `ai.invalid_request` / `ai.network` / `ai.invalid_response` | `detail` |
| `ai.api_error` | `status`・`kind`・`detail` |
| `ai.rate_limited` | `retry_after` (秒。切り上げ。分からなければ無い) |
| `ai.refused` | `category` (分からなければ無い) |
| `ai.texture_too_large` (法線マップの提案で、テクスチャの画像の一辺がデコードの上限を超える。読めない画像は `ai.invalid_request`) | `texture`・`width`・`height`・`max` (画素) |
| `ai.template` | `template` (組めなかった自動リギングのテンプレート。`character`・`module`)・`detail` (足りないもの・解けなかったもの。英語) |
| `ai.time_limit` | `minutes` (1 回の提案の時間の上限。分。切り上げ) |
| `ai.invalid_time_limit` | `min`・`max` (選べる範囲。分) |
| `mcp.invalid_port` / `mcp.no_binary` / `mcp.unsupported_platform` / `mcp.not_mcpb_file` | なし |
| `mcp.bridge_start_failed` / `mcp.bundle_write_failed` | `detail` |
| `mcp.open_failed` | `path`・`detail` |

MCP サーバ (`ugoe-mcp`) のツールのエラーにも `mcp.*` のコード (`mcp.invalid_arguments`・`mcp.outside_root` など) があるが、
エージェントが見分けるためのもので画面には出さないので、この表と訳の表には入れない。一覧は
[crates/ugoe-mcp/README.md](../crates/ugoe-mcp/README.md#ツールのエラー) (名前は上の表と重ならないようにする)。

## 型の生成

Rust の型を正として、次のファイルを生成します。**生成物はリポジトリにコミットします** (Rust をビルドしなくてもフロントエンドを検査・ビルドでき、変更の差分がレビューで見えるように)。手で編集しません。

| 生成物 | 元 | 使うところ |
| --- | --- | --- |
| `apps/editor/src/lib/generated/types.ts` | `Command`・`Model`・`Motion`・`Expression`・`PhysicsSettings` (とそれらが参照する型)、IPC の戻り値 (`AppInfo`・`ProjectStatus`・`AnimationData` など) | フロントエンド (`ipc.ts` が再エクスポート) |
| `apps/editor/src/lib/generated/samples.ts` | フィクスチャのモデルと、全バリアントのコマンドを serde で書き出した JSON | 型検査だけ (生成した型と serde の JSON が合うことを `svelte-check` で確かめる) |
| `apps/editor/src/lib/generated/messageCodes.ts` | バックエンドが出すエラー・警告のコード → 引数の名前 (`ugoe_editor_core::message_catalog` と `src-tauri/src/error.rs` の見本) | 翻訳のカタログの検査 (`src/lib/messageCodes.test.ts`。[エラーと警告のコード](#エラーと警告のコード)) |
| `crates/ugoe-editor-core/schema/command.schema.json` | `Command` の JSON Schema (draft 2020-12) | AI のツール定義 (`ugoe_editor_core::codegen::COMMAND_SCHEMA_JSON`、`command_schema()`) |
| `spec/schema/*.schema.json` | `Model`・`Module`・`PhysicsSettings`・`Motion`・`Expression` の JSON Schema (draft 2020-12、`ugoe_model::spec_files`) | 公開仕様 ([spec/schema/README.md](../spec/schema/README.md)。CC BY 4.0) |

- **作り直す:** Rust の型 (`Command`・モデルの型・IPC の戻り値) を変えたら `just gen` を実行し、生成物も同じコミットに含めます。`just gen` は `apps/editor/src-tauri/examples/gen.rs` と `crates/ugoe-model/examples/spec_files.rs` を実行します。
- **検査:** `just test` の `generated_files_are_up_to_date` (src-tauri)・`checked_in_command_schema_is_up_to_date` (ugoe-editor-core)・`checked_in_spec_files_are_up_to_date` (ugoe-model) が、生成物が最新でなければ失敗します。CI はさらに `just gen` の後に `git diff --exit-code` で差分がないことを確かめます。
- **道具:** TypeScript は [ts-rs](https://github.com/Aleph-Alpha/ts-rs)、JSON Schema は [schemars](https://github.com/GREsau/schemars) (どちらも MIT)。どちらも serde の属性 (`tag`・`rename_all`・`default`・`skip_serializing_if` など) を読むので、生成した型は serde の JSON と一致します。
- **ランタイム層:** `ugoe-core` と `ugoe-model` は feature `ts` / `schema` の裏で `TS` / `JsonSchema` を実装します。有効にするのは `ugoe-editor-core` だけで、ランタイム (`ugoe-wasm` など) の依存には入りません。
- **型の対応:**
  - `Id<T>` (`ParamId` など) は `string`。`Vec2` は `[number, number]` (glam の serde 形式。フィールドに `ts(as = "crate::Vec2Repr")` と `schemars(with = ..)` を付ける)。`u64` も `number`。
  - `skip_serializing_if` と `default` を持つフィールドは省略できる (`field?: T`)。`default` だけのフィールドは、書き出す JSON には必ずあるので必須にする。
  - 未知の値 (`Unknown` のバリアント) は書き出せないので、型にもスキーマにも含めない (`ts(skip)` / `schemars(skip)`)。
  - JSON Schema は読み込み側の形なので、`default` を持つフィールドは必須にしない。
- モデルに型を追加したら、`#[cfg_attr(feature = "ts", derive(ts_rs::TS))]` と `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` を付けます (既存の型を参照)。
