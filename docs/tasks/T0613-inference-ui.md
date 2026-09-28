---
id: T0613
title: エディタ: 推定の操作と由来の表示
status: done
milestone: M2
crates: [apps/editor]
depends_on: [T0611, T0602, T0608]
assignee: claude-worker-T0613
---

# T0613: エディタ: 推定の操作と由来の表示

## 目的

パラメータのキーの一覧で、セルの由来 (`authored` / `derived`) を表示し、推定・ミラー・推定のやり直しを操作できるようにする。

## 受け入れ条件

- [x] 操作はすべて `Command` を送ることで行う
- [x] `derived` のセルが見た目で区別できる
- [x] 自動のミラーの作り直しが失敗した (ミラー元の頂点が許容距離を超えて動いた) ことを利用者に知らせる (T0614 のメモ)
- [x] `just web-check` が通る
- [x] `just ci` が通る

## 参考

- `docs/spec/keyform-inference.md`

## メモ

- **コマンド** (`ugoe-editor-core`): `set_keyform_inference { target, inference }` (格子の推定の設定を置き換え、
  `touched` に積んで方法を記録していない `derived` のセルを新しい設定で作り直す。未知の方法は拒否、軸でない
  yaw / pitch は検証で拒否) と、任意の項目だった `set_mirror_pair { target, mirror, axis }` (メッシュ・デフォーマの
  対を双方向に張り、前の相手の対は外す。対が変わった要素を積むので、それをミラー元にする格子をミラーし直す)。
- **警告の設計:** `inference::rederive` はミラーのやり直しの失敗を `RederiveWarning { target, source, message }`
  (格子とミラー元の組ごとに 1 つ) として返す。`Document::apply_with_report` が `ApplyReport { warnings }` を返し、
  `Document::apply` は署名を変えずに警告を捨てる (CLI・PSD 取り込みなどの呼び出し側はそのまま)。Tauri の
  `apply_command` は `ApplyReport` を返す (`ipc.ts` も更新)。フロントエンドは `EditorDocument.warnings` に積み、
  インスペクタの上の通知 (`RederiveNotice.svelte`) で「Dismiss」まで出す。加算合成・深度からの回転の失敗は今までどおり拒否。
- **UI**: `panels/inspector/inference/` (`InferenceSection.svelte`: 推定の設定・Infer・Infer (overwrite derived)・
  Mark authored / derived・Mirror from…・ミラー元の表示・Mirror partner、`OriginLegend.svelte`、`RederiveNotice.svelte`、
  `cellSelection.svelte.ts`、純粋な関数の `inference.ts`)。キーの格子 (`KeyformInspector.svelte`) はセルを由来ごとの
  記号 (`A` `+` `R` `M` `D`) と色 (推定は方法の色の破線と薄い地) で示し、凡例とツールチップ (作り直す方法・ミラー元) を
  付け、クリックで選択・Shift / Ctrl で追加選択できるようにした。`InspectorPanel.svelte` には通知と推定の節を置いただけ。
- **テスト**: Rust 8 件 (`inference_tests.rs`: 警告なしの成功、頂点を動かしてミラーのやり直しが失敗したときの警告と
  セルの保持・`apply` との一致、対を外したときの警告、`set_keyform_inference` の作り直し (回転の設定・加算合成へ
  戻す) と拒否、`set_mirror_pair` の双方向の張り替え・自分自身・拒否)。JSON の往復の見本と `sample_commands` に追加。
  Vitest 15 件 (`inference.test.ts`: 由来の表示、設定のフォームの往復と誤り、コマンドの組み立て、ミラーの元の候補、
  セルの選択、警告の文と重ね方)。
- **検証したこと**: ヘッドレス Chrome (WebGL2、SwiftShader) で `vite build` した画面に、Tauri の IPC を本物の
  `Document::apply_with_report` に中継する使い捨ての HTTP ブリッジ (スクラッチ領域、リポジトリ外) をつなぎ、選択は
  ツリーのクリックで行った。モデルは `keyform_inference` の `EyeL` / `EyeR` を AngleX の 3 キーの格子にしたもの。
  確かめたこと: `EyeR` の `D A D` →「Mirror」で `M A M` とミラー元の表示「Mirrored cells come from EyeL.」
  (モデルの `origins` と `mirror_source` も一致)、クリックと Shift クリックで 2 セルを選び「Mark authored」で `A A A`、
  「Mark derived」で加算合成で作り直されて `+ A +`、`EyeL` の「Mirror from」の候補 (EyeR・自分自身)、`EyeL` の
  「Mirror partner」を Nose にすると通知「Could not re-mirror mesh EyeR from mesh EyeL: ... is not the mirror of ...」が
  出て `EyeR` のセルは `M A M` のまま、「Dismiss」で消え、Undo で対が戻ること。`Face` (ワープ) の格子の `D` の表示と
  設定のフォーム (深度からの回転、AngleX / AngleY)、Angle scale 0.5 の「Apply settings」で `inference` が変わりセルが
  `R` に作り直され値も変わること、Additive にすると `inference` が消え、「Infer (overwrite derived)」で `+` になること。
- **検証できなかったこと**: 実際の Tauri の WebView での操作 (T0608 と同じく)。
- **後続の候補**: ミラーの失敗の頂点 (`UnmatchedVertices`) をビューポートで強調する。深度の塗り (T0505 の AI 推定の前に
  手で塗る道具)。回転の中心をビューポートでドラッグして決める。
