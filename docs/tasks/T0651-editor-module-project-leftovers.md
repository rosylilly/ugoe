---
id: T0651
title: エディタ: モジュールのプロジェクトの残り (無効にしていない欄・見た目を保つ移動・本体の自動の読み直し)
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0650]
assignee: claude-worker-T0651
---

# T0651: エディタ: モジュールのプロジェクトの残り

## 目的

T0650 の後続の候補をまとめる。

## 受け入れ条件

- [x] まだ無効にしていない本体の欄を無効にする: Lighting の「Lit meshes」の本体のメッシュ、物理のリグの並べ替え (▲▼)、テンプレートの物理 (TemplateSection) で本体の要素を選べること
- [x] スロットを取り付け先の違うスロットに移すとき、見た目を保って移す (取り付け先の座標に写し直す) 選択肢
- [x] 本体の変化の自動の読み直し (外での書き換えの「Reload automatically」と同じ設定にするかを決める)。本体が見つからないときに、保存していない編集があっても本体を選び直せる
- [x] 本体の問題のエラーの文で、ファイル名を文から見分けられるようにする (今は「The base model moved.ugoeproj no longer has slot…」と続けて読めてしまう)
- [x] モジュールのプロジェクトのプレビューで、自分のスロットに今の編集を書き出したものを付けて、ほかのモジュールと差し替えて比べられる (今は付けない)
- [x] ヘッドレスのブラウザで確かめる。マニュアルを更新する。`just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0650-editor-module-usability.md` のメモ

## メモ

- **無効にした欄:** Lighting の「Lit meshes」は本体のメッシュに `base` の印を付けて無効にし、「All on/off」「Light selected」はモジュールの
  メッシュだけを変える (`isBaseMesh`)。Physics のリグの ▲▼ は、モジュールのリグどうしでだけ使える (モジュールのリグは合成で本体のリグの後に
  並び、`split` はモジュールのリグの本体のリグとの前後を保たないため。`canMoveRig`)。「+ From selected deformers」は本体のデフォーマを鎖に入れない。
  TemplateSection は「Create missing parameters」を無効、首の候補から本体のデフォーマを除く (`defaultNeck(model, candidates)`)、本体にあるリグを
  作り直すテンプレートと、本体にない中継パラメータ `BodyBounce` を作る「+ Bounce」を無効にする (`missingRelays`)。「+ Body follow」は
  `BodyAngleX` を持つ本体なら通る (画面で確認)。
- **見た目を保つ移動:** `Command::SetModuleSettings` に `keep_look: bool` (`#[serde(default)]`) を足した (enum は足していない。`OP_COUNT` は 100 のまま)。
  `ModuleContext::keep_look` が、合成し直した作業用のモデルでモジュールの最上位のデフォーマとメッシュの親をいったん前の取り付け先に戻し、
  `deformer_edit::reparent` (ReparentPreservingLook と同じ) で新しい取り付け先へ移す。`rederive`・検証・`split` を確かめてから採用。
  画面は「Keep where it appears」(既定で入。取り付け先が変わるときだけ出す)。物理の粒子・衝突体の位置は写し直さない (後続の候補)。
- **本体の自動の読み直し:** 外での書き換えの「Reload automatically」と同じ設定にした (`baseAction`。本体の読み直しは未保存の編集を保つので、
  未保存でも読み直す。本体が見つからないときは尋ねる)。確認の画面にも同じ欄を出す。理由は editor.md に書いた。
- **開き直さない付け替え:** `reload_base(base, slot)` (IPC の引数を足した) → `Document::rebase_module_on(base, slot)`。本体が見つからない確認の画面の
  「Locate base…」と、読み直せなかったときの問題の枠の「Locate…」「Use this slot」がこれを使い、保存していない編集を保つ (付け替えると未保存の印)。
  開けなかったプロジェクトは従来どおり `open_module_with_base`。
- **問題の文:** 本体のファイル名を引用符で囲む (`The base model "moved.ugoeproj" no longer has slot "Hair".`)。
- **プレビューの比較:** モジュールのプロジェクトのプレビューでは、本体だけの `.ugoe` (`get_preview_base_bundle`、`ugoe_editor_core::export_base_bundle`)
  を描き (`ViewportController.setPreviewModel`)、今の編集の `.ugoemod` (`get_preview_module`、`export_module` と同じ検査) を一覧の先頭の `own` の行
  (`editing` の印) として付ける (`DressUp.setOwn`、`PreviewMode.comparing`)。同じスロットのほかのモジュールと差し替えられ、自分のスロットの行も出る。
  同じ名前のファイルは付けない (`same_name`)。書き出せない間は T0650 の形 (合成したモデル・自分のスロットを除く) に戻り、理由を出す。
  **T0125 との重なり:** `dressUp.svelte.ts` の `loaded` に `#update` の 1 行、`#afterChange` の付けたい組の更新で自分のスロットを除く 1 行、`#load` の
  同じ名前の判定、`#update` の同じ名前の行の扱い。`preview.svelte.ts` の `loaded` の物理の条件 (rebase で T0125 の `physicsReplaced` と衝突し、両方を残した)。
  警告の扱いには触れていない。
- **IPC:** `reload_base` に `base`・`slot`、`get_preview_base_bundle`・`get_preview_module` を足した (ipc.ts)。`just gen` で types.ts・samples.ts・
  command.schema.json を作り直した (`keep_look`)。
- **テスト:** editor-core `module_project_tests.rs` (見た目を保つ移動と保たない移動・`rebase_module_on` でのスロットの付け替え・`export_base_bundle`)、
  Tauri `module.rs` (開き直さない本体とスロットの付け替え・未保存の編集が残る・問題の診断)、Vitest (`canMoveRig`・`isBaseMesh`・問題の文の引用符・
  `baseAction`・`missingRelays`・`defaultNeck` の候補・`DressUp.setOwn` の付け外しと差し替え・同じ名前・比べる前の付けたい組)。
- **確かめたこと:** main (T0125・T0709・T0712・T0126 ほか) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci` (`cargo check -p ugoe-editor` を
  含む)・`just test-gpu`。ヘッドレスの Chrome (SwiftShader) と本物の `Document`/`Project` への使い捨ての HTTP ブリッジ (scratchpad/T0651/bridge。T0650 の
  ものに `reload_base` の引数・プレビューの 2 つ・見本の `.ugoemod` からモジュールのプロジェクトを作る `from-module` を足した) で、着せ替えの見本
  (`ugoe sample dress-up`) の本体 `.ugoe` と CatEars のモジュールのプロジェクト: Lit meshes (13 行のうち本体の 11 行が無効、All off でモジュールだけ)
  → Physics (リグ 2 つの ▲▼、テンプレートの欄。+ Body follow は通る) → プレビュー (本体 + CatEars `editing`、BunnyEars (名前を変えた見本) と差し替え・
  戻す、同じ名前の cat-ears.ugoemod は `same_name`、Sailor を重ねる、出て入り直しても付けたい組が残る) → CatEars を Outfit (BodyWarp) へ「Keep where it
  appears」で移すと耳は頭の上に残る / 外すと首元へずれる、Undo → 本体の `.ugoe` を外で変える → 確認の画面で「Reload automatically」→ 2 回目は
  尋ねずに読み直す → 本体を動かす → 「The base model is gone」→「Locate base .ugoe…」で Headwear のない本体 → 枠に
  `The base model "no-headwear.ugoe" no longer has slot "Headwear".` →「Use this slot」(Hair) で未保存の編集 (リグ) を保ったまま付け替え。横のはみ出し 0。
  スクリーンショット: scratchpad/T0651/shots/t0651-*.png (マニュアルの画像は module-keep-look.png・module-compare-preview.png・module-problem.png (撮り直し))。
  使い終えた vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ (OS のダイアログ・`State` 越しの呼び出し)。プレビュー中の編集で今の編集を取り直すこと (プレビュー中は
  Undo のボタンが無く、Ctrl+Z でも取り直しは起きなかった。取り直しは Vitest の `setOwn(…, true)` と、本体の読み直し・MCP の編集で `revision` が変わった
  ときの経路)。本体に物理のリグがあるときの ▲▼ の画面 (見本の本体にリグが無い。規則は Vitest)。「Open base project」のボタン。`.ugoeproj` の本体での
  自動の読み直し (ロジックは同じ)。大きな本体で、プレビュー中の編集ごとに本体だけのバンドルを作り直す重さ。
- **範囲外の変更:** なし (editor-core・apps/editor・docs だけ)。
- **後続の候補:**
  - 見た目を保つ移動で、モジュールの物理の粒子・衝突体 (取り付け先に付くもの) も写し直す。
  - プレビューの比較で、本体だけのバンドルを本体の状態が変わったときだけ作る (今は編集ごとに作ってバイト列を比べる)。
  - プレビューの比較で、今の編集と前に書き出した同じ名前の `.ugoemod` を並べて比べる (今は同じ名前は付けない)。
  - 本体に物理のリグがある見本 (T0712 のプロジェクト出力) で ▲▼ とテンプレートの画面を確かめる。
