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
| テクスチャ | `add_texture`、`set_texture_image`、`remove_texture` |
| キャンバス | `set_canvas` (`width`・`height`・`origin`。大きさは 0 より大きい有限の値。要素の座標は変えない) |
| キーフォームの推定 | `infer_keyforms`、`mirror_keyforms`、`mark_keyform_authored`、`mark_keyform_derived`、`set_keyform_inference`、`set_mirror_pair` (下の「推定とミラー」)、`set_depth`・`apply_depth_preset` ([深度の編集](#深度の編集)) |
| モーション・表情 | `add_motion`、`remove_motion`、`rename_motion`、`set_motion_properties`、`add_motion_curve`、`remove_motion_curve`、`set_motion_curve_fade`、`set_motion_key`、`move_motion_key`、`remove_motion_key`、`set_motion_segment`、`set_motion_events`、`add_expression`、`remove_expression`、`rename_expression`、`set_expression_properties`、`set_expression_parameters` ([モーションと表情の編集](#モーションと表情の編集)) |
| 物理 | `set_physics_world`、`add_physics_rig`、`create_physics_rig`、`remove_physics_rig`、`rename_physics_rig`、`set_physics_inputs`、`set_physics_outputs`、`set_physics_particles`、`set_physics_normalization`、`set_physics_rig_colliders`、`set_physics_frame`、`add_physics_collider`、`remove_physics_collider`、`set_physics_collider`、`reorder_physics_rig` ([物理の編集](#物理の編集)) |
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
| `remove_mesh` | **連鎖して直す。** 他のメッシュの `masks` と `mirror` から外し、このメッシュを使うグルーと当たり判定を削除する。描画用の複製の元を消すと、複製はつなぐグルーを失って普通のメッシュ (基準の形) として残る |

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

### 物理の画面での操作

| 場所 | 操作 |
| --- | --- |
| 物理のパネル (`panels/physics/PhysicsPanel.svelte`) | 右の列の上の区画の「Physics」タブ (「Inspector」と切り替える)。「Preview」/「Stop preview」、「World」(重力・風・周波数 → `set_physics_world`)、リグの一覧 (上から処理する順。▲▼ で並べ替える → `reorder_physics_rig`。前のリグの出力を入力に読むリグには「relay」の印) と「+ Rig」(空のリグ → `add_physics_rig`)、「+ From selected deformers」(ツリーで選んだ回転デフォーマを親から子の順に並べて `create_physics_rig`) |
| リグの編集 (`panels/physics/RigEditor.svelte`) | 名前、「Gravity frame」、「Delete rig」、粒子 (「Length」`radius`・「Sway」`mobility`・「Speed」`delay`・「Settle」`acceleration`・「Hit r」`collision_radius`・位置。「+ Particle」は最後の区間と同じ向き・長さで足す)、入力、出力 (種類・書き込み先・粒子・成分・倍率・重み・反転・パラメータへの出力の合成の方法「Overwrite」/「Add」([physics.md#合成の方法](spec/physics.md#合成の方法))。頂点の出力は作ったとき重みがすべて 1)、「Collides with」、「Input range」。欄を確定するたびにコマンドを 1 つ送る |
| 衝突体 (`panels/physics/ColliderSection.svelte`) | 「+ Circle」「+ Capsule」(主な選択がデフォーマならその原点 (ワープは中心) に取り付け、なければキャンバスの中央)。名前・「Attach to」・「Shape」(どちらも今の見た目の位置を保って写し直す)・位置・半径・削除 |
| ビューポート (`viewport/tools/physics/PhysicsOverlay.svelte`) | 物理のパネルを開いている間、衝突体 (青) と粒子の鎖 (オレンジ、根元は四角。選んだリグは濃い色) を重ねる。粒子は動的物理の設定のときだけ (物理空間がキャンバス座標なので)。衝突体をドラッグで動かし (カプセルは端の丸で端だけ)、離したときに `set_physics_collider` を 1 つ送る。鎖を押すとそのリグを選ぶ |

- 衝突体と粒子の位置は wasm の `WebPuppet.deformer_point(id, x, y)` (デフォーマの局所座標 → キャンバス座標) で置き、ドラッグの逆写像は `deformer_jacobian` を使ったニュートン法です (`viewport/tools/physics/physicsGeometry.ts`、Vitest)。
  止めているときの粒子は初期位置 (根元は取り付け先のデフォーマで写し、ほかは根元からの相対位置)、プレビュー中は `WebPuppet.physics_particles(rig)` の今の位置です。
- パネルの組み立て (新しいリグ・衝突体・粒子の足し引き・出力先) は `panels/physics/physics.ts` の純粋な関数で、Vitest で検査します。

### 物理のプレビュー

`ViewportController.startPhysicsPreview()` / `stopPhysicsPreview()` (`viewport/controller.svelte.ts`)。

- プレビュー中は `requestAnimationFrame` ごとに `WebPuppet.update(dt)` (1 フレーム最大 0.1 秒) を呼んで描き、`animationFrame` を増やしてオーバーレイに読み直させます。スライダー (`set_parameter`) は `Puppet::update` の基準値になるので、動かすと根元が動いて揺れます。
- WebPuppet の段は、プレビュー中は物理だけを有効にし、それ以外 (目パチ・呼吸・リップシンク・ポーズ) は常に止めます (モデルを読み直すたびに設定し直す)。
- 始めるときと止めるときに `reset_physics` で粒子を静止姿勢に戻します。止めると物理の段を止め (動的物理の加算量が 0 に戻る)、物理が書き込んだパラメータをスライダーの値に戻すので、編集中の姿に戻ります。
- プレビュー中に編集すると、モデルと物理を読み直し、揺れは静止した姿からやり直します。

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
- **まだ無いもの:** フェイストラッキング (ビューアの T0703 の仕組み) は T0636 で足す (`FrameDriver.beforeUpdate` で入力を渡す想定)。目パチ・呼吸・リップシンクの
  設定 (`Model.effects`) を編集するコマンドと画面はまだ無い (読み込んだモデルの設定で動く)。

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
| `save_project` | 開いた (前回保存した) 場所に保存する。保存先がなければエラー |
| `save_project_as(path)` | 別の場所に保存し、以後の保存先にする。拡張子 `.ugoeproj` がなければ付け、名前をディレクトリ名にする |
| `export_bundle(path)` | `.ugoe` ファイルに書き出す。保存状態は変えない |
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

### ファイルダイアログ

ファイルメニューのパスは OS のファイルダイアログ (`tauri-plugin-dialog` / `@tauri-apps/plugin-dialog`、MIT OR Apache-2.0) で選びます。
パスの入力欄はありません (ダイアログが使えない環境の予備も置かない。ヘッドレスの確認では下の差し替えを使う)。
取り込みの警告はメニューの横に一覧で、保存・書き出しの結果 (書いた場所の絶対パス) はメニューの横に出します。

| 操作 | ダイアログ | フィルタ | 既定 |
| --- | --- | --- | --- |
| New from PSD… / Import PSD… | ファイルを開く | `.psd` | 最初の場所 |
| Open… | **フォルダーを選ぶ** (`.ugoeproj` はディレクトリ) | なし | 最初の場所 |
| Save As… (保存先のない Save も) | 保存 (新しいフォルダーを作れる) | `.ugoeproj` | 最初の場所に `<プロジェクト名>.ugoeproj` |
| Export… | 保存 | `.ugoe` | 最初の場所に `<プロジェクト名>.ugoe` |

- **最初の場所:** 最後にダイアログで選んで成功した場所 → 開いているプロジェクトの隣 → ドキュメント (`documentDir()`、なければホーム) の順。
  プロジェクト名のうちファイル名に使えない文字は `_` にする。
- 保存のダイアログで選んだ名前に拡張子 (`.ugoeproj` / `.ugoe`) がなければ付け足す (`save_project_as` も `.ugoeproj` を付ける)。
- ダイアログを閉じた (取り消した) ときは何もしない。ダイアログを開いている間はメニューのボタンを押せない。
- 権限 (`capabilities/default.json`) は `dialog:allow-open` と `dialog:allow-save` だけ (メッセージ・確認のダイアログは使わないので入れない)。
  既定の場所を得る `@tauri-apps/api/path` は `core:default` に含まれる。
- コード: ダイアログの呼び出しは `src/lib/fileDialogs.ts` (`FileDialogs`)、フィルタ・拡張子・パスの整形は `src/lib/filePaths.ts` (Vitest)。
- **差し替え (テスト用):** `window.__UGOE_FILE_DIALOGS__` に `FileDialogs` (`openFile`・`openDirectory`・`saveFile`・`defaultDirectory`) を置くと、
  ダイアログの代わりにそれを呼ぶ。ヘッドレスのブラウザでは、IPC の差し替え (`window.__TAURI_INTERNALS__`) と一緒に固定のパスを返すものを置く。

### ヘルプ

ヘッダーの「Help」は、操作マニュアル (`docs/manual/README.md` の GitHub の URL) を OS の既定のブラウザで開きます
(`tauri-plugin-opener` / `@tauri-apps/plugin-opener`、MIT OR Apache-2.0。Tauri v2 の WebView は `target="_blank"` のリンクを開かないため)。

- 権限は `opener:allow-open-url` を **`https://github.com/rosylilly/ugoe/*` だけ** に限っています (`capabilities/default.json`)。ほかの URL・ファイルは開けません。
- 開けなかったときは、理由と URL を選んで写せる形で出します (`src/lib/HelpButton.svelte`)。
- **差し替え (テスト用):** `window.__UGOE_OPEN_URL__` に `(url) => Promise<void>` を置くと、プラグインの代わりにそれを呼びます (`src/lib/help.ts`)。

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
| 下 | タイムライン (モーションと表情。ビューポートの下、畳める。[タイムライン](#タイムライン))、AI の提案パネル (M5) |

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
