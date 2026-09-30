---
id: T0653
title: エディタ: モジュールのプロジェクトの仕上げ (見た目を保つ移動の物理・本体のバンドルの作り直し・書き出したものとの比較)
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0651]
assignee: claude-worker-T0653
---

# T0653: エディタ: モジュールのプロジェクトの仕上げ

## 目的

T0651 の後続の候補をまとめる。

## 受け入れ条件

- [x] 見た目を保つ移動 (`SetModuleSettings.keep_look`) で、物理の粒子と衝突体の位置も新しい取り付け先の座標に写し直す
- [x] プレビューの比較で使う本体だけのバンドルを、本体が変わったときだけ作る (今はプレビュー中の編集のたびに作り直す)
- [x] 前に書き出した同じ名前の `.ugoemod` と、今の編集を並べて比べられる
- [x] T0712 の見本のプロジェクトで、物理のリグの ▲▼ とテンプレートの無効化を画面で確かめる。プレビュー中の編集で今の編集を取り直す経路も画面で確かめる
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0651-editor-module-project-leftovers.md` のメモ

## メモ

- **見た目を保つ移動の物理:** `ModuleContext::keep_look` に物理を渡し、モジュールの衝突体で取り付け先に付くもの (モジュールの中では `attach` なし)
  の形状の位置 (`center`・`a`・`b`) を、基準の姿で前の取り付け先 → キャンバス → 新しい取り付け先の局所座標と写し直す (`deformer_edit::Rest`・`Frame`。
  ReparentPreservingLook と同じ写し方。半径はピクセルなので変えない)。**リグの粒子は写し直さない:** モジュールのリグの根元はモジュールのデフォーマ
  (出力先か `frame`) の局所座標かキャンバス座標で、本体のデフォーマを参照するリグはモジュールとして表せない (`Splitter::physics`) ので、
  デフォーマを写し直せば基準の姿での根元の位置は保たれる (テストで根元のキャンバス座標が変わらないことを確かめた)。Undo で戻る (スナップショット)。
  Command の enum は足していない (`OP_COUNT` 100 のまま)。
- **本体だけのバンドル:** `ModuleContext::base_revision()` (本体を読み込むたびにプロセスの中で重ならない番号。`reconfigured` では変わらない。
  静的な `AtomicU64`) を `ModuleStatus.base_revision` で渡し、`PreviewMode` は本体の版が前に取ったものと違うときだけ `get_preview_base_bundle` を呼ぶ
  (バイト列の比較 `sameBytes` を除いた)。取ったバイト列はプレビューを出ても覚えておく。
- **書き出したものとの比較:** 今の編集は `<名前> (editing)` の名前で読み込む (`get_preview_module(name)`・`ugoe_editor_core::export_module_as`・
  `editingName`)。前に書き出した同じ名前の `.ugoemod` も読み込めて、同じスロットで差し替えて見比べられる。一覧の行はモジュールの名前
  (`DressUpEntry.label`) と `editing` の印、スロットの選択肢は「CatEars (editing)」。`same_name` の扱いは、その読み込み名と同じ名前のファイルだけに残した。
- **IPC:** `get_preview_module` に `name`、`ModuleStatus` に `base_revision` (ipc.ts・types.ts は `just gen`)。
- **テスト:** editor-core `module_settings_keep_physics_in_place_when_the_anchor_changes` (LongHair を Hair → Outfit、衝突体と根元のキャンバス座標・
  分けた形・Undo・保たないとき)、`base_revision_changes_only_with_the_base`、`module_can_be_exported_under_another_name`。
  Vitest `compares the current edits with an exported file of the same name (T0653)`。
- **確かめたこと:** main (T0717・T0129・T0406) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci` (`cargo check -p ugoe-editor` を含む)・
  `just test-gpu`。ヘッドレスの Chrome (SwiftShader) と本物の `Document`/`Project` への使い捨ての HTTP ブリッジ (scratchpad/T0653/bridge。T0651 のものに
  `base_revision`・`get_preview_module` の `name`・本体にリグと衝突体を足す `base-rig` を足し、Tauri のイベント `mcp-bridge` を発火できるようにした) で、
  `ugoe sample dress-up --project` の見本:
  - 本体にリグ `BaseHead` と衝突体を足した見本の `twin-tails.ugoeproj`: `BaseHead` の ▲▼ は無効、`TailL`/`TailR` はモジュールどうしで入れ替わり
    (`TailL` ▼ → `BaseHead,TailR,TailL`)、Undo で戻る。テンプレートは「Create missing parameters」無効、首の候補は `TailL`/`TailR` だけ、
    「+ Body follow」有効・「+ Bounce」「+ Neck follow」無効 (理由の説明つき)。
  - `cat-ears.ugoeproj`: HeadWarp を選んで「+ Circle」→ 設定で Outfit (BodyWarp) へ「Keep where it appears」で移すと、衝突体は `attach: BodyWarp`・
    `center (0.5,0.5) → (0.5,0.0625)` で画面の同じ位置に残る。Undo で戻り、外して移すと耳と一緒に首元へずれる。
  - プレビュー: 入ると本体 1 回・今の編集 1 回。書き出した `cat-ears.ugoemod` (同じ名前) を読み込むと付き (エラーなし)、選択肢は
    `CatEars (editing)`・`CatEars`。**プレビュー中の編集** (エージェントの編集と同じ経路: ブリッジで `remove_mesh` → `mcp-bridge` の `document` の知らせ →
    `doc.refresh`) で今の編集だけ取り直し (本体 0 回)、耳のない今の編集と耳のあるファイルを差し替えて見比べた。出て入り直しても本体は 0 回、
    `reload_base` の後は 1 回。横のはみ出し 0。
  スクリーンショット: scratchpad/T0653/shots/t0653-*.png (マニュアルの画像は module-compare-exported.png を足した)。使い終えた vite preview とブリッジは止めた。
  (scratchpad = /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad)
- **確かめていないこと:** 実際の Tauri のウィンドウ (`State` 越しの `get_preview_module(name)`)。利用者が画面から行うプレビュー中の編集 (プレビュー中は
  編集の列が隠れ、Undo のボタンも無い。取り直しの経路はエージェントの編集で確かめた)。カプセルの衝突体の写し直し (円と同じ関数。画面では円だけ)。
  物理を動かしたときの揺れ方 (写し直すのは基準の姿だけ)。
- **範囲外の変更:** なし (editor-core・apps/editor・docs だけ)。
- **後続の候補:**
  - 見本の本体に物理のリグを入れる (T0712 の見本の本体にはリグが無く、今回はブリッジで足した)。
  - プレビューの一覧で、今の編集と書き出したファイルの違い (要素の数・変わったメッシュなど) を文字でも出す。
  - 本体を開き直したときも本体だけのバンドルをエディタのセッションで持ち回る (今はフロントエンドが版ごとに 1 つ覚えるだけ)。
