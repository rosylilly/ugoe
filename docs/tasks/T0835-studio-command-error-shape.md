---
id: T0835
title: "スタジオ: コマンドのエラーを { code, args, message } の形にそろえる"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0835
---

# T0835: スタジオ: コマンドのエラーを { code, args, message } の形にそろえる

## 目的

エディタとビューア (T0729) の Tauri のコマンドは、失敗を `{ code, args, message }` (ADR 0029) で返し、画面がコードから訳す。
スタジオのコマンドのエラーは、引数が平らに並び (`args` にまとまっていない)、`message` の無いものがある (T0729 の候補)。
同じ形にそろえ、操作ウィンドウと出力ウィンドウの訳し方を共通にする。

## 受け入れ条件

- [x] スタジオのコマンドのエラーがすべて `{ code, args, message }` で返る。形を固定するテストがある
- [x] 画面がコードと `args` から訳す (知らないコードは `message`)。翻訳のキーの検査が通る
- [x] 外部 API (`docs/studio-api.md`) のエラーの形との関係を確かめ、違うなら理由を書いた
- [x] `just ci` が通る

## 参考

- T0729 のメモ (起票の候補)、ADR 0029
- apps/viewer/src-tauri/src/error.rs (T0729 の形)

## メモ

- T0019 で起票した (元の候補: T0729 のスタジオ)。
- やったこと (claude-worker-T0835):
  - `src-tauri/src/error.rs` に `CommandError { code, args, message }` (ビューアの T0729 と同じ形) を置き、スタジオの
    すべての Tauri のコマンドの失敗をこれにした。`Result<_, String>` だったもの (仮想カメラの `start_virtual_camera`・
    `set_virtual_camera_settings`・`stop_virtual_camera`、出力ウィンドウの操作、刻みの速さ、描画の負荷の報告、
    受け手のファイル (`scene_model` など)、操作 API のパネルのコマンド) も含む。ホットキーの知らせ (`Monitor::Hotkey` の `error`)
    も同じ形にした (以前の `HotkeyError`・`InternalError` は消した)。
  - コードは ADR 0029 の書き方 (`profile.not_json`・`scene.model_missing`・`animation.no_model` など) にした。以前の
    camelCase の名前 (`notJson` など) はスタジオの中のコマンドだけで使っていて、バックエンドと画面は同じ版で配るので、
    この機に名前を変えた。一覧は apps/studio/README.md の「コマンドのエラー」。
  - 画面: `src/lib/errors.ts` を書き直し (カタログの `errors.<code>` を `args` で引く。無いコードは `message`。数は今の言語の書式)。
    操作ウィンドウのシーンのエラーの判断はコードの接頭辞 `scene.` で行う。出力ウィンドウとブラウザソースは @ugoe/i18n を
    読まないので、同じ形を `src/lib/commandMessage.ts` の `englishErrorText` で英語の文にする。仮想カメラ・ホットキーの
    パネルも `describeError` を使う。`profile.ts` の `StudioError`・`isStudioError` は消した。カタログ (en・ja) の `errors` を
    コードに合わせて入れ子にした。
  - テスト: error.rs (JSON の形・`asset` の名前・すべてのコードの書き方と、両方のカタログに文があり差し込む引数が `args` に
    あること)、lib.rs (ホットキーの知らせの形)、errors.test.ts (訳・数の書式・知らないコード・シーンのエラーの判断・英語の文)。
  - 外部 API との関係: 操作 API の `-32010` の `error.data` は `ProfileError`・`SceneError`・`AnimationError` をそのまま
    書いた別の形 (平らな引数・camelCase のコード) のまま変えていない。外のツールとの公開の約束で、同時に配られる
    コマンドのエラーとは事情が違うため。理由と対応 (`unknownScene` ↔ `scene.unknown` など) を README に書いた。
    docs/studio-api.md は変えていない。
- やらなかったこと:
  - 実際のアプリ (Tauri の WebView) での表示は確かめていない (vitest・svelte-check・Rust のテストで確かめた)。
  - 仮想カメラの止まった理由 (`Status` の `code`。`virtualCamera.errors.*`) と受け手が読めなかった理由 (`loadFailures`) は
    コマンドのエラーではなく状態なので、この形にしていない (README に書いた)。
- 起票の候補:
  - 操作 API の `-32010` の `data` にも `args` (と ADR 0029 のコード) を足すか (足すだけなら互換を壊さない)。外のツールが
    スタジオと同じ訳を使えるようになる。公開 API の変更なので利用者の判断が要る。
