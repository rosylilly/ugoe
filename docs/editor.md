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
| パーツ | `add_part`、`remove_part`、`rename_part`、`set_part_parent`、`set_part_keyforms` |
| デフォーマ | `add_deformer`、`remove_deformer`、`rename_deformer`、`set_deformer_parent`、`set_deformer_part`、`set_warp_keyforms` (`rows` / `cols` は変えない)、`set_rotation_keyforms`、`create_warp_deformer`・`create_rotation_deformer`・`reparent_preserving_look`・`move_rotation_origin`・`unwrap_deformer` ([デフォーマの編集](#デフォーマの編集)) |
| メッシュ | `add_mesh` (`meshes` の末尾に追加)、`remove_mesh`、`rename_mesh`、`set_mesh_properties`、`set_mesh_parent`、`set_mesh_keyforms`、`set_mesh_keyform`、`reorder_mesh`、`regenerate_mesh` ([メッシュの再生成](#メッシュの再生成))、`move_mesh_vertices`・`add_mesh_vertex`・`remove_mesh_vertices`・`retriangulate_mesh` ([メッシュの編集](#メッシュの編集))、`create_draw_clone`・`sync_draw_clone` ([描画用の複製](#描画用の複製)) |
| キーフォーム | `add_keyform_key`、`remove_keyform_key`、`set_keyform_cell`、`set_mesh_draw_order`、`set_keyform_interpolation` ([キーフォームの編集](#キーフォームの編集)) |
| テクスチャ | `add_texture`、`set_texture_image`、`remove_texture`、`set_normal_map` ([法線マップ](#法線マップ)) |
| ライティング | `set_lighting` (`Model::lighting` を変える。[法線マップ](#法線マップ)) |
| エフェクト | `set_effects` (`Model::effects` を丸ごと置き換える。[エフェクトの設定](#エフェクトの設定)) |
| キャンバス | `set_canvas` (`width`・`height`・`origin`。大きさは 0 より大きい有限の値。要素の座標は変えない) |
| キーフォームの推定 | `infer_keyforms`、`mirror_keyforms`、`mark_keyform_authored`、`mark_keyform_derived`、`set_keyform_inference`、`set_mirror_pair` (下の「推定とミラー」)、`set_depth`・`apply_depth_preset` ([深度の編集](#深度の編集)) |
| モーション・表情 | `add_motion`、`remove_motion`、`rename_motion`、`set_motion_properties`、`add_motion_curve`、`remove_motion_curve`、`set_motion_curve_fade`、`set_motion_key`、`move_motion_key`、`remove_motion_key`、`set_motion_segment`、`set_motion_events`、`add_expression`、`remove_expression`、`rename_expression`、`set_expression_properties`、`set_expression_parameters` ([モーションと表情の編集](#モーションと表情の編集)) |
| 物理 | `set_physics_world`、`add_physics_rig`、`create_physics_rig`、`remove_physics_rig`、`rename_physics_rig`、`set_physics_inputs`、`set_physics_outputs`、`set_physics_particles`、`set_physics_normalization`、`set_physics_rig_colliders`、`set_physics_frame`、`add_physics_collider`、`remove_physics_collider`、`set_physics_collider`、`reorder_physics_rig`、`apply_physics_template` ([物理の編集](#物理の編集)・[物理のテンプレート](#物理のテンプレート)) |
| その他 | `batch` |

- `set_mesh_properties` は `part`・`texture`・`blend`・`culling`・`masks`・`mask_mode`・`mask_only` のうち、指定したフィールドだけを変えます。
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

- 推定のエラー (中立のキーがない、基準のセルが `derived`、ミラーの対でない、など) は `EditError::Inference` で拒否し、モデルは変えません。
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
| 通知 (`inspector/inference/RederiveNotice.svelte`) | 適用したコマンドが返した `RederiveWarning` を、インスペクタの上に「Dismiss」で閉じるまで出す (同じ格子とミラー元の組は新しいもので置き換える) |

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

### 削除の連鎖と拒否

| コマンド | 他の要素が参照しているとき |
| --- | --- |
| `remove_parameter` | **連鎖して直す。** このパラメータを軸に持つキーフォーム格子 (パーツ・デフォーマ・メッシュ・グルー) から軸を取り除き、パラメータの `default` と等しいキーの断面を残す。等しいキーがなければ最も近いキー (等距離なら小さい方) の断面を残す。セルの由来 (`origins`) も同じセルを残す。深度からの回転の `yaw` / `pitch` がこのパラメータなら外す (回転の中心などの設定は残す)。他のパラメータの `mirror` は `null` にし、エフェクト (目パチ・呼吸・リップシンク) の対象からは取り除く |
| `remove_part` | 子パーツ・所属するデフォーマ・所属するメッシュが残っていれば **拒否する** (`EditError::HasChildren` に子の一覧が入る)。ポーズからは取り除く: そのパーツの要素を消し、他の要素の `links` からも外し、空になったグループは消す |
| `remove_deformer` | 子デフォーマ・子メッシュが残っていれば **拒否する**。他のデフォーマの `mirror` からは外す |
| `remove_mesh` | **連鎖して直す。** 他のメッシュの `masks` と `mirror` から外し、このメッシュを使うグルーと当たり判定を削除する (その当たり判定を指すアバタープロファイルの標準の当たり判定も外す)。描画用の複製の元を消すと、複製はつなぐグルーを失って普通のメッシュ (基準の形) として残る |

- 削除 (`remove_part`・`remove_deformer`・`remove_mesh`) で要素がなくなったら、その要素をミラー元 (`keyforms.mirror_source`) にしている格子から記録を外し、その格子の `mirror` のセルを `authored` にします (値は変えない)。

- 拒否する場合は、先に子を削除するか付け替えます。まとめて行うなら `batch` を使います。
- 物理はドキュメントが持つので、パラメータ・デフォーマ・メッシュを消すと参照を外します ([物理の編集](#物理の編集))。モーション・表情は `Model` の外のデータなので、`remove_parameter` はこれらを書き換えません。消したパラメータへの参照が残ることがあります (ランタイムは無視する。タイムラインは `(missing)` の行として出す)。

### Undo

最初はスナップショット方式 (`Model` を丸ごと clone) で実装します。簡単で確実だからです。
モデルが大きくなってメモリや時間が問題になったら、各コマンドが逆操作を返す方式に移行します ([ADR 0005](adr/0005-editor-command-model.md))。

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

画面はビューポートの下のパネル (`src/lib/panels/timeline/`) です。「Motions」「Expressions」のタブを持ち、畳める (`localStorage`)。操作はすべて上のコマンドを送るだけで、表示の状態 (選んだモーション・再生位置・再生中・選んだ行とキー・表情のプレビュー) は `TimelineState` (`timeline.svelte.ts`) にあり、Undo の対象ではありません。

| 場所 | 操作 |
| --- | --- |
| 上の段 | モーションの選択・「New motion」(`add_motion`、長さ 2 秒)・「Delete」・名前 (`rename_motion`)・長さ・「Loop」・フェード (`set_motion_properties`)・「Events」(`set_motion_events`) |
| 再生の段 | 「Play」/「Stop」・「Repeat」(最後で折り返す。モーションの `looping` でも折り返す)・「◀」「▶」(1 フレーム)・時刻の欄・「◆ Key changed」・「Delete key」 |
| 行 (ドープシート) | モデルのすべてのパラメータと、パラメータ以外のカーブ (パーツの不透明度・エフェクトの強さ。モデルにないパラメータのカーブは `(missing)`)。行の「◆」は今のスライダーの値でキーを打つ (`set_motion_key`)。ひし形はキーで、押すと選んで再生位置をその時刻にし、左右にドラッグすると時刻を変える (`move_motion_key`、1 フレームに揃える)。目盛りの押す・ドラッグでシーク (Shift で揃えない) |
| カーブのグラフ (`CurveGraph.svelte`) | 選んだ行のカーブ。ベジェは (時刻, 値) の平面の 3 次ベジェなので SVG の `C` で描く。キーの円のドラッグ (`move_motion_key`)、取っ手の四角のドラッグ (`set_motion_segment` の `c1` / `c2`、区間の中に収める)、ダブルクリックでキー、選んだキーの時刻・値の欄、入る区間の種類のボタン、カーブのフェード (`set_motion_curve_fade`)、「Delete curve」 |
| キーボード | パネルの中 (入力欄の外) で Space: 再生・停止、← / →: 1 フレーム、Delete / Backspace: 選んだキーを消す |

- **「◆ Key changed」:** スライダーの値が、その時刻のカーブの値 (カーブがなければ既定値) と違うパラメータすべてに、今の値でキーを打つ (値域の 1e-4 倍までは同じとみなす)。複数なら 1 つの `batch` (Undo の 1 単位)。
- **フレーム:** キーの時刻は 30 fps のフレームに揃えます (`snapToFrame`)。時刻の欄とキーの欄に数を入れれば、揃えない時刻も使えます。
- **ドラッグ中** は表示だけを変え (ドープシートのひし形・グラフの曲線)、離したときに 1 つのコマンドを送ります。

**再生とシーク:** `TimelineState` は、選んだモーションの各カーブをその時刻で評価した値 (wasm の `sample_motion_json(json, t)`、`ugoe_motion::curve::evaluate` そのもの。フェードなし) を、スライダーと同じ `ViewportController.setParameter` で書き込みます。再生は `requestAnimationFrame` の経過時間で再生位置を進めます (`advance`)。

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
| 上の段 | 名前 (`rename_expression`)・フェード (`set_expression_properties`)・「Delete」 |

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
| `add_physics_collider { collider }`・`set_physics_collider { collider }`・`remove_physics_collider { id }` | 衝突体の追加・置き換え (`collider.id` のもの)・削除 (リグの `colliders` からも外す) |

- **検証:** `Document::apply` は、適用後の物理を `validate_physics` で検査し、適用前になかったエラーが生じたら `EditError::Invalid` で拒否します (存在しない参照先・種類の違うデフォーマ・頂点の重みの数・負の半径など。モデルの検証と同じ規則)。
  `validate_physics` が見ない値はコマンドが検査し、`EditError::InvalidPhysics` で拒否します: 有限でない値、`fps` が 0 以下、粒子が 0 個、`mobility` が 0〜1 の外、`delay`・`radius`・`collision_radius` が負、出力の `particle` が 0 (根元)、出力の `blend` が未知の値、正規化の範囲が `min <= default <= max` でない、空の ID。
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
  トラッキングが体の回転を頭の回転 × 0.3 にしている ([avatar-profile.md#頭の向き](spec/avatar-profile.md#頭の向き)) と、体の追従の結果は振り子の角度 × 0.3 になり、`overwrite` と同じ動きです。体の回転を書かないホストでは `overwrite` を選びます。中継パラメータへの出力は常に `overwrite` です。
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
| リグの編集 (`panels/physics/RigEditor.svelte`) | 名前、「Gravity frame」、「Delete rig」、粒子 (「Length」`radius`・「Sway」`mobility`・「Speed」`delay`・「Settle」`acceleration`・「Hit r」`collision_radius`・位置。「+ Particle」は最後の区間と同じ向き・長さで足す)、入力、出力 (種類・書き込み先・粒子・成分・倍率・重み・反転・パラメータへの出力の合成の方法「Overwrite」/「Add」([physics.md#合成の方法](spec/physics.md#合成の方法))。頂点の出力は作ったとき重みがすべて 1)、「Collides with」、「Input range」。欄を確定するたびにコマンドを 1 つ送る |
| 衝突体 (`panels/physics/ColliderSection.svelte`) | 「+ Circle」「+ Capsule」(主な選択がデフォーマならその原点 (ワープは中心) に取り付け、なければキャンバスの中央)。名前・「Attach to」・「Shape」(どちらも今の見た目の位置を保って写し直す)・位置・半径・削除 |
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
- トラッキングの上書き・視線・スロット・当たり判定そのもの (`set_hit_areas`) は、今は画面では編集しません (コマンドは使えるので、
  MCP サーバや CLI の見本のジェネレータからは設定できる)。

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
- **モーション・表情:** ドキュメントのもの (`EditorDocument.motions` / `expressions`、保存前のものも) を JSON で
  `start_motion_json(json, "force")`・`set_expression_json` に渡します。モーションはフェード・ループ・イベント込みで
  再生し (発火したイベントを最近の 5 つまで出す)、「Stop」は `stop_motions` (フェードアウト)。表情は 1 つだけ選び、
  「(none)」で `stop_expressions`。
- **物理の設定:** バンドル (`get_model_bundle`) にはドキュメントの物理が入っています。バンドルが作れず JSON で表示している
  ときは物理が無いので、`WebPuppet` に物理が無ければ `set_physics_json` でドキュメントの物理を渡します。
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
  - MediaPipe の wasm とモデルは、`pnpm dev` / `pnpm build` の前に `ugoe-mediapipe-assets public/mediapipe`
    (パッケージの `scripts/mediapipe-assets.mjs`) が用意します。wasm は npm パッケージから写し、モデル
    (`face_landmarker.task`) は版を固定した URL から取って SHA-256 を検証します。`public/mediapipe/` はコミットしません。
    同じ場所にライセンスの表記 (`third-party-notices.txt`、Apache-2.0) を写し、パネルの「Third-party licenses」で表示します。
  - Tauri 側 (`src-tauri/src/tracking.rs`): webview からのカメラの許可の要求だけを許し (`on_permission_request`)、
    記録の保存は `write_json_file` (保存のダイアログで選んだ `.json` だけ)。macOS は `Info.plist` の
    `NSCameraUsageDescription` と `Entitlements.plist` の `com.apple.security.device.camera`。
  - ヘッドレスのブラウザでは、記録した値の列 (JSON) か画像・動画を「File…」で入れて確かめます
    (`window.__UGOE_TRACKER__` でトラッカーを読める)。
- **まだ無いもの:** 目パチ・呼吸・リップシンクの設定 (`Model.effects`) を編集する画面はまだ無い
  (読み込んだモデルの設定で動く)。コマンドは `set_effects` ([エフェクトの設定](#エフェクトの設定))。

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
| パラメータ一覧 (`ParametersPanel.svelte`) | 「+」でパラメータを作る (ID・名前・最小・既定・最大)。名前を押すと名前・値域・既定値の編集と削除 (`rename_parameter` と `set_parameter_range` を `batch` で 1 つの Undo に)、左右の対の「Mirror」(なし / 自分自身 / 相手。選ぶとすぐ `set_parameter_mirror`。標準の命名から対を提案する。`panels/parameters/`)、「Relay (internal)」(切り替えるとすぐ `set_parameter_internal`)。中継用のパラメータは一覧に残し、名前を斜体にして「relay」の印を付ける。スライダーの下に、主な選択の要素のキーを ◆ で示し (押すとそのキーへ)、「Snap to keys」で要素のすべての軸を最も近いキーへ動かす。行の値の欄に数を入力して Enter (またはフォーカスを外す) でスライダーをその値にする (数でない・値域の外なら理由を出して拒否)。行の ◆ ボタンで、今の値にキーを追加 (今の値がキーならそのキーを削除。キーの上とみなす許容差の中に別のキーがあれば拒否)。スライダーは表示の状態なので IPC を通らない |
| インスペクタ (`inspector/keyforms/KeyformInspector.svelte`) | 主な選択の要素の軸とキー (× で削除、押すとそのキーへ移動、「+ 値」で今の値にキーを追加)、軸のないパラメータへの軸の追加、セルの格子 (1 軸は 1 行、2 軸は表、3 軸目以降は今の値に最も近いキーの断面)。メッシュでは描画順を表示し、「Set for all cells」で `set_mesh_draw_order` を送る。メッシュ・ワープ・回転では「Smooth curve between keys」のチェックで `set_keyform_interpolation` (`smooth` / `linear`) を送る (未知の値なら線形として扱うことを添える。ブレンドシェイプ格子の切り替えはまだ画面にない)。セルは記号と色で由来を示し ([推定の画面での操作](#推定の画面での操作))、今の値がすべての軸でキーの上にあればそのセルを枠で示す。キーの上になければ「Move to nearest key」を出す |
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
- 編集は [フォーム編集](#キーフォームの編集) と同じく **今のパラメータ値のセル** に `set_keyform_cell` で書き込み (離したときに 1 つ、Undo の 1 単位)、キーの上にないときは編集を拒否して「Move to nearest key」を出します。制御点と原点は `canvas_to_local` で親の座標系に直します。角度は画面上の回転角を足します (親や自分の反転で向きが裏返っていれば符号を変える)。拡大率は正の値に限ります。
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
- 法線マップの生成 (アルファの形・深度推定から) は `ugoe_ai::normal` が `set_normal_map` と `set_lighting` の列を提案します ([ai-integration.md#法線マップの生成](ai-integration.md#法線マップの生成))。エディタの画面からの呼び出しは T0637 (AI の提案のプレビューと採用) で扱います。

## エフェクトの設定

- `set_effects` は目パチ・呼吸・リップシンクの設定 (`Model::effects`、[motion.md#エフェクト](spec/motion.md)) を丸ごと置き換えます。
  対象のパラメータがモデルにあること・時間の値の範囲は、適用後の検証 (`validate`) が調べ、通らなければ拒否してモデルを変えません。Undo で前の設定に戻ります。
- パラメータを削除すると、エフェクトの対象からも取り除きます (`remove_parameter`)。
- 自動リギングの提案 (`ugoe_ai::rig`) は、作ったパラメータに向けた標準の設定をこのコマンドで足します ([ai-integration.md#テンプレート-ugoe_airigpropose_rig](ai-integration.md#テンプレート-ugoe_airigpropose_rig))。
- 設定を編集する画面は後続 (今は提案か、読み込んだモデルの設定で動く)。

## プロジェクト

- 保存形式は [`.ugoeproj/`](spec/file-formats.md#プロジェクト-ugoeproj) です。`ugoe_editor_core::Project` がプロジェクト 1 つ分の中身 (`ProjectMeta`・モデル・テクスチャ・物理・モーション・表情) を持ち、`save(dir)` / `load(dir)` で読み書きし、`export_bundle(&Project)` で `.ugoe` のバイト列を作ります。
- 編集中は、モデル・テクスチャ・物理・モーション・表情を `Document` が持ちます (`Project::document()` で作り、`Project::from_document` で保存用に戻す。モーションと表情は T0630、物理は T0629 で移した)。
- 物理の設定は Tauri の `get_physics` (`PhysicsSettings | null`) で読みます。ビューポートは `get_model_bundle` のバンドルに入った物理を使います。
- **未保存の変更:** `Document` は状態ごとに番号を振り、保存した状態の番号を覚えます。`is_modified()` は現在の状態が保存した状態と違うかを返すので、Undo で保存時の状態に戻ると「変更なし」に戻ります。失敗したコマンドは状態を変えません。

### Tauri コマンド

| コマンド | 内容 |
| --- | --- |
| `project_status` | 名前・保存先・未保存の変更の有無 (`ProjectStatus`) を返す |
| `get_animations` | ドキュメントのモーションと表情 (`AnimationData { motions, expressions }`) を返す。タイムラインの表示用 (編集は `apply_command`)。`src-tauri/src/commands.rs` |
| `new_project` | 空のモデル (1024×1024 のキャンバス) の新しいプロジェクトにする |
| `new_project_from_psd(path, mesh_options?)` | 空のプロジェクトを作って PSD を取り込み (キャンバスは PSD の大きさ、警告は `import_psd` と同じ)、それを今のプロジェクトにする。名前はファイル名 (拡張子を除く)、保存先はなし。取り込みは Undo の 1 単位で、未保存の状態から始まる (Undo すると空のプロジェクトに戻る)。失敗したら今のプロジェクトはそのまま |
| `open_project(path)` | `.ugoeproj` ディレクトリを開く。失敗したら今のプロジェクトはそのまま |
| `save_project(overwrite?)` | 開いた (前回保存した) 場所に保存する。保存先がなければエラー。開いた後で外で書き換えられていたら、`overwrite` が真でない限り書かずにエラー ([外での書き換え](#外での書き換え)) |
| `reload_project` | 開いているプロジェクトを保存先から開き直す。未保存の変更は破棄する (確認はフロントエンドで行う) |
| `check_external_change` | 開いた (最後に保存した) 後で保存先のファイルが外で変わったか (`ExternalChange \| null`) |
| `save_project_as(path)` | 別の場所に保存し、以後の保存先にする。拡張子 `.ugoeproj` がなければ付け、名前をディレクトリ名にする |
| `get_export_settings` / `set_export_settings(settings)` | 書き出しの設定 (`ExportSettings`) を読む / プロジェクトに残す ([書き出し](#書き出し))。範囲の外はエラー |
| `export_bundle(path)` | プロジェクトの書き出しの設定で `.ugoe` ファイルに書き出し (アトラス化は任意)、`ExportReport` を返す。保存状態は変えない |
| `import_psd(path, mesh_options?)` | PSD ファイルを今のドキュメントに取り込み ([PSD の取り込み](#psd-の取り込み)、Undo の 1 単位)、警告 (`PsdImportWarning[]`) を返す。最上位のパーツの名前はファイル名。`src-tauri/src/psd.rs` |

- パスの引数は **絶対パスだけ** を受け付けます (相対パスは `path must be absolute: …` のエラー)。相対パスはプロセスの作業ディレクトリ
  (開発時は `apps/editor/src-tauri`) を基準にしてしまい、思わぬ場所に保存されるためです (T0619)。パスは [ファイルダイアログ](#ファイルダイアログ) で選びます。
- ファイルメニューは「New」「New from PSD…」「Open…」「Save」「Save As…」「Export…」「Import PSD…」です。
- 未保存の変更がある状態で「New」「New from PSD…」「Open…」を押すと、もう一度押すまで実行しません (確認のダイアログの代わり)。
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

### 外での書き換え

`ugoe-mcp` (ファイルを直接扱う形)・別のエディタ・手での編集が、開いているプロジェクトを書き換えたことに気づき、黙って上書きしない
([ADR 0023](adr/0023-editor-mcp-bridge.md) の 5、T0641)。操作は [マニュアル](manual/screen.md#外でプロジェクトが書き換えられたとき)。

- **検出の方法: ファイルの指紋を比べる (ファイルの監視は使わない)。** `ugoe_editor_core::ProjectFingerprint` が、`Project::load` の読むファイル
  (`project.json`・`model.json`・`physics.json`・`motions/`・`expressions/`・`textures/*.png`) の中身のハッシュと大きさを持つ。
  OS のファイルの通知 (`notify` クレートなど) は、WSL 越しのパス (`\\wsl.localhost\...`、9P) やネットワークのドライブ (SMB) では届かないことが多く、
  エディタを Windows で動かし、エージェントを WSL で動かす使い方 (ADR 0023 の 4) で働かない。指紋を比べる方式はどのファイルシステムでも同じに働き、
  依存も増えない。代わりに、確かめた時点でしか気づかない。
- **いつ確かめるか:** ウィンドウに戻ったとき (`focus`)、ウィンドウが見えている間は 5 秒ごと (`CHECK_INTERVAL_MS`)、保存の前。
  バックエンドの `save_project` も書く直前に確かめ、変わっていれば `overwrite` なしでは書かない (画面の確認とのあいだの書き換えも拾う)。
- **比べ方:** 開いた (保存した) ときの指紋を `ProjectSession` が覚え、`ProjectFingerprint::reread` で取り直して比べる。中身が同じなら更新日時が変わっても
  変更としない (`touch`・同じ内容の書き直し)。JSON は毎回読んでハッシュし、PNG は大きさと更新日時が前と同じなら前のハッシュを使い回す
  (大きな画像を 5 秒ごとに読まないため。大きさも日時も同じまま PNG の中身だけを変えられると気づかない)。ハッシュは `std` の `DefaultHasher` で、ファイルには残さない。
- **自分の保存を外の変更と取り違えない:** 保存の直後に指紋を取り直す (`ProjectSession::save_to`)。確かめる処理は保存と同じロックを持って行う。
  開くときの指紋は読み込みの **前** に取る (`Project::load_tracked`)。読み込みの途中で書き換えられたら、次に確かめたときに変更として見える (見逃すより開き直しを促す)。
- **応じ方 (`src/lib/externalChange.ts`、Vitest):** 変わっていれば `ExternalChangeDialog` を出す。未保存の変更がなければ「Reload」・「Later」、
  未保存の変更がある (または保存しようとした) ときは「Reload and discard my changes」・「Overwrite with my version」・「Save As…」・「Later」、
  `project.json` がなければ (移された・消された)「Save here again」・「Save As…」・「Later」。「Later」を選んだ外の状態 (`ExternalChange.token`) は、
  間隔の確認では二度尋ねない (保存の前は尋ねる)。「Reload automatically when there are no unsaved changes」(localStorage に覚える、既定は切) を入れると、
  未保存の変更がないときは尋ねずに開き直す。
- **開き直し (`reload_project`):** 保存先から `Project::load_tracked` で読み直す。AI への送信の「今後は確かめない」は同じプロジェクトなので残し、
  AI の提案は前のドキュメントに対するものなので消す。ビューポートの表示の位置とパラメータの値は保つ (新規・開くと違い Fit しない)。

### ファイルダイアログ

ファイルメニューのパスは OS のファイルダイアログ (`tauri-plugin-dialog` / `@tauri-apps/plugin-dialog`、MIT OR Apache-2.0) で選びます。
パスの入力欄はありません (ダイアログが使えない環境の予備も置かない。ヘッドレスの確認では下の差し替えを使う)。
取り込みの警告はメニューの横に一覧で、保存・書き出しの結果 (書いた場所の絶対パス) はメニューの横に出します。

| 操作 | ダイアログ | フィルタ | 既定 |
| --- | --- | --- | --- |
| New from PSD… / Import PSD… | ファイルを開く | `.psd` | 最初の場所 |
| Open… | **フォルダーを選ぶ** (`.ugoeproj` はディレクトリ) | なし | 最初の場所 |
| Save As… (保存先のない Save も) | 保存 (新しいフォルダーを作れる) | `.ugoeproj` | 最初の場所に `<プロジェクト名>.ugoeproj` |
| Export… の欄の「Export .ugoe…」 | 保存 | `.ugoe` | 最初の場所に `<プロジェクト名>.ugoe` |

- **最初の場所:** 最後にダイアログで選んで成功した場所 → 開いているプロジェクトの隣 → ドキュメント (`documentDir()`、なければホーム) の順。
  プロジェクト名のうちファイル名に使えない文字は `_` にする。
- 保存のダイアログで選んだ名前に拡張子 (`.ugoeproj` / `.ugoe`) がなければ付け足す (`save_project_as` も `.ugoeproj` を付ける)。
- ダイアログを閉じた (取り消した) ときは何もしない。ダイアログを開いている間はメニューのボタンを押せない。
- 権限 (`capabilities/default.json`) は `dialog:allow-open` と `dialog:allow-save` だけ (メッセージ・確認のダイアログは使わないので入れない)。
  既定の場所を得る `@tauri-apps/api/path` は `core:default` に含まれる。
- コード: ダイアログの呼び出しは `src/lib/fileDialogs.ts` (`FileDialogs`)、フィルタ・拡張子・パスの整形は `src/lib/filePaths.ts` (Vitest)。
- **差し替え (テスト用):** `window.__UGOE_FILE_DIALOGS__` に `FileDialogs` (`openFile`・`openDirectory`・`saveFile`・`defaultDirectory`) を置くと、
  ダイアログの代わりにそれを呼ぶ。ヘッドレスのブラウザでは、IPC の差し替え (`window.__TAURI_INTERNALS__`) と一緒に固定のパスを返すものを置く。

### 書き出し

「Export…」は書き出しの欄を開きます。欄で設定を選び、「Export .ugoe…」で保存のダイアログを開いて書き出します (`src/lib/ExportPanel.svelte`)。

- **設定 (`ugoe_editor_core::ExportSettings`):** テクスチャのアトラス化の有無 (既定はしない)、アトラス 1 枚の最大の大きさ
  (512〜16384px から選ぶ。既定 2048)、余白 (端の画素の複製の幅、0〜64px。既定 2)。規則は
  [file-formats.md#テクスチャのアトラス化](spec/file-formats.md#テクスチャのアトラス化)。
- 設定は変えるたびにプロジェクトに残し (`set_export_settings`)、保存で `project.json` の `editor.export` に書きます
  ([file-formats.md#projectjson](spec/file-formats.md#projectjson))。すべて既定値なら書きません。設定を変えると未保存の印が付きます
  (`ProjectSession.meta_modified`。Undo の対象ではない。モデルを変えないので `Command` にしない)。
- **結果:** 書き出した後、欄の下にテクスチャの枚数・描画の状態の切り替え回数・描画の回数の前 → 後と、アトラス化の詳細
  (詰めた枚数とアトラスの大きさ、同じ画像として 1 つにしたもの、まとめずに残したものと理由、使われないので取り除いたもの) を出します
  (`src/lib/exportSummary.ts`、Vitest)。別のプロジェクトに替わると欄を閉じて結果を消します。
- **切り替え回数の見積もり (`ugoe_editor_core::export::estimate_draws`):** エディタのバックエンドは `ugoe-render` (GPU) に依存しないので、
  参照レンダラの本描画のまとめ方 ([rendering.md#描画の最適化](spec/rendering.md#描画の最適化)) を CPU で真似ます。既定のパラメータで
  描画順に並べた、表示されていて三角形を持つメッシュについて、(テクスチャ・合成モード・カリング) が前と変わる回数を数えます。
  マスクアトラスの違いは数えないので、マスクを多く使うモデルでは `ugoe render --stats` の `state_changes` より少なく出ることがあります
  (`character.psd` では前後とも `ugoe render --stats` と一致した: 12 枚 → 1 枚、切り替え 10 → 2、描画 11 → 3)。
- 着せ替えのモジュール (T0111) のテクスチャはアトラスに含めません ([file-formats.md#テクスチャのアトラス化](spec/file-formats.md#テクスチャのアトラス化))。

### ヘルプ

ヘッダーの「Help」は、操作マニュアル (`docs/manual/README.md` の GitHub の URL) を OS の既定のブラウザで開きます
(`tauri-plugin-opener` / `@tauri-apps/plugin-opener`、MIT OR Apache-2.0。Tauri v2 の WebView は `target="_blank"` のリンクを開かないため)。

- 権限は `opener:allow-open-url` を **`https://github.com/rosylilly/ugoe/*` と `https://www.anthropic.com/legal/*`** (AI の確認の画面の規約) だけに限っています (`capabilities/default.json`)。ほかの URL・ファイルは開けません。
- 開けなかったときは、理由と URL を選んで写せる形で出します (`src/lib/HelpButton.svelte`)。
- **差し替え (テスト用):** `window.__UGOE_OPEN_URL__` に `(url) => Promise<void>` を置くと、プラグインの代わりにそれを呼びます (`src/lib/help.ts`)。

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
    読んだ値は `ApiKey::new` に渡します。
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
- **設定の画面:** ヘッダーの「Settings」(`SettingsDialog.svelte`)。プロバイダ・モデル・ベースの URL・JSON の出力の形の選択
  (「Apply」で保存)、選んだプロバイダのキーの保存・置き換え・削除 (プロバイダを変えたら先に Apply する)、「外部に送らない」、
  「Ask again」、「Test connection」(画像なしの小さなリクエストを、外部なら同意を得てから送ってプロバイダとキーを確かめる) があります。
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
  - 深度推定 (T0505) の実際の推定器はまだないので、顔の振り向きは深度マップなし (顔の外接矩形に内接する楕円体。
    `TurnDepth::Rules`) で組みます。推定器ができたら、同じ流れに「深度を推定する」段階を足します。
  - 分類とその PSD はプロジェクトを開いている間だけ持ちます (ファイルには残さない)。確からしさの低いレイヤー
    (`needs_review`) は、パネルと提案の理由の末尾に出します。分類の結果を画面で直す機能はまだありません。
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
  ドキュメントが変わったら (編集・Undo・採用) プレビューをやめます。
- **採用:** 選んだコマンドを 1 つの `Batch` (ラベルは提案の題名。一部だけなら「(n of m commands)」を添える) として適用し、
  **1 回の Undo で戻ります**。採用した後に手で直したセルは、ほかの編集と同じく `authored` になります (推定に任せた `derived` の
  セルも)。採用した提案は一覧から隠しますが、Undo で提案を作った状態に戻ると再び出て、選び直して採用し直せます
  (AI にもう一度尋ねずに済むように)。却下 (「Reject」) は一覧から消します。
- **一部だけの採用 (コマンドの単位):** コマンドごとにチェックを外せます。一覧は、同じ対象への続いたコマンドを 1 つのまとまりに
  まとめ (まとまりのチェックで全部を切り替える)、開くと 1 つずつ選べます。
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
| `get_proposals` | `ProposalState { proposals, psd, classification, regenerable }` (一覧・元にする PSD・最後の分類・作り直せる提案) |
| `classify_psd_layers(psd?, use_ai, instructions?, revise, consent?)` | 分類して覚える。`AiReply<Classification>`。`psd` を省くと取り込んだ PSD。`revise` なら今の分類を前の答えとして送る |
| `propose_rig_from_psd(use_ai, instructions?, consent?)` | 分類からリグを提案し、一覧に置く。`AiReply<ProposalView>` |
| `regenerate_rig_proposal(id, instructions?, consent?)` | 提案を前の答えと注文を添えて作り直し、後ろに並べる (いつも AI)。`AiReply<ProposalView>` |
| `preview_proposal(id, excluded)` | 外したコマンドを除いて複製に適用した姿。`ProposalPreview { model_json, report }` |
| `get_proposal_preview_bundle` / `clear_proposal_preview` | プレビューの `.ugoe` バンドル (生のバイト列) / プレビューの複製を捨てる |
| `adopt_proposal(id, excluded)` | 外したコマンドを除いて採用する (1 回の Undo で戻る)。`ApplyReport` |
| `reject_proposal(id)` | 提案を捨てる。`ProposalState` |

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
  - **保存:** エディタの保存と同じ `ProjectSession::save_to` (指紋を取り直すので、自分の保存を外の変更と取り違えない)。書き出し先 (`bundle`)
    はプロジェクトの書き出しの設定 (アトラス化) で書く。開いた後で外で書き換えられていたら、エージェントの保存は断る (利用者が
    [外での書き換え](#外での書き換え)のダイアログで決める)。
  - **外での書き換えとぶつからない:** ブリッジの編集はエディタの文書に入り、ディスクには保存するまで書かないので、外の変更としては見えない。
- **提案の出どころ:** `Change::Proposed` の名前を `Proposals::set_source` で提案に記し、`ProposalView::source` として画面に出す
  (「from claude-code」。エディタの中で作った提案は `null`)。HTTP はセッションを持たないので、名前は最後に握手したクライアント。
- **画面へのイベント (`mcp-bridge`、`BridgeEvent`):** `status` (始めた・止めた・握手した)・`approval` / `approval_closed`・`document`
  (適用・Undo・Redo・保存。画面は `EditorDocument.refresh`)・`proposals` (一覧を取り直し、AI タブを開いていなければタブに「•」)。
- **設定 (`mcp-bridge.json`):** アプリの設定のディレクトリに `enabled` (既定は偽)・`port` (無ければ空いているポート)・`token` を残す。
  トークンは初めて有効にしたときに OS の乱数で作り、起動し直しても同じ (HTTP で直接つなぐクライアントの設定を保つ)。Unix では 0600 で書く。
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
| 右 | インスペクタと物理のパネル (上。タブで切り替える) とパラメータ一覧 (下。スライダー、キーの追加と削除)。別々にスクロールし、間の区切り (`role=separator`) のドラッグか ↑ / ↓ / Home / End で高さの配分 (15〜85%) を変える。配分は表示の状態で、`localStorage` に覚える (`src/lib/layout.ts`、T0627) |
| 下 | タイムライン (モーションと表情。ビューポートの下、畳める。[タイムライン](#タイムライン))、AI の提案パネル (M6、T0637) |

- **Undo / Redo のショートカット:** Ctrl + Z / Ctrl + Shift + Z / Ctrl + Y (macOS は Cmd + Z / Cmd + Shift + Z) を `App.svelte` がウィンドウで受けます。入力欄・選択欄・編集可能な要素の中では、その要素自身の取り消しに任せて何もしません。判定は `src/lib/shortcuts.ts` (Vitest)。
- **要素の表記:** 要素を選ぶ選択欄 (ミラーの相手・ミラー元・マスク・テクスチャ) と見出し (インスペクタの先頭・デフォーマの親) は、表示名と ID を `顔 (Layer8)` の形で出します (名前が ID と同じなら ID だけ。テクスチャは使っているメッシュの名前を添える)。`src/lib/labels.ts` (Vitest)。

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
| `apps/editor/src/lib/generated/types.ts` | `Command`・`Model`・`Motion`・`Expression`・`PhysicsSettings` (とそれらが参照する型)、IPC の戻り値 (`AppInfo`・`ProjectStatus`・`AnimationData` など) | フロントエンド (`ipc.ts` が再エクスポート) |
| `apps/editor/src/lib/generated/samples.ts` | フィクスチャのモデルと、全バリアントのコマンドを serde で書き出した JSON | 型検査だけ (生成した型と serde の JSON が合うことを `svelte-check` で確かめる) |
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
