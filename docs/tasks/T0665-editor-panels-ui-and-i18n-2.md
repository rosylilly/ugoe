---
id: T0665
title: エディタ: パネルの移行 2 (物理・ライティング・スロット・ポーズ・グルー・検証・AI・エージェント・書き出し・プレビュー)
status: done
milestone: M8
crates: [apps/editor, docs]
depends_on: [T0663]
assignee: claude-worker-T0665
---

# T0665: エディタ: パネルの移行 2 (物理・ライティング・スロット・ポーズ・グルー・検証・AI・エージェント・書き出し・プレビュー)

## 目的

T0663 の続き。残りのパネルを共有部品と翻訳に移す。T0664 と並行できるよう、触るディレクトリを分ける。

## 受け入れ条件

- [x] panels/physics・lighting・slots・poses・glues・verify、ai・agents、ExportPanel、preview を共有部品に置き換え、文言をカタログに移す
- [x] T0908 の検査で、エディタの許す一覧が空になる (T0664 と後から入る方が空にする)
- [x] `just ci` が通る

## 参考

- T0663 のメモ

## メモ

- **やったこと:**
  - panels/physics・lighting・slots・poses・glues・verify、ai (提案のパネル・設定のダイアログ・送る前の確認)・agents (接続の画面・承認の帯)、
    ExportPanel・exportSummary、preview (着せ替え・違いの重ね・ライセンス) の直書きの文言を、エディタのカタログ (最上位のキー `physics`・`verify`・
    `lighting`・`slots`・`poses`・`glues`・`preview`・`export`・`proposals`・`agents`・`consent` と `settings.ai.*`) へ移し、日本語を欠けなく訳した。
    .ts の文 (表の label・進み具合・送れない理由・規約のリンクの名前など) は関数か getter にして呼ぶたびに `t()` を通す。
    シェルのコマンドの例・製品名・モデル ID の例には `i18n-ignore` と理由。
  - 生の button・checkbox・range・number・select と `title` を @ugoe/ui (Button・IconButton・Checkbox・Toggle・Slider・NumberField・Select・
    Section・Notice・Tooltip・ColorPicker) と TipButton・TabStrip に置き換えた。すぐ効く入・切は Toggle (docs/ui.md の約束)。
    光源の色は `LightColor.svelte` (ColorPicker を包み、選んでいる間は表示だけ、閉じたら残す)。古い変数と色の直書きはトークンへ。
  - 設定のダイアログを `<dialog>` から Dialog (lg) に、送る前の確認を AlertDialog (`danger={false}`。開くとフォーカスは取り消し) に、
    LicenseDialog を Dialog に移した。「一般」のタブ (言語・テーマ) はそのまま。入れ子 (設定の「接続を確かめる」→ 確認) は Bits UI の
    大域の層で上だけが Esc・フォーカスを受ける。確かめのボタンを待つ間に無効にするとフォーカスが body に落ちたので、待つ間もそのボタンは
    無効にしない (押し直しは無視)。docs/ui.md に「重なるダイアログ」と h3 を書いた。
  - main への rebase (T0664・T0909) の後、許す一覧を `--update` で空 (`{}`) にした。T0664 の `useBaseLock()` を法線マップのスライダーに使った
    (グルーは `locked` を明示で渡している)。
  - 確かめたこと: `just ci`・`cargo check -p ugoe-editor`。画面は T0663 の使い捨てのブリッジを写して (scratchpad の `T0665/bridge`・`shots.mjs`・`run.sh`。
    送る前の確認を返す `test_ai_connection` などの代わりを足した)、英語と日本語で撮った (scratchpad の `T0665/shots/`: 各タブ・書き出し・プレビュー・
    設定の 3 タブ・確認の入れ子と Esc の後・モジュールのプロジェクトの 4 タブ)。入れ子は、確認を開くとフォーカスが「Cancel」、Tab で外に出ない、
    Esc で確認だけ閉じて「接続を確かめる」にフォーカスが戻り、もう一度 Esc で設定が閉じることを記録した。
- **見た目と操作の変化 (主なもの):** チェックボックス → Toggle (すぐ効くもの)。`title` の長い説明を見える補足 (`.ed-hint`) にした所がある。
  設定のダイアログの幅 640 → 720px と右上の ✕。確認のダイアログの幅 480 → 360px (AlertDialog は sm 固定)。プレビューのパラメータの
  ダブルクリックでのリセットは無くなった (リセットのボタンはある)。英語の文が少し変わった所 (「1 particle」の単数形・一覧の `formatList`)。
  Select の項目ごとの説明 (`option` の title) は出す場所が無く消えた。
- **確かめていないこと:** 本物の Tauri の WebView での表示・スクリーンリーダー。320px の右の列で長い行が切れないかは撮った範囲でしか見ていない
  (プレビューと各パネルの下の方はスクロールして撮っていない)。入れ子のダイアログの Vitest は無い (エディタのテストに DOM の環境が無い)。
  検証のパネルの見つかった問題の一覧・提案の一覧・エージェントの承認の帯・着せ替えでモジュールを読み込んだ状態は、ブリッジに中身が無く撮っていない。
- **置き換えなかった所:** 文字の入力欄 (名前・ポート・検索・注文の textarea・API キー)、一覧の行の選ぶボタン、空と placeholder を持つ数値の欄
  (グルーのキー)、行の中のメッシュのチェックボックス (隣の名前のボタンと別の操作)、コマンドの一覧の行の indeterminate のチェックボックス、
  プロバイダの 2 択のラジオ (ラベルが長い)。フォーカスを受けない要素や Select・NumberField を包む span の `title` (訳した文言) は残した (T0907 の見直し)。
- **範囲外の変更:** `scripts/hardcoded-strings-allowlist.json` (縮めて空にした)。
- **後続の候補:**
  - ExportPanel を開く「書き出し…」は自前の開閉で、Esc や外のクリックで閉じない (前から)。Popover に移す。
  - @ugoe/ui に TextField と、Select・NumberField に tooltip の口 (`title` を消すため)。AlertDialog に幅の指定。
  - 着せ替えの違いの文 (`DifferenceItem.text`) はバックエンドの英語のまま。コードと引数にすれば訳せる。
  - 書き出しの要約・着せ替えの知らせ・読み込みの警告は作った時の言語の文字列で残る (言語を切り替えても次に作り直すまで変わらない)。

