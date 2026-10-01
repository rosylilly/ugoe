---
id: T0659
title: エディタ: モジュールのポーズの違いを中身まで出す・グルーの画面・強調した要素へ寄せる
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0657]
assignee: claude-worker-T0659
---

# T0659: モジュールの違いの仕上げ

## 目的

T0657 の後続の候補をまとめる。

## 受け入れ条件

- [x] ポーズの違いを中身まで出す
- [x] グルーの一覧と編集の画面
- [x] 強調した要素へビューポートを寄せる (Fit to element)
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0657-editor-module-diff-more.md` のメモ

## メモ

- **ポーズの違い (editor-core `module_compare.rs`):** 「Pose groups or follows changed」の 1 行をやめ、`DifferenceKind` に `pose_group`・
  `pose_follow` を足した。グループは ID を持たないので要素のパーツを ` / ` でつないだもの (`HairA / HairB`) を ID にし、要素のパーツを
  1 つでも共有するグループを同じとみなして、要素の並び (`entries HairA / HairB → HairB / HairA`) と要素ごとのリンクを添える。追従
  (`ModulePose::links`) は本体の要素のパーツを ID に、リンクの足した・除いたパーツを添える。どちらにも出ない違い (グループの並びだけ) は
  「Pose: order of groups changed」。強調するメッシュはグループの要素とリンクのパーツの下、追従はリンクのパーツの下。
  `DifferenceItem.parts` (今の編集のパーツの ID。ポーズの 2 種類だけ) を足し、Select は Poses タブを開いてツリーでパーツを選ぶ。
- **グルーの Command (OP_COUNT 101 → 104):** 既存の Command にはグルーを足す・消す・設定するものが無かった (描画用の複製と
  キーフォームのセルの編集だけ) ので、enum の末尾に `CreateGlue { id?, mesh_a, mesh_b, distance }`・`SetGlue { glue, distance? }`・
  `RemoveGlue { id }` を足した (`glue_edit.rs`、codegen.rs の `glue_samples`、tests.rs の `OP_COUNT`、`EditError::InvalidGlue`)。
  頂点の組は基準の姿のキャンバス座標 (親で写した後・グルーの前) で、B の頂点ごとに距離以内でいちばん近い A の頂点を選ぶ。
  T0660 も Command を足すなら、後から main に入った方が `OP_COUNT` を合わせる。
- **グルーの画面:** 右の列に「Glue」タブ (`lib/panels/glues/`: `GluesPanel.svelte`・`glues.ts`・`glueView.svelte.ts`)。一覧・両端と
  組の数、選んだグルーの重み (すべての組に同じ値)・強さ (キーフォームごと)・Swap (見た目を保って入れ替え)・組の作り直し・削除・両端をツリーで選ぶ、
  New glue (ツリーで選んだ 2 メッシュを既定)。モジュールのプロジェクトでは、どちらかの端が本体のメッシュのグルーを本体のものとして
  読み取り専用 (`BaseLock`)、候補はモジュールのメッシュだけ。描画用の複製のグルーも読み取り専用 (Sync を案内)。着せ替えの違いのグルーの
  Select は Glue タブを開いてそのグルーを選び、両端もツリーで選ぶ (T0657 ではツリーで両端を選ぶだけだった)。
- **Fit to element:** 強調の知らせに「Fit」。描いた輪郭・点・形の範囲がビューポートの 80% に収まるように、範囲の中心で `zoomAt` して
  中央へ `panBy` (`fitMove`。範囲は 24 ピクセルより小さくみなさない)。wasm は変えていない。
- **IPC:** `compare_preview_module` の `DifferenceItem` に `parts`、`DifferenceKind` に 2 つ (`just gen` の types.ts)。Command の 3 つは
  `apply_command` を通る (types.ts・samples.ts・command.schema.json を `just gen` で作り直した)。Tauri コマンドは足していない。
- **テスト:** editor-core `glue_edit_tests.rs` (距離以内の組・ID の重なり・モジュールのメッシュの接頭辞・拒否する入力・置き換えと組の作り直し・
  削除)、`module_compare_tests.rs::pose_differences_tell_what_changed` (並びの入れ替え・追従を外す・リンクを変える・グループを除く、
  選ぶパーツと強調するメッシュ)。Vitest `glues.test.ts`、`differenceHighlight.test.ts` (グルー・ポーズの選ぶ先・Select の説明・`fitMove`)。
- **確かめたこと:** main (T0721) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`cargo check -p ugoe-editor`・`just test-gpu`
  (`CARGO_BUILD_JOBS=8`)。ヘッドレスの Chrome (SwiftShader) と本物の `Document` への使い捨ての HTTP ブリッジ (scratchpad/T0659/bridge。T0657 の
  ものに、本体へグルー BaseSeam・ポーズのグループ ArmA / ArmB・スロット Hair の `pose_parts` を足す `mutate glue-pose` を足した) で
  long-hair.ugoeproj: Glue タブで BaseSeam は「base」の印と無効の欄 → Back + Front を距離 400 で「+ Glue」(35 組、選ばれる) → 重み 0.25 →
  エージェントの編集で差分のパーツ HairA / HairB・ポーズのグループ・ArmA への追従を足して書き出し、並びを入れ替え・追従を外し・重みを 0.75 に →
  プレビューで書き出したファイルを読み込むと「Glues: 1 → 1 (changed: Glue_Back_Front)」「Pose groups: 1 → 1 (changed: HairB / HairA)」
  「HairB / HairA: entries HairA / HairB → HairB / HairA」「Pose follows: 1 → 0 (removed: ArmA)」→ 今の編集を付けてグループを押すと輪郭、
  縮めてずらしたビューポートで「Fit」を押すと要素が中央に 8 割の大きさで寄る → グルーの Select で Glue タブ・Glue_Back_Front・ツリーで Back と
  Front → 入り直してグループの Select で Poses タブ・ツリーで HairA と HairB。横のはみ出し 0。スクリーンショット: scratchpad/T0659/shots/t0659-*.png
  (マニュアルに module-glue.png・module-compare-pose.png を足した)。vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ。本体のプロジェクトでのグルーの画面 (モジュールのプロジェクトだけで見た。Vitest で
  本体のときの候補を確かめた)。描画用の複製のグルーの読み取り専用の表示 (Vitest の判定だけ)。軸のある強さのキーフォームの欄 (見本に無い。
  Vitest で見出しだけ)。除いた追従の強調は、リンクのパーツのメッシュが今の編集にもあればそれを描き「… in your current edits」と出る
  (ファイルを付けなくても見える。紛らわしいかもしれない)。大きなメッシュでの組の作り直しの重さ (頂点の総当たり)。
- **範囲外の変更:** なし (editor-core・apps/editor・docs だけ)。
- **後続の候補:**
  - Glue タブで選んだグルーの頂点の組をビューポートに重ねる (今は着せ替えの違いの強調で両端の輪郭が出るだけ)。ビューポートで組を足す・外す。
  - 強さのキーフォームに軸を足す画面 (今はキーフォームごとの値を変えるだけ。軸を足すのは `add_keyform_key` を送る AI・エージェントだけ)。
  - 除いた要素の強調で、今の編集を付けているときは「removed」と分かる知らせにする (メッシュが残っていても)。
