---
id: T0985
title: "ui-audit: ダイアログ・メニュー・Popover などを待つ 5 秒の上限を、混んだ機械でも揺れない 1 つの上限にそろえる"
status: done
milestone: M8
crates: [tests/ui-audit, docs]
depends_on: []
assignee: claude-worker-T0985
---

# T0985: ui-audit の待ちの上限をそろえる

## 目的

T0884 で、スタジオの自動の保存の帯と承認の知らせを待つ上限を 5 秒から 20 秒 (`NOTICE_TIMEOUT_MS`) にした。ほかの場面にも、
ダイアログ・確認・メニュー・Popover・ハイライトなどを `waitFor({ timeout: 5000 })` で待つ所が十数か所ある (`tests/ui-audit/lib/apps.mjs`)。
混んだ機械 (ほかのワーカーと並行・3 つのアプリをまとめる回) では 5 秒を超えて揺れうるので、条件の待ちの上限を 1 つの定数にそろえる。

## 受け入れ条件

- [x] `tests/ui-audit/` の条件の待ち (`waitFor`・`expect` の `timeout` など) の上限を数え、1 つの定数 (例: `WAIT_TIMEOUT_MS`。T0884 の
      `NOTICE_TIMEOUT_MS` とまとめるかも決める) に寄せた。固定の待ち (`waitForTimeout` など) は増やしていない
- [x] 出るまでの条件の待ちなので、通る回の時間が変わらないことを、`just ui-audit --only <アプリ>` の前後の時間で確かめた
- [x] 待ちが時間切れになったとき、どの場面の何を待っていたかが失敗の文に出る (T0884 の帯と同じく、手がかりを添える)
- [x] `just ui-audit` (3 つのアプリ。混んだ機械なら `--only` で分けてよい) がエラー 0 で通る
- [x] tests/ui-audit/README.md に待ちの上限の説明を書いた
- [x] `just ci` が通る

## 参考

- T0884 のメモ (`NOTICE_TIMEOUT_MS`・`--variant-timeout`・`--deadline`)、tests/ui-audit/README.md、docs/testing.md

## メモ

- T0095 で起票した (元の候補: T0884 (ui-audit のほかの場面の 5 秒の待ち (ダイアログ・メニューなど) も 20 秒の上限にそろえる))。
- M8 の後続 (T0884 と同じ)。番号: 3 つのアプリに共通の道具なので、共有 UI の帯 (T09xx)。
- T0884 の別の候補「エージェントの手順で、ui-audit に `--deadline` を付けるか `--only` でアプリごとに流すよう案内する」は、T0095 で AGENTS.md の
  `just ui-audit` の行に 1 文を足して済ませた。
- やったこと (T0985):
  - 数えた条件の待ち: `lib/apps.mjs` の `waitFor`・`waitForFunction`・`waitForSelector` が 29 か所 (5 秒が 16・2 秒が 2 (うち 1 つは
    `closeOverlay` の Esc の間)・10 秒が 1・20 秒が 7 (帯の `NOTICE_TIMEOUT_MS` の 2 を含む)・30 秒が 1・60 秒が 2)。`run.mjs`・`walk.mjs` には条件の待ちは無い
    (`run.mjs` の失敗の撮影の `timeout: 10000` は条件の待ちではないので残した)。
  - `lib/limits.mjs` に `WAIT_TIMEOUT_MS` (20 秒) と `waitUntil(what, wait, { timeout, hint })`・`WaitTimeoutError` を足し、
    T0884 の `NOTICE_TIMEOUT_MS` はこれにまとめた。`apps.mjs` の待ちはすべて `waitUntil` か、それを包む `waitFor(locator, what, …)` を
    通す。重い読み込み (ビューアのサンプルのモデル (30 秒だった)・GPU を戻した後の描き直し (60 秒だった)) だけは
    `LOAD_TIMEOUT_MS` (= 3 × `WAIT_TIMEOUT_MS`)。
  - 時間切れの文: `timed out after 20.0s waiting for <何を> (<手がかり>)`。スタジオの帯は画面にあった帯を手がかりに添える
    (T0884 と同じ)。ビューアのサンプルの読み込みは今の status を添える。`run.mjs` は場面の失敗の文すべてに `last scene` を添える
    (これまでは `--variant-timeout`・`--deadline` のときだけ)。
  - 振る舞いの変わったところ: (1) `closeOverlay` は Esc を 3 回押しても閉じなければ、すぐ失敗にせず `WAIT_TIMEOUT_MS` まで閉じるのを
    待つ (Esc の間の 2 秒は上限ではなく押し直すまでの間なので残した)。(2) エディタの `-module-preview` の「違い」は、10 秒待って
    無ければ黙って開かずに撮っていたのを、出るまで待つ条件にした (前の回の 12 の組すべてで出て開いていた)。(3) ツリーのドラッグが
    始まる (`.tree.dragging`) 待ちは 2 秒から 20 秒。
  - 単体テスト: `test/limits.test.mjs` に `waitUntil` の 4 つ (上限を渡す・時間切れの文・手がかりが失敗しても文を保つ・
    ほかの失敗はそのまま)。Playwright の `locator.waitFor`・`page.waitForFunction` の時間切れが `TimeoutError` の名前で来て
    文が組み立つことは、ヘッドレスの Chrome の空のページで手で確かめた。
  - 前後の時間: 前のコードを一時的に別のディレクトリに写して、同じビルド (`--no-build`) で交互に流した。ビューア 61・59 秒 →
    61・59 秒、スタジオ 153 秒 → 159 秒、エディタ 401 秒 → 389 秒 (ロードアベレージ 7〜19。揺れの範囲)。3 つのアプリとも
    `--only` で分けてエラー 0 (変えた後の回: ビューア 89・スタジオ 273・エディタ 604 の場面)。
- やらなかったこと: 固定の待ち (`waitForTimeout`) は触っていない (増やしていない)。
