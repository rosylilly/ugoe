---
id: T0943
title: "MediaPipe の利用状況の送信 (odml.pa.googleapis.com) を送らせない方法を調べ、プライバシーの説明に書く"
status: todo
milestone: M9
crates: [packages/face-tracking, docs]
depends_on: []
assignee:
---

# T0943: MediaPipe の利用状況の送信 (odml.pa.googleapis.com) を送らせない方法を調べ、プライバシーの説明に書く

## 目的

T0029 で、トラッキングが 60 秒を超えると MediaPipe (@mediapipe/tasks-vision 1.0.1) が利用状況を `https://odml.pa.googleapis.com/v1/log` へ
送ろうとし (60 秒ごとと閉じたとき)、アプリの CSP (`connect-src`) が止めていることが分かった。e2e は止まって正しいものとして数えない
(`tests/e2e/lib/harness.mjs` の `EXPECTED_BLOCKS`)。止める設定は見つかっていない (T0029 の候補)。
3 つのアプリと `@ugoe/runtime` を使うページ (CSP の無いホスト) では、送信が止まらない場合がある。

## 受け入れ条件

- [ ] 送信の仕組み (どこで・何を・いつ送るか。ライブラリの設定・`ModuleFactory` の差し替え・ビルドの選択で止められるか) を調べ、結果と出典をメモに書いた
      (MediaPipe のソースはリポジトリの外で読む。AGENTS.md の外部のコードの規則)
- [ ] 送らせない方法があれば `packages/face-tracking` で使い、CSP の違反が出ないことを e2e (`EXPECTED_BLOCKS` から外して) で確かめた。無ければ理由を書いた
- [ ] 送らせられないなら、CSP で止めていること (アプリ) と、ホストのページで止めるための CSP の例 (使う側) を、docs (face-tracking の README・
      docs/manual のトラッキングの章などプライバシーを説明する場所) に書いた
- [ ] `just ci` が通る

## 参考

- T0029 のメモ (CSP・起票の候補)
- tests/e2e/lib/harness.mjs (`EXPECTED_BLOCKS`)、packages/face-tracking

## メモ

- T0038 で起票した (元の候補: T0029 (MediaPipe の利用状況))。
