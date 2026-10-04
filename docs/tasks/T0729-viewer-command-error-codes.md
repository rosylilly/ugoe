---
id: T0729
title: ビューアのコマンドのエラーをコードで返し、画面で訳す
status: done
milestone: M9
crates: [apps/viewer]
depends_on: [T0724]
assignee: claude-worker-T0729
---

# T0729: ビューアのコマンドのエラーをコードで返し、画面で訳す

## 目的

ADR 0029 は「Tauri のコマンドのエラーは `code`・`args`・`message` で渡し、画面で訳す」と決めているが、ビューアのコマンド
(`apps/viewer/src-tauri/src/lib.rs`) は英語の文 (`format!("{}: file is too large ...")` など) を `String` で返し、画面はそれを
そのまま出す (ドロップで読めなかったものの知らせの `detail`、画像の保存の失敗、トラッキングの記録の保存の失敗)。
日本語の画面に英語の文が混ざる。コードレビュー (T0724) で見つけた。

## 受け入れ条件

- [x] コマンドのエラーを ADR 0029 の形 (`code`・`args`・`message`) で返す。コードの例: `file.not_model`・`file.too_large`・
  `file.not_found`・`file.read_failed`・`file.not_json`・`file.not_png`・`file.write_failed`・`request.missing_path`
- [x] 画面 (`host.ts` と、エラーを出すところ) が `errors.<code>` を引いて訳す (カタログは en と ja)。カタログに無いコードは `message`
- [x] エディタとスタジオのエラーの扱い (どこで訳すか) と揃える
- [x] テストがある (Rust のコードと引数、画面の訳)
- [x] `just ci` が通る

## 参考

- docs/adr/0029-shared-ui-and-i18n.md (エラー)
- T0724 のメモの所見の表

## メモ

- **やったこと:**
  - バックエンド: `src-tauri/src/error.rs` に `CommandError` (`code`・`args`・`message`) を置き、すべてのコマンドのエラーをこれにした。
    コードは `file.not_model`・`file.too_large` (`size`・`max`)・`file.not_found`・`file.read_failed`・`file.write_failed`・
    `file.not_json`・`file.not_png`・`file.not_chosen`・`file.not_chosen_for_save`・`file.unsupported_save_type`・
    `request.not_png_data`・`request.missing_path`・`request.invalid_path`・`dialog.failed`・`internal` (別スレッドの失敗。スタジオ T0819 と同じ)。
    一覧は README の「コマンドのエラー」。`serde` (ワークスペースに既にある) を ugoe-viewer の依存に足した。
  - 画面: `src/lib/errors.ts` (エディタの `lib/errors.ts` と同じ扱い。`errors.<code>` を `args` で引き、無ければ `message`。数の引数は
    `formatNumber`)。`host.ts` はコマンドの失敗を `CommandError` (`Error`、`message` は今の言語の文) にして投げ直すので、
    `@ugoe/player`・`@ugoe/face-tracking` (範囲の外) は変えずに訳した文を出す。`App.svelte`・`DressUpPanel`・`LicenseDialog`・`drop.ts` は `errorText()`。
  - ドロップの知らせ: 読めなかったものが 1 つなら理由 (訳した文) も出す (`drop.skipped.unreadableReason`)。今まで `detail` は作るだけで出していなかった。
  - T0150 の `format.chunk_limit_exceeded`・`format.total_limit_exceeded` の訳をカタログに置いた (エディタの訳と同じ文)。
- **やらなかったこと:** wasm (`ugoe-wasm`) の読み込みのエラーは英語の文 (`JsError`) のままなので、上の `format.*` のコードはまだビューアに届かない
  (範囲の外)。スタジオのエラーは `{ code, ...フィールド }` の平らな形で、エディタ・ビューアの `{ code, args, message }` と違う (ADR 0029 の形はこちら)。
- **起票の候補:**
  - ugoe-wasm の読み込みのエラー (`load_bundle`・`load_json`) を ADR 0029 の形 (コード・引数) で返し、`@ugoe/player` が訳す
    (ビューアとスタジオの `format.*` の訳が効くようになる)。
  - スタジオのコマンドのエラーを `{ code, args, message }` の形に揃える (いまは引数が平らで、`message` の無いものがある)。

