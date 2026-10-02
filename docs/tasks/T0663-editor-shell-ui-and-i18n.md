---
id: T0663
title: エディタ: 外枠 (メニュー・ツールバー・ダイアログ・ヘルプ) の共有部品への移行と日本語・英語
status: done
milestone: M8
crates: [apps/editor, docs, packages/i18n/README.md]
depends_on: [T0901, T0903, T0908]
assignee: claude-worker-T0663
---

# T0663: エディタ: 外枠 (メニュー・ツールバー・ダイアログ・ヘルプ) の共有部品への移行と日本語・英語

## 目的

エディタの外枠を @ugoe/ui と @ugoe/i18n に移す。パネルは T0664・T0665 で行う。

## 受け入れ条件

- [x] App.svelte・FileMenu・各ダイアログ・ヘルプ・ショートカット・知らせの帯を共有部品に置き換え、文言をカタログに移す (日本語の訳を付ける)
- [x] 設定に言語の切り替え (既定は OS の言語) を置き、覚える。OS の言語は tauri-plugin-os の locale() で取る (T0723 と同じ。利用者の判断)
- [x] パネルの移行で使う、エディタの中の共通の形 (パネルの見出し・行のレイアウト) を決めて docs/ui.md に書く
- [x] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- docs/editor.md、docs/ui.md

## メモ

- **やったこと:**
  - 外枠を @ugoe/ui に移した: ヘッダーのボタン (`TipButton` = Button + Tooltip。Undo / Redo に `Mod+Z` などの表記)、ファイルの操作
    (ghost の Button の並び。取り込みの警告は Popover)、ヘルプ (失敗は Notice)、右の列のタブ (`lib/ui/TabStrip.svelte`。APG の Tabs の
    矢印・Home・End、説明は Tooltip、AI の新しい提案は attention の印と読み上げ。team-lead に相談した A (packages/ui の Tabs に tooltip) は不要にした)、
    ダイアログ (BaseChangeDialog・ExternalChangeDialog・ModuleSettingsDialog・NewModuleDialog を Dialog・Checkbox・Select・Button に)、
    知らせ (保存・書き出しの結果は Toast、失敗はヘッダーの下の知らせの帯の Notice、ModuleProblemBar も Notice に)、
    保存していない変更を捨てる確かめを「同じボタンを 2 度押す」から AlertDialog に変えた (docs/editor.md・マニュアルの warnings.md も直した)。
    `app.css` は古い変数をトークンの別名にし、`tokens.css` を main.ts で読む。`<html data-density="compact">`。
  - 文言をカタログ `apps/editor/locales/{en,ja}.json` (名前空間 `editor`、`src/lib/i18n.ts`) に移した。`moduleProject.ts`・`emptyState.ts`・
    `externalChange.ts` の文は関数にして呼ぶたびに `t()` を通す (`BASE_EDIT_HINT`・`EMPTY_*_HINT`・`MODULE_CAN*` を関数に。使っている
    `panels/lighting/LightingPanel.svelte`・`panels/TreePanel.svelte` は 1 行ずつ付け替えた)。
  - エラーの表示: `errorText` / `messageText` が `errors.<code>` をカタログから引く (T0666 のコード 203 個をすべて英語・日本語で訳した)。
    カタログに無いコードは `message`。識別子の引数 (`kind`・`what`・`expected`・`actual`・`target_kind`) は `errorArgs.<値>` で訳し、
    `cause` は `errorCause` (「… 原因: …」) で後ろに付ける。取り込みの警告・本体の問題の詳細も同じ関数で出す。
  - 言語の設定: 設定 (SettingsDialog) に「General」のタブを足し、`LanguageSetting.svelte` (SegmentedControl。設定の画面は `<dialog>` の
    最上位の層にあり、body に出す Select の一覧が隠れるため) で「OS に合わせる・English・日本語」。`localStorage` の `ugoe-editor.locale` に覚える
    (`lib/locale.ts`)。起動時に tauri-plugin-os の `locale()` を `setSystemLanguages()` に渡す (`tauri-plugin-os` をエディタの Cargo.toml と
    `lib.rs`、`@tauri-apps/plugin-os` を package.json に足し、権限 `os:allow-locale`)。ヘッダーの「Settings」は General を、AI の提案の
    パネルからは AI provider を開く。
  - docs/ui.md に「エディタの中の共通の形」(見出し・行 `.ed-field`・ボタンの並び `.ed-buttons`・補足 `.ed-hint`・TipButton・TabStrip・
    知らせ・BaseLock・確かめ・ダイアログ・文字の欄・要素の名前、キーの最初の段、`<dialog>` と body に出す部品の注意) を書いた。
    `.ed-*` と見出し `<h2>` の形は `app.css` に入れた (h2 は大文字化と字間をやめ、色と大きさと太さにした。パネルの h2 も変わる)。
  - Vitest: テストは英語で検査する (`src/test-setup.ts` で `setLocale("en")`)。`errors.test.ts` (訳・未知のコード・識別子の訳・複数形・原因)、
    `locale.test.ts` を足した。T0908 の許す一覧を 9 ファイル 169 件縮めた (`--update`)。
  - 確かめたこと: `just ci`・`cargo check -p ugoe-editor`。画面は、ビルドした画面をヘッドレス Chrome で開き、IPC を本物の ugoe_editor_core
    (着せ替えの見本のプロジェクト) に中継する使い捨てのブリッジ (scratchpad の `T0663/bridge`・`shots.mjs`・`run.sh`) で、OS の言語を
    en-US と ja-JP にして撮った (scratchpad の `T0663/shots/`。起動・モデル・タブのツールチップ・設定・捨てる確かめ・開けなかった帯
    (日本語の訳で出る)・保存の Toast・外での書き換え・モジュールの印と設定・新しいモジュール・言語の切り替え)。
- **確かめていないこと:** 本物の Tauri のウィンドウ (WebView2・WKWebView・WebKitGTK) での表示と、`locale()` が返す値 (OS ごと)。スクリーンリーダーでの
  読み上げ。本体の問題の帯 (ModuleProblemBar) とヘルプの失敗の知らせは画面で出していない (Vitest も無い)。狭い幅 (1024px) での折り返し。
- **やらなかったこと:** パネルの中 (panels/・viewport の道具・ai/・agents/・ExportPanel・exportSummary.ts・preview/) は T0664・T0665。
  SettingsDialog は General のタブを足しただけで、ほかの文言と `<dialog>` は T0665 で移す (見出しの「Settings」や「Close」は英語のまま)。
  ファイルの操作を「File」のメニュー (Menu / Menubar) にまとめることはしていない (見た目の大きな変更なので、ボタンの並びのまま)。
- **範囲外の変更:** ルートの `Cargo.lock` と `pnpm-lock.yaml` (依存の追加)、`scripts/hardcoded-strings-allowlist.json` (縮めただけ)。
  `packages/i18n/README.md` の「エディタは … `setLocale("en")` を `main.ts` で呼び」が古くなったが、範囲外なので直していない。
- **後続の候補:**
  - `packages/i18n/README.md` の上の 1 行を「エディタは設定の General で切り替える (T0663)」に直す。
  - マニュアル (`docs/manual/`) の画面の名前とエラーの文: 英語の画面の名前で書いているので、日本語の画面と食い違う。warnings.md の英語の文も、
    カタログの英語 (バックエンドの文と少し違う) と食い違う。
  - ファイルの操作を「File」のメニューにまとめる (ヘッダーが日本語で折り返しやすい。1440px でも未保存の印が付くと 2 行になる)。
  - @ugoe/ui に文字の入力欄の部品 (TextField) を足す (いまはダイアログの中だけ `app.css` で装う)。
  - 保存のショートカット (Ctrl + S) は無い (ツールチップにも出していない)。
- **review の後の手直し (team-lead の指摘。画面確認の手伝いの報告の 1・3・4):**
  - 設定のダイアログの外枠を移した: 題・タブ (TabStrip。「一般・AI のプロバイダ・エージェントの接続」)・閉じるボタン (@ugoe/ui の Button) を
    カタログで訳し、見た目をトークンで Dialog にそろえた。要素は `<dialog>` のまま (中から外部の AI に送る前の確認の `<dialog>` (ConsentDialog) を開くので、
    Bits UI の Dialog のフォーカスの閉じ込めと重なると操作できなくなるおそれがある。T0665 で中身と ConsentDialog と一緒に Dialog へ移す)。
  - ヘッダーを 1 行に収めた: ファイルの操作の並びが残りの幅を使い、プロジェクトの名前の欄の幅の基準を 0 にして省略する (最小 4em)。
    ヘッダーの ghost のボタンの左右の余白を詰めた。日本語・英語、モデルとモジュールのプロジェクト、未保存の印ありで 1440px の 1 行を撮って確かめた。
  - 失敗の知らせの帯は、重ねずにヘッダーの最後の行に入れた (ヘッダーが折り返して高くなり、ビューポートを下げる)。Screenshot などのボタンを隠さない。
  - 上書きの「保存」でも「… に保存しました」の Toast を出す。あわせて、保存が外での書き換えで断られたときの判定を直した (前は失敗の文字列を
    `isExternalChangeError` に渡しており、常に偽だった。T0666 の後から)。
  - 言語の設定のキーを、ビューア (`ugoe-viewer.locale`)・スタジオにそろえて `ugoe-editor.locale` にした。OS の言語の取り方 (tauri-plugin-os の
    `locale()` → `setSystemLanguages()`) はビューアと同じ。
  - `packages/i18n/README.md` のエディタの 1 行を直した (team-lead の承認。crates: に足した)。
  - エラーのカタログの扱い (下書きの手伝いの報告): kind の値は docs の 4 つより多い 18 種類あり、すべて `errorArgs` に訳を置いた
    (表に無い値はそのまま出す)。`target`・`element`・`children` の値は ``mesh `EyeL` `` のような英語の表記で、そのまま差し込む (訳さない。
    後続で種類と ID を別の引数にすれば訳せる)。`ai.rate_limited` の `retry_after` と `ai.refused` の `category` は省かれることがあるので、
    文に使わない (省かれても文が成り立つ)。`ai.api_error` の `kind`・`compile.unsupported` の `feature`・`edit.unsupported_interpolation` の
    `interpolation`・PSD の合成モードはそのまま出す。
  - 後続の候補に足す: TabStrip が 2 つ目のアプリでも要るようになったら @ugoe/ui へ移す。ビューポートの「Edit mesh」「Form edit」「Screenshot」
    「Fit」「Module outline」は T0664 で移す (T0663 では触らない)。
  - 撮り直し: scratchpad/T0663/shots/ の ja-04・ja-05・ja-07・ja-11 と en の同じ場面 (ほかの場面も撮り直した)。

