---
id: T0730
title: スクリーンショットの PNG の変換と保存の重複をまとめる (エディタ・ビューア)
status: done
milestone: M9
crates: [apps/viewer, apps/editor, packages/player, docs]
depends_on: [T0724]
assignee: claude-worker-T0730
---

# T0730: スクリーンショットの PNG の変換と保存の重複をまとめる (エディタ・ビューア)

## 目的

スクリーンショット (T0420) の処理が、エディタとビューアにほぼ同じ形で 2 つある。

- 画面: `apps/viewer/src/lib/screenshot.ts` と `apps/editor/src/lib/viewport/screenshot.ts` (`checkScreenshot`・`encodePng`・
  `PngCanvas` などの型・`PNG_PATH_HEADER`。既定のファイル名もほぼ同じ)
- バックエンド: ビューアの `write_png_file`・`percent_decode` (`apps/viewer/src-tauri/src/lib.rs`) と `apps/editor/src-tauri/src/screenshot.rs`

片方だけ直る (例: T0724 でビューアの例外の文を訳した。エディタは英語の固定の文) ことを避けるため、画面の分を共有のパッケージ
(`Screenshot` を持つ `@ugoe/player` など) に寄せる。バックエンドの分は、共有できる場所があるか (GPL のアプリで共有するクレート) を決める。
コードレビュー (T0724) で見つけた。

## 受け入れ条件

- [x] PNG への変換と画素の検査を 1 か所にし、エディタとビューアがそれを使う (例外の文はカタログで訳す)
- [x] バックエンドの PNG の書き込みとパスのヘッダーの扱いを共有するか、しない理由を README に書く
- [ ] 既存のテストを移し、両方のアプリで画像の保存が動く
- [x] `just ci` が通る

## 参考

- T0724 のメモの所見の表
- apps/viewer/README.md#画像の保存-t0420

## メモ

### T0730 の作業 (claude-worker-T0730)

**やったこと**

- `packages/player/src/screenshot.ts` を作り、`Screenshot` の型 (player.svelte.ts から移した)・`PngCanvas` などの型・`checkScreenshot`・
  `encodePng`・`timestampedPngName` (既定の名前の `-<年月日>-<時分秒>.png` の部分)・`PNG_PATH_HEADER` をまとめた。例外の文は
  `@ugoe/player` のカタログ (`screenshot.invalidSize`・`pixelCount`・`noCanvas`・`encodeFailed`。ビューアのカタログから移した) で訳す。
  これでエディタの例外の文も画面の言語に従う (前は英語の固定の文)。
- ビューア: `src/lib/screenshot.ts` は既定のファイル名 (モデル名の整え方) だけにし、`App.svelte`・`host.ts` は `@ugoe/player` から import。
- エディタ: `package.json` に `@ugoe/player` を足し (ADR 0029 の向き `apps/* → @ugoe/player` の内)、`viewport/screenshot.ts` は
  既定のファイル名と `PNG_FILTER` だけにした。`saveScreenshot.ts`・`controller.svelte.ts`・`ipc.ts` は `@ugoe/player` から import。
  `vite build` で、Player 本体は木の剪定で落ち、変換とカタログだけが入ることを確かめた。
- テスト: 画素の検査と変換のテストは `packages/player/src/screenshot.test.ts` に移し (日本語・英語の文も)、両アプリのテストは
  既定のファイル名だけを確かめる。
- バックエンドは共有しない。理由は `apps/viewer/README.md#画像の保存-t0420` (共有の GPL のクレートが無い・エラーの型が違う・
  書けるパスの決め方が違う)。`docs/editor.md#スクリーンショット`・`packages/player/README.md`・`docs/ui.md` の表も直した。
- `crates:` の外: `pnpm-lock.yaml` (エディタの依存を足したため) だけ。

**やらなかったこと・確かめていないこと**

- 受け入れ条件の「両方のアプリで画像の保存が動く」は、単体テスト・svelte-check・エディタの `vite build` までで、Tauri のアプリを
  起動して実際に保存するのは確かめていない (画面の無い環境のため)。保存の流れ (ダイアログ・`write_png_file`) のコードは変えていない。
- エディタの `PNG_FILTER` の名前 (`PNG image (.png)`) は英語の固定のまま (このタスクの範囲の外)。

**起票の候補**

- アプリのバックエンドで共有する GPL のクレート (例: `apps/shared-backend`) を作り、`atomic_write.rs` (ビューアとエディタ)・
  `write_png_file` の中身 (パーセント符号の復号・PNG の検査) を移す。
- エディタの保存のダイアログのフィルタ名をカタログで訳す。

