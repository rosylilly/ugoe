---
id: T0723
title: ビューア: 共有部品への移行と日本語・英語の切り替え
status: done
milestone: M8
crates: [apps/viewer, packages/player, docs]
depends_on: [T0901, T0903, T0904, T0908]
assignee: claude-worker-T0723
---

# T0723: ビューア: 共有部品への移行と日本語・英語の切り替え

## 目的

ビューアを @ugoe/ui と @ugoe/i18n に移し、日本語と英語を切り替えられるようにする。

## 受け入れ条件

- [x] 画面の部品を @ugoe/ui に置き換え、独自の部品と CSS を減らす
- [x] すべての文言をカタログに移し、英語の訳を付ける。設定に言語の切り替え (既定は OS の言語) を置き、覚える。OS の言語は tauri-plugin-os の locale() で取り、@ugoe/i18n の setSystemLanguages() に渡す (WebView の navigator.languages は OS の言語を正しく返さない。利用者の判断、2026-10-02)
- [x] T0908 の検査で、ビューアの許す一覧が空になる
- [x] ヘッドレスの Chrome で両方の言語の画面を確かめる。`just ci`・ビューアのビルドが通る

## 参考

- apps/viewer/README.md、docs/ui.md

## メモ

- **範囲の追加 (team-lead の指示):** `packages/player` (62 件の日本語の文言を名前空間 `player` のカタログへ移す) を crates に足した。
  ほかに、生成物の `Cargo.lock`・`pnpm-lock.yaml` と、`scripts/hardcoded-strings-allowlist.json` (`--update` で viewer と player の行が消えた) が変わる。
  ルートの `Cargo.toml` の `tauri-plugin-os` はスタジオ (T0801) が足したものを使い、触っていない。
- **やったこと:**
  - 画面を `@ugoe/ui` に移した: ツールバー (Button・Menu・Toolbar・IconButton・ColorPicker・Toggle・Tooltip)、パネル (Section・Slider・
    Toggle・Checkbox・SegmentedControl・Select・Notice)、ダイアログ (Dialog・AlertDialog。「すべて忘れる…」は確かめのダイアログに)、
    画像の保存の知らせ (Toast)。`app.css` はトークンを使う数行と、移していない face-tracking のパネルのための別名 (下) だけ。
    強調色は紫、密度は comfortable。光源の面 (LightPad)・光源の印・パラメータの絞り込みの欄は @ugoe/ui に部品が無いので、自前のままトークンで装った。
  - 文言: ビューアは名前空間 `viewer` (`apps/viewer/locales/`)、player は `player` (`packages/player/locales/`)。原文は英語、
    日本語の訳は前の表示と同じ文 (player のテストは `setLocale("ja")` で前の文と一致することを確かめ、英語の文のテストも足した)。
    player の定数の一覧 (`MOTION_PRIORITIES`・`TRACKING_GROUPS`・`PRESETS`) の名前は getter で読むたびに引く (言語の切り替えに追従)。
    一覧の区切り (「・」「、」) は `formatList`、日時は `formatDate`、百分率は `formatNumber`。
  - 言語の切り替え: ツールバーの歯車 → 設定のダイアログに「言語」(OS に合わせる・日本語・English)。`ugoe-viewer.locale` に覚える。
    OS の言語は tauri-plugin-os の `locale()` (`ViewerHost.systemLocale`、権限 `os:allow-locale`) で取り、`setSystemLanguages()` に渡す。
    最初の画面から OS の言語で出すよう、`main.ts` で取ってから描く。
  - 許す一覧: viewer 12 ファイル 230 件・player 7 ファイル 62 件 → 0 (一覧から消えた)。
  - 狭い幅 (720px 未満) ではパネルをビューポートの下に置き、縦が足りないときはモデルの機能ごとのパネルが縮んで中を送るようにした
    (comfortable で文字が大きくなり、前の作りではパネルがステータスバーに重なったため)。
  - `public/third-party-notices.txt` に、バンドルに入る JS のライブラリ (svelte・bits-ui とその依存・i18next・@tauri-apps/*) と MIT の本文を足した
    (ソースマップから入っているパッケージを数えた。svelte・i18next・@tauri-apps は前から入っていたのに載っていなかった)。
- **確かめたこと:** `just ci`、`pnpm --filter @ugoe/viewer build`、`cargo build -p ugoe-viewer`。ヘッドレスの Chrome (SwiftShader の WebGL2、
  ビルドしたものを vite preview) で、英語と日本語 × ライト・ダークの 12 場面ずつ (空・サンプルのメニュー・アバター・ライティング・着せ替え・
  モーション・覚えたもの・すべて忘れるの確かめ・設定・ライセンス・360px 幅) を撮った。英語の画面に日本語の文字が残っていないこと
  (テキストと aria-label・title・placeholder・aria-valuetext)、`<html lang>`、スクリプトのエラーが無いこと (favicon の 404 を除く)、
  設定で英語 → 日本語 → OS に合わせるへ切り替えるとその場で変わり覚えることを確かめた。スクリプトと画像は scratchpad の `T0723/`
  (`shots.mjs`・`run.sh`・`shots/`)。
- **確かめていないこと:** Tauri の実物 (WebKitGTK・WebView2・WKWebView) での表示と、tauri-plugin-os の `locale()` が返す値
  (Windows・macOS は手元に無い)。スクリーンリーダーでの読み上げ。本物のカメラ。
- **変えたふるまい:** サンプルのメニューは名前だけ (説明は `Menu` の項目に出せないので出さない)。画像を保存した場所はステータスバーではなく Toast。
  パラメータの行の ID のツールチップ (`title`) は外した (絞り込みは ID でもできる)。光源の横・縦・手前・範囲のスライダーは % で出す。
  面と印の表示の色はトークンの外のまま (描画の中身。docs/ui.md の例外)。`screenshot.ts` の例外の文 (起きないはずの画素の数の食い違いなど) は日本語のまま。
- **後続の候補:** face-tracking のパネルを @ugoe/ui に移し、`app.css` の古い変数の別名と標準のボタンの装いを消す (T0907 か face-tracking の移行)。
  `Menu` の項目に説明の行を出せるようにする (サンプルの説明を戻す)。`packages/i18n/README.md` の「ビューアが `setLocale("ja")` を呼ぶ」の記述が古い
  (範囲外なので触っていない)。player の状態に入れた文 (エラー・警告) を、コードと引数で持って描くときに訳す形にする (言語を切り替えても前の言語のまま残るため)。
