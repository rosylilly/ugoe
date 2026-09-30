---
id: T0649
title: エディタ: プレビューで着せ替えを試す
status: done
milestone: M4
crates: [apps/editor, docs]
depends_on: [T0123, T0647]
assignee: claude-worker-T0649
---

# T0649: エディタ: プレビューで着せ替えを試す

## 目的

ADR 0024。本体のプロジェクトのプレビューで、`.ugoemod` を読み込んで付け外しし、動きと描画順を確かめられるようにする。

## 受け入れ条件

- [x] プレビューモードの着せ替えのパネル: `.ugoemod` を読み込む・付ける・外す・同じスロットの差し替え。合成のエラー (スロットがない・要件を満たさない) を分かりやすく出す
- [x] トラッキング・物理・モーションと一緒に動く。付け外しでパラメータの値とポーズが保たれる
- [x] ヘッドレスのブラウザで確かめる。マニュアルの着せ替えの章に節を書く
- [x] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0024-module-authoring.md`

## メモ

- **画面:** プレビューのパネルの「Runtime」の下に「Dress-up」の節 (`preview/DressUpSection.svelte`)。「Load .ugoemod…」(OS のダイアログ) と
  プレビュー中のウィンドウへのドロップ (Tauri では `onDragDropEvent` でパスを受け、ブラウザでは HTML のドロップの `File`)、
  スロットの行 (本体のスロットの定義順。そのスロットの読み込んだモジュールから選ぶ = 差し替え。「(none)」で外す)、
  モジュールの行 (チェックボックスで付け外し・「↻」でファイルから読み直し・「✕」で一覧から除く)、自動で外れたことの知らせ。
  状態と WebPuppet の操作は `DressUp` (`dressUp.svelte.ts`)、`PreviewMode.dressUp` が持つ。エラーの説明は `dressUp.ts` の
  `describeProblem` (DressUpError の `code` ごとに、何が起きてどこを直すか。元の文言は「Details」)。
- **決めたこと:**
  - **モジュールはプレビューの間だけ付ける。** 出るとき `set_modules([])` で外し (付けたい組は覚える)、入るとき付け直す。
    編集の画面 (ツリー・選択・ヒットテスト・drawable の ID) は本体の要素だけを前提にしているため。
  - **再装着 (T0123):** `PreviewMode.loaded` → `DressUp.loaded` で、付けたい組と `attached_modules()` を比べる。違えば
    `set_modules` で揃え、だめなら 1 つずつ試して、付けられないものを外し「X was taken off: 理由」と知らせる (理由は試したときの
    DressUpError から。load の警告の文字列は読まない)。プレビューの外でスロットを消してから入ったときも同じ経路。
    WebPuppet を作り直したとき (GPU の喪失) のために、読み込んだバイト列を持っておき読み込み直す。
  - **一覧をプロジェクトに残す:** 残す。`project.json` の `editor.dress_up` = `{ modules: [{ path, attached }] }`
    (`src-tauri/src/dress_up.rs`、serde default、空なら書かない)。モジュールを書き出し直しては本体で試す、を繰り返すため。
    パスは `.ugoeproj` と同じフォルダの中なら相対パス。プレビューに入るたびにファイルから読み直す (書き出し直した版を使う)。
    変えると未保存の印が付く (光源と同じ)。自動で外れたものは残した一覧を書き換えない。ブラウザでドロップしたもの (パスなし) は残さない。
    editor-core は触らず、`ProjectSession` に `editor_value` / `set_editor_value` を足した (`project.rs` は 2 つの関数を `pub(crate)` にしただけ)。
  - IPC: `get_preview_modules`・`set_preview_modules`・`read_module_file` (`.ugoemod` だけ)。`ipc.ts` と `types.ts` (`PreviewModule`) を更新。
- **テスト:** `dressUp.test.ts` (Vitest 8 件: 説明の文言・スロットの行・偽の WebPuppet と IPC で読み込み/差し替え/保存・エラー・
  開き直し・プレビューの外で外す・読み直しで外れたことの知らせ・作り直した WebPuppet への読み込み直し)、`dress_up.rs` のテスト 3 件。
  `tracking.test.ts` の偽の WebPuppet に `modules_json` / `attached_modules` を足した。
- **確かめたこと:** ビルドした画面 (`just wasm`・`vite build`) を本物の `Document` への HTTP ブリッジ (scratchpad/T0649/bridge。`gen` で
  T0111 のフィクスチャ `dress_up_base`・`long_hair_module`・`dress_module` から本体のプロジェクトと `.ugoemod` 5 つを作る。
  リポジトリには入れていない) でヘッドレスの Chrome (SwiftShader・WebGL2) で開いた。AngleX 20・BodySway 1 にしてから
  LongHair → Dress を読み込む (値は 20 / 1 のまま) → ShortHair で Hair が差し替わる (値はそのまま) → スロットの欄で LongHair に戻す →
  Hat (`unknown_slot`) と Cape (`missing_parameter`) はその行に理由 → `notes.txt` と壊れた `Broken.ugoemod` のドロップはパネルの上に
  理由 → 物理 (LongHair のリグ) で画像が毎フレーム変わる → Dress を外す/付ける (AngleX -30・BodySway 1 のまま) → 出ると本体だけ →
  ページを読み直すと残した一覧から LongHair と Dress が付く → Slots タブで Outfit を消して入り直すと「Dress was taken off: …」→
  横のはみ出し 0。トラッキングの記録 (ヨー 20) を再生しながら全部外す・ShortHair を付けても AngleX 20 のまま、トラッカーは動き続ける。
  スクリーンショット: scratchpad/T0649/shots/b-0{1..6}-*.png・c-tracking.png (マニュアルの画像 2 枚はそのうち b-02・b-03)。
  `just ci`・`cargo check -p ugoe-editor` が通る。使い終えた vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ (OS のダイアログ・OS からのドロップ `onDragDropEvent`・`read_module_file`)。
  モーションと表情を付けたまま付け外しすること (見本の本体にモーションが無い。Puppet の規則とテストは T0122/T0123)。
  ライティングの光源を当てたモジュールの見た目 (法線マップ付きのモジュールは T0123 で確認)。ポーズの引き継ぎ (本体にポーズが無い)。
  JSON で表示しているとき (バンドルが作れないとき) に、読み直しの付け直しの後で物理がドキュメントの設定にならないことがある
  (T0123 の付け直しが `set_physics_json` の前に走るため。バンドルが作れれば起きない)。
- **範囲:** `apps/editor` と `docs` だけ。T0648 と重なりそうな所: `project.rs` (関数 2 つの可視性と `ProjectSession` のメソッド 2 つ)、
  `App.svelte` (2 行)、`lib.rs` のハンドラの登録、`bindings.rs` の 1 行。
- **後続の候補:**
  - モジュールを付けたまま編集の画面でも見る (スロットの範囲を調整しながら重なりを確かめる)。drawable の ID とツリーの対応が要る。T0648 の下敷きと合わせて考える。
  - T0124 (再装着の警告を構造化) が入れば、`DressUp.loaded` の 1 つずつ試す処理を警告の `code` に置き換えられる。
  - 見本のアバターのスロットと見本のモジュール (T0710) ができたら、マニュアルの画像をそれで撮り直す (今はフィクスチャの四角形)。
