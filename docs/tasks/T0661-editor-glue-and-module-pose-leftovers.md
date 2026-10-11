---
id: T0661
title: エディタ: グルーの頂点の組をビューポートで編集・強さの軸・着せ替えのモジュールのポーズをプレビューで切り替え
status: done
milestone: M4
crates: [ugoe-wasm, ugoe-editor-core, apps/editor, bindings/web, docs]
depends_on: [T0659, T0660]
assignee: claude-worker-T0661
---

# T0661: グルーとモジュールのポーズの残り

## 目的

T0659・T0660 の後続の候補のうち、エディタの画面で閉じるものをまとめる。

## 受け入れ条件

- [x] Glue タブで選んだグルーの頂点の組をビューポートに重ねて表示し、ビューポートで組を足す・外す (Command を通す)
- [x] グルーの強さのキーフォームに軸を足す画面 (今は `add_keyform_key` を送る AI・エージェントだけが足せる)
- [x] 着せ替えの違いの強調で、除いた要素は今の編集にメッシュが残っていても「removed」と分かる知らせにする
- [x] プレビューの Pose の節に、着せ替えで付けたモジュールのポーズのグループも出す (WebPuppet にポーズのグループを返す口が要る。ugoe-wasm)
- [x] 物理の無い本体で開けないモジュールを、衝突体の参照を外して開く選択肢 (外すことを明示して確認する。T0655 の「黙って消さない」を守る)
- [x] `just ci`・`just gen`・`just wasm-check`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0659-editor-module-diff-poses.md`・`docs/tasks/T0660-editor-pose-and-sample-leftovers.md` のメモ

## メモ

- **crates に bindings/web を足した:** team-lead の指示どおり、ugoe-wasm に `pose_json` を足したので bindings/web のスタブ
  (`stub/ugoe_wasm.d.ts`) にも 1 行足した (並びはアルファベット順)。
- **グルーの組をビューポートで編集** (`apps/editor/src/lib/viewport/tools/glue/GlueOverlay.svelte`。ToolOverlay が出し、App から `GlueView` を渡す):
  Glue タブで選んだグルーの両端の輪郭 (A 緑・B 紫の破線)・組の線・組んだ頂点を重ねる。下のバーの「Add pairs」で両端の頂点を出し、A と B を
  1 つずつ押すと `set_glue` で組を足す (`glues.ts` の `withPair`。同じ B の組は置き換え、重みはグルーの重み)。「Remove pairs」で組の中点の四角を
  押すと外す (`withoutPair`)。Esc で抜ける。本体・描画用の複製のグルーは重ねるだけ。モードと先に押した頂点は `GlueView.pairMode`・`pending`。
  Command は足していない (`SetGlue` で足りた。OP_COUNT は 104 のまま)。
- **強さの軸** (`GluesPanel.svelte`): パラメータとキーを選んで「+ Axis」、軸ごとに「+ Key」と「×」(`add_keyform_key`・`remove_keyform_key` の
  `target: { kind: "glue" }`。editor-core は前から対応していた)。キーの入力は範囲の中で軸にまだ無い値だけ (`keyValue`)。
- **除いた要素の強調** (`differenceHighlight.ts` の `missingIn`・`highlightNotice`、`DifferenceOverlay.svelte`): 描いている版に無い要素
  (今の編集で除いた・ファイルに無い足した) は、メッシュが残っていても知らせを「Removed: … is not in your current edits; the outline shows its
  meshes that are still there. Put on the file to see it.」(足したものは「Added: …」) にし、輪郭を破線にする。
- **プレビューのポーズにモジュールのグループ** (ugoe-wasm `WebPuppet::pose_json` = `animation::pose_json`。合成したモデルのポーズの JSON):
  `preview.ts` の `modulePoseGroups` (編集中のモデルのグループを `previewPart` で見分けて除く)・`modulePoseLabels` (`Sailor group 1`)。
  `PreviewMode.modulePoses`・`modulePoseShown`・`showModulePose` (付け外しと読み込みのたびに読み直し、選んだ要素を渡し直す)。このグループがあれば
  「Pose」の段に「(not set up)」を出さない。ugoe-wasm の README と、エディタ・bindings/web のスタブに足した。
- **衝突体の参照を外して開く** (editor-core `ColliderReference`・`ModuleContext::drop_unavailable_colliders`・
  `Project::load_module_without_unavailable_colliders`、Tauri `ModuleProblem.collider_references`・`open_module_with_base(.., drop_colliders)`、
  エディタ `ModuleProblemBar` の「Open without these colliders…」→ 外す参照と保存までファイルが変わらないことを並べた確認 →「Remove and open」、
  `moduleProject.ts` の `dropCollidersPrompt`・`droppedCollidersNotice`)。外せば開けるときだけ出す (`module_open_problem` が外して読めたときだけ
  `collider_references` を入れる)。`reload_base` の問題には出さない。開いたら未保存の印 (保存すると `module.json` から参照が消える)。
- **IPC:** `open_module_with_base` に `drop_colliders: bool` (ipc.ts の `openModuleWithBase(.., dropColliders = false)`)、`ModuleProblem` に
  `collider_references`、`ColliderReference` を types.ts に (`just gen`)。
- **文書:** editor.md (プレビューのポーズ・除いた要素の強調・IPC の表・本体の問題の表と「衝突体の参照を外して開く」・グルーの画面の強さの軸と組の編集)、
  manual/dress-up.md・preview.md (画像 module-glue-pairs.png・preview-module-pose.png・module-problem-drop.png を足した)、ugoe-wasm README。
- **テスト:** ugoe-wasm `pose_json_includes_module_groups`。editor-core `exposed_colliders_need_the_base_physics` (外すと開け、モジュール自身の衝突体への
  参照は残る) と `module_project_opens_without_unavailable_colliders` (ディスクのプロジェクト。外さずには開けず、外して保存すると module.json から消える)。
  エディタ `exposed_colliders_without_base_physics_are_explained` に `collider_references`。Vitest: glues.test.ts (`withPair`・`withoutPair`・軸のコマンド・
  `unboundParameters`・`keyValue`)、differenceHighlight.test.ts (`missingIn`・Removed / Added の知らせ)、preview.test.ts (`modulePoseGroups`・`modulePoseLabels`)、
  moduleProject.test.ts (`drop_colliders` の直し方・確認と知らせの文)。
- **確かめたこと:** `just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`just wasm-check`・`cargo check -p ugoe-editor`・`just test-gpu` (`CARGO_BUILD_JOBS=8`。
  main は ae681e5 から動いていない)。ヘッドレスの Chrome (SwiftShader) と使い捨てのブリッジ (scratchpad/T0661/bridge。T0660 のものに
  `drop_colliders`・`collider_references`・`compare_preview_module`・見本の準備 `t0661-prepare` を足した。Sailor のスカーフと襟を別のパーツ ScarfA / CollarB に
  分け、それをグループにした `sailor-posed.ugoemod` を書く) で:
  A. long-hair の Glue タブで Back + Front を距離 400 で作る → ビューポートに 35 本の組 → Remove pairs で 1 つ外す (34) → Add pairs で組んでいない A と B を
  押す (35。先に押した頂点は橙、案内「Now click a vertex of Front」) → Esc でモードを抜け、Ctrl+Z で 34 に戻る。B. 「+ Axis」で AngleX (キー 0) →
  「+ Key」で 30 → 強さの欄が「AngleX = 0」「AngleX = 30」になり 30 を 0 に。C. 本体のプレビューで sailor-posed.ugoemod を付けると「Pose」に
  「Sailor group 1」(ScarfA / CollarB) が出て CollarB に切り替わる。D. sailor のプロジェクト (グループなし) でファイル (グループあり) を読み込み、
  今の編集を付けてグループを押すと破線の輪郭と「Removed: ScarfA / CollarB is not in your current edits; …」。E. 物理の無い本体で肩に当たる long-hair を
  開く → 枠の「Open without these colliders…」→ 確認 (Sway → Shoulders) →「Remove and open」で開き、リグの衝突体は空、未保存の印、知らせ。
  横のはみ出し 0。スクリーンショット: <scratchpad>/T0661/shots/t0661-01〜10-*.png。vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ (IPC の引数名 `dropColliders` → `drop_colliders` の変換は Tauri の既定に頼っている)。本体のプロジェクトでのグルーの
  組の編集 (モジュールのプロジェクトだけで見た)。組んだ頂点がグルーで重なっているときに、重なった頂点の片方を選び分けること (上に描いた B が押される)。
  大きなメッシュでの頂点の丸の数 (すべて描く)。本体のポーズとモジュールのポーズの両方があるときの並び (Vitest だけ)。モジュールの追従 (`links`) だけを
  持つモジュール (本体のグループに入るので、モジュールのグループには出ない。Vitest だけ)。
- **範囲外の変更:** bindings/web のスタブ 1 行 (team-lead の指示。crates に足した)。
- **後続の候補:**
  - グルーの組の編集で、重なった頂点を選び分ける (Alt + クリックで順に選ぶ、またはグルーを掛ける前の位置で描く。WebPuppet にグルーの前の頂点を返す口が要る)。
  - 「Add pairs」で範囲選択して、まとめて最も近い頂点と組む。
  - プレビューのポーズのグループに、モジュールの追従 (本体の要素の `links` に入った衣装の差分) を見せる (今は本体の要素と一緒に切り替わるだけで、どの衣装が追従するかは出ない)。
  - 衝突体の参照を外して開いた後、「Physics」タブの外したリグに「本体に物理が戻れば当たりを足し直せる」案内を出す (今は知らせだけ)。

