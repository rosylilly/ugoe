---
id: T0648
title: エディタ: モジュールのプロジェクトと .ugoemod の書き出し
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0647, T0123]
assignee: claude-worker-T0648
---

# T0648: エディタ: モジュールのプロジェクトと .ugoemod の書き出し

## 目的

ADR 0024。本体を参照するモジュールのプロジェクトを作り、衣装を組んで `.ugoemod` に書き出せるようにする。

## 受け入れ条件

- [x] プロジェクトの種類 (本体 / モジュール) と、モジュールのプロジェクトが参照する本体 (`.ugoeproj` か `.ugoe`) とスロット。既存のプロジェクトはそのまま読める (`docs/spec/file-formats.md` を更新)
- [x] `Document` がモジュールを編集できる (パーツ・デフォーマ・メッシュ・グルー・テクスチャ・物理・ポーズ)。モデル用の `Command` をモジュールでも使えるようにするか、モジュール用に分けるかを決めて書く
- [x] 本体を下敷きとして読み取り専用で表示し、合成した姿をビューポートとプレビューで見せる。本体のパラメータで動かせる
- [x] 衣装の PSD の取り込み・自動メッシュ・リギングがモジュールでも使える
- [x] `.ugoemod` の書き出し (`validate_module` と本体との合成の検証)。新規作成の画面 (本体とスロットを選ぶ)
- [x] ヘッドレスのブラウザで、衣装の PSD からモジュールを作って書き出し、本体に付けて確かめる。マニュアルの着せ替えの章に節を書く
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0024-module-authoring.md`、`docs/spec/modules.md`

## メモ

- **方針 (Document がモジュールを編集する方法):** `Document` は本体とモジュールを合成した「作業用のモデル」を持ち、`Command` はモデルのものを
  そのまま使う (モジュール用に分けない)。`Document::with_module(Arc<ModuleContext>)`。適用のたびに `ModuleContext::split` で作業用のモデルを
  モジュールと本体に分け、本体の部分が変わっていれば `EditError::ReadOnlyBase`、表せなければ `EditError::ModuleShape` で拒否する。
  キャンバス直下に作られたモジュールのデフォーマとメッシュは見た目を保って取り付け先の下へ、最上位のパーツはスロットのパーツの下へ移す (`normalize`)。
  理由と詳細は `docs/editor.md#モジュールのプロジェクト` と ADR 0024 の補足 (新しい ADR は書いていない)。
  - ID は合成と同じ `<名前>/<ID>` で読み込み、分けるときに接頭辞を外す (外すと重なる ID は拒否)。描画順は本体の空間で編集し、分けるときに
    0〜1000 に戻す (幅 0 の範囲は 500)。法線マップの名前は合成が付け替えないので、作業用の画像ではここで付け替える。
- **ファイル:** `project.json` に `kind` (`model` 既定・書かない / `module`) と `module { base, slot }`。`base` は保存のたびに相対パスで書く。
  モジュールのプロジェクトは `module.json` (`Module`) と `textures/` だけ。本体は `.ugoeproj` か `.ugoe` (`project::load_base`)。
  `Project::new_module`・`Project::load_with_base`・`export_module` (`validate_module` と `compose`・`compose_physics` を確かめて `.ugoemod`)。
  指紋 (T0641) は `module.json` を含め、本体のファイルは含めない (本体は書かないので失われるものがない。開き直せば新しい本体で合成し直す)。
  既存のプロジェクトはそのまま読める。互換性テストの資料 (`.ugoe`・`.ugoemod`) の形は変えていない。
- **画面:** ファイルメニューに「New Module…」(`NewModuleDialog.svelte`: 本体 → スロット → 名前)、`Module: <スロット>` の印、
  「Export Module…」(モジュールのとき「Export…」の代わり)、本体が見つからないときの「Locate base project… / .ugoe…」。
  ツリーは本体の要素に `base` の印を付けて薄く表示 (`src/lib/moduleProject.ts`、Vitest)。`EditorDocument.module` (`module_status`)。
  Tauri: `src-tauri/src/module.rs` (`module_status`・`read_module_base`・`new_module_project`・`open_module_with_base`・`export_module`)。
  `import_psd` はモジュールのとき描画順をスロットの範囲に写す。App.svelte とプレビューのパネル (T0649) は触っていない。
- **確かめたこと:** `just ci`・`just gen` (差分なし)・`just test-gpu`・Vitest・`cargo check -p ugoe-editor` (ci の中)。main (T0649・T0706) に rebase 済み。
  ヘッドレスの Chrome と本物の `Document`/`Project` への使い捨ての HTTP ブリッジ (scratchpad/T0648、リポジトリ外) で、見本のアバターに
  スロット `Headwear` (取り付け先 `HeadRotation`・パーツ `face`・970〜1000) を足した本体から「New Module…」→ 衣装の PSD (猫耳のカチューシャ。
  ブリッジが ugoe-psd のテスト用の PSD ライターで作った 1024×1024 の 5 レイヤー) の取り込み → 本体の `AngleX`/`AngleZ` で一緒に動くこと →
  本体のメッシュの削除が `ReadOnlyBase` で拒否されること → 保存 (`module.json` と画像 5 枚、`base` は `../base.ugoeproj`) → `.ugoemod` の書き出し
  → ランタイム SDK の `Puppet::attach_module` で本体の `.ugoe` に付けて ugoe-headless で描画、まで。横のはみ出しは 0。
  スクリーンショット: scratchpad/T0648/shots/module-0{1,2,3,3c,4,5}-*.png と attach-{neutral,anglex}.png (マニュアルの画像は module-new.png・module-workspace.png)。
- **確かめていないこと:** 実際の Tauri のウィンドウ。AI の提案 (PSD からのリギング) をモジュールのプロジェクトで使うこと (提案がパラメータを足すと
  `ReadOnlyBase` で採用が拒否されるはず)。推定・ミラー・物理の編集をモジュールで行うこと (同じ `Command` なので通るはずだが画面では試していない)。
  T0649 のプレビューの着せ替えを、モジュールのプロジェクトのプレビューで使ったときの振る舞い (作業用のモデルはすでに合成済みなので、同じスロットに
  別のモジュールを重ねて付けられてしまう)。
- **見本:** `assets/samples` の見本の出力は変えていない (見本のアバターにスロットを入れるなら、team-lead に相談してから。T0647 の候補と同じ)。
- **後続の候補:**
  - モジュールの名前・条件 (`requires`)・スロットを後から変える画面 (今は新規作成のときだけ)。
  - AI のリギングの提案をモジュール向けにする (パラメータを足さず、公開されたパラメータだけを使う)。
  - モジュールのプロジェクトのプレビューでは、T0649 の着せ替えの一覧から自分のスロットを除く (二重に付けない)。
  - 本体の変更に気づく (本体の指紋を取り、下敷きが古いことを知らせて開き直しを促す)。
  - 見本のアバターにスロットと見本のモジュール (この猫耳など) を入れる。
  - インスペクタ・パラメータの画面で、本体の要素の編集の欄を無効にする (今は変えると拒否のメッセージが出る)。
