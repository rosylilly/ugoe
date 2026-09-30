---
id: T0650
title: エディタ: モジュールのプロジェクトの使い勝手 (設定の画面・本体の項目の無効化・本体の変化)
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0648, T0649]
assignee: claude-worker-T0650
---

# T0650: エディタ: モジュールのプロジェクトの使い勝手

## 目的

T0648 の後続の候補をまとめる。モジュールのプロジェクトを便利に使えるようにする。

## 受け入れ条件

- [x] モジュールの名前・要件 (`requires`)・スロットを後から変える画面
- [x] インスペクタで本体の要素・本体のもの (アバタープロファイル・パラメータなど) の欄を無効にし、「本体のプロジェクトで直す」と示す
- [x] 本体が変わった (開いた後に本体のファイルが更新された・モジュールの要素が合わなくなった) ことに気づき、読み直すか尋ねる
- [x] モジュールのプロジェクトのプレビューの着せ替えのパネル (T0649) で、このモジュールが使っているスロットに別のモジュールを付けられないようにする
- [x] 本体の要素をビューポートでも見分けられるようにする (ツリーは T0648 で済み)。本体の要素を編集しようとしたときの言葉を「本体の要素はここでは編集できません。本体のプロジェクトで直してください」の形に。ID の接頭辞 (合成で付くもの) をインスペクタやテクスチャの名前でも隠す
- [x] モジュールで使える機能・使えない機能 (パラメータを足す AI の提案など) を画面に示す (新規作成のダイアログ・`Module:` の印のツールチップ)
- [x] 本体に関わるすべてのエラー (本体が無い・スロットが無い・本体が変わって合成できない) で「Locate base…」などの直し方を出す。読み取り専用で開く形も検討する
- [x] ヘッドレスのブラウザで確かめる。マニュアルを更新する。`just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0648-editor-module-project.md`・`docs/tasks/T0649-editor-dress-up-preview.md` のメモ

## メモ

- **モジュールの設定:** `Command::SetModuleSettings { name, slot, requires }` を足した (enum の末尾。`OP_COUNT` 99 → 100、codegen の見本、
  `schema/command.schema.json`)。`Document::apply_with_report` が単独で受け、作業用のモデルを今の設定で分けて新しい設定で合成し直す
  (`ModuleContext::reconfigured`)。Undo で設定も戻るよう、`ModuleContext` を `Document` から `Snapshot` に移した。`Batch` の中では拒否。
  画面は `Module:` の印を押して開く `ModuleSettingsDialog.svelte` (名前・スロット・条件のレベル/スロット/パラメータ。取り付け先の違う
  スロットに移すときの注意)。名前を変えると ID の接頭辞が変わるので、選択を付け替える (`renamePrefix`、FileMenu の `onrenamemodule`)。
- **本体のものの無効化:** `BaseLock.svelte` (`fieldset disabled` + 「Edit it in the base project」)。インスペクタ (本体の要素・アバタープロファイル)、
  パラメータ (「+」・定義の編集・本体の要素への ◆)、Physics (World・本体のリグ・本体の衝突体)、Lighting (モデル全体・本体のテクスチャの法線マップ)、
  Slots タブ全体。`ModuleStatus` に本体のテクスチャ・リグ・衝突体・スロット・パラメータ・アバターのレベルと `requires` を足した。
- **ビューポートと言葉:** 本体の要素の選択枠を灰色の破線にし、編集のツール (フォーム・ギズモ・Edit mesh) を出さずに「Base model: can't be edited here…」。
  「Module outline」(既定で入) でモジュールのメッシュの輪郭を点線で重ねる。`ReadOnlyBase` の文言を
  `` … belongs to the base model and can't be edited here. Edit it in the base project `` にした。合成の接頭辞は `labels.ts` の
  `setHiddenIdPrefix` で表記 (インスペクタの見出し・選択欄・テクスチャの名前) から隠す (値の ID はそのまま)。
- **本体の変化:** `ProjectFingerprint::read_base` (`.ugoeproj` はプロジェクトの指紋、`.ugoe` はファイル 1 つ) を `ProjectSession.base_disk` に覚え、
  `check_base_change` を外での書き換えと同じ機会 (focus・5 秒ごと、未保存でも) に呼ぶ。`BaseChangeDialog.svelte` で「Reload the base」→
  `reload_base` (`Document::rebase_module`: 保存していない編集と保存済みかどうかを保ち、Undo の履歴は消す。状態の番号は続ける)。
  読み直せなければドキュメントはそのままで、同じ本体の状態はもう尋ねない。
- **本体の問題の直し方:** `module_open_problem(path, base)` が `ProjectError` から `ModuleProblemKind` (`base_not_found`・`base_unreadable`・
  `unknown_slot`・`does_not_fit`) と本体のスロットの一覧を返し、`ModuleProblemBar.svelte` が「Locate base project… / .ugoe…」「Use this slot」
  (`open_module_with_base` に `slot` を足した。`Project::load_module_with`)・「Open base project」を出す。選び直した本体でも開けなければ、その本体で
  確かめた次の問題を出す。**読み取り専用で開く形は作らなかった** (本体が無いと取り付け先の局所座標を表示できない。editor.md に理由)。
- **プレビューの着せ替え:** `DressUpHost.ownSlot` (PreviewMode が `doc.module.slot.id` を渡す) で、自分のスロットの行を出さず、そのスロットの
  モジュールは付けない (`own_slot` の説明)。T0124 の警告の部分には触れていない (`#reconcile` の付けたい組の絞り込み 1 行と `attach` の最初の判定だけ)。
  main (T0124・T0120・T0710) に rebase 済みで、衝突はなかった。
- **IPC:** `module_open_problem`・`check_base_change`・`reload_base` を足し、`open_module_with_base` に `slot`、`module_open_problem` に `base`
  (ipc.ts・types.ts 更新)。型: `ModuleProblem`・`ModuleProblemKind`・`BaseReload`、`ModuleStatus` の欄。
- **テスト:** editor-core `module_project_tests.rs` に 5 件 (名前の変更と Undo・スロットの移動と条件と拒否・文言・本体の読み直し・`.ugoe` の指紋)、
  Tauri `module.rs` に 1 件 (本体の変化の検出・読み直し・問題の種類・選び直した本体での診断)、Vitest (`moduleProject.test.ts`・`labels.test.ts`・
  `dressUp.test.ts` の自分のスロット)。
- **確かめたこと:** `just ci` (rebase 後。`cargo check -p ugoe-editor` を含む)・`just gen` (差分なし)・`just wasm` (エディタ・ビューア)。
  ヘッドレスの Chrome (SwiftShader) と本物の `Document`/`Project` への使い捨ての HTTP ブリッジ (scratchpad/T0650/bridge。T0648 のものを元に、
  見本のアバター + スロット `Headwear`・`Accessory` の本体と猫耳のモジュール) で: 開く → 本体の要素 (face) を選ぶとインスペクタが無効・
  ビューポートに印・「+」が無効 → モジュールの要素の見出しは `band` (接頭辞なし) → 設定で名前を `Kitty` に (ID が `Kitty/…`、選択が付いてくる)、
  条件に `Accessory` → スロットを `Accessory` に移して Undo で戻る → 本体を外で変える (スロットの範囲) と「The base model changed」→ 読み直すと
  範囲が 965〜1000 に、未保存の変更は残る → 保存後に本体から `Headwear` を消すと読み直せず枠が出る (二度は尋ねない) → 開き直すと
  `unknown_slot` の枠 →「Use this slot」で `Accessory` で開く → 本体を動かすと `base_not_found` → 「Locate base project…」で動かした本体を選ぶと
  `unknown_slot` の枠 (その本体について) →「Use this slot」で開く → `.ugoemod` を書き出し、プレビューで読み込むと `own_slot` の説明で付かない
  (Slots には `Accessory` だけ) → 新規作成の画面の「What you can do」。横のはみ出し 0。
  スクリーンショット: scratchpad/T0650/shots/t0650-*.png (マニュアルの画像は module-base-readonly.png・module-settings.png・module-problem.png)。
  使い終えた vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ (OS のダイアログ・`State` 越しの `reload_base`/`check_base_change` の呼び出し。ロジックは Rust のテスト)。
  取り付け先の違うスロットへの移動 (見本の本体のスロットは両方 `HeadRotation`。テストは空のモジュールで `Hair` → `Outfit`)。本体が `.ugoe` の
  ときの変化の検出 (指紋の単体テストのみ)。`does_not_fit`・`base_unreadable` の枠の画面 (種類の判定はテスト・ロジックは同じ)。
  「Open base project」のボタンの操作。`just test-gpu` (描画・構造体の変更なし) は流していない。
- **範囲外の変更:** なし (editor-core・apps/editor・docs だけ)。
- **後続の候補:**
  - Lighting の「Lit meshes」の本体のメッシュ、物理のリグの並べ替え (▲▼) など、まだ無効にしていない欄がある (変えようとするとバックエンドが拒否する)。
    テンプレートの物理 (TemplateSection) も本体の要素を選べてしまう。
  - スロットを取り付け先の違うスロットに移すとき、見た目を保って移す (取り付け先の座標に写し直す) 選択肢。
  - 本体の変化の自動の読み直し (外での書き換えの「Reload automatically」と同じ設定にするか)。本体が見つからないときに、保存せずに本体を選び直す
    (今は開き直すので、保存していない編集がある間は選べない)。
  - AI のリギングの提案をモジュール向けにする (T0648 の候補のまま。今はできないこととして画面に出すだけ)。
  - モジュールのプロジェクトのプレビューで、自分のスロットのモジュールを書き出したものと差し替えて比べる (今は付けない)。
