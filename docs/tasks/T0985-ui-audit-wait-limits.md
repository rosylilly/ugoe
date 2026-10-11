---
id: T0985
title: "ui-audit: ダイアログ・メニュー・Popover などを待つ 5 秒の上限を、混んだ機械でも揺れない 1 つの上限にそろえる"
status: todo
milestone: M8
crates: [tests/ui-audit, docs]
depends_on: []
assignee:
---

# T0985: ui-audit の待ちの上限をそろえる

## 目的

T0884 で、スタジオの自動の保存の帯と承認の知らせを待つ上限を 5 秒から 20 秒 (`NOTICE_TIMEOUT_MS`) にした。ほかの場面にも、
ダイアログ・確認・メニュー・Popover・ハイライトなどを `waitFor({ timeout: 5000 })` で待つ所が十数か所ある (`tests/ui-audit/lib/apps.mjs`)。
混んだ機械 (ほかのワーカーと並行・3 つのアプリをまとめる回) では 5 秒を超えて揺れうるので、条件の待ちの上限を 1 つの定数にそろえる。

## 受け入れ条件

- [ ] `tests/ui-audit/` の条件の待ち (`waitFor`・`expect` の `timeout` など) の上限を数え、1 つの定数 (例: `WAIT_TIMEOUT_MS`。T0884 の
      `NOTICE_TIMEOUT_MS` とまとめるかも決める) に寄せた。固定の待ち (`waitForTimeout` など) は増やしていない
- [ ] 出るまでの条件の待ちなので、通る回の時間が変わらないことを、`just ui-audit --only <アプリ>` の前後の時間で確かめた
- [ ] 待ちが時間切れになったとき、どの場面の何を待っていたかが失敗の文に出る (T0884 の帯と同じく、手がかりを添える)
- [ ] `just ui-audit` (3 つのアプリ。混んだ機械なら `--only` で分けてよい) がエラー 0 で通る
- [ ] tests/ui-audit/README.md に待ちの上限の説明を書いた
- [ ] `just ci` が通る

## 参考

- T0884 のメモ (`NOTICE_TIMEOUT_MS`・`--variant-timeout`・`--deadline`)、tests/ui-audit/README.md、docs/testing.md

## メモ

- T0095 で起票した (元の候補: T0884 (ui-audit のほかの場面の 5 秒の待ち (ダイアログ・メニューなど) も 20 秒の上限にそろえる))。
- M8 の後続 (T0884 と同じ)。番号: 3 つのアプリに共通の道具なので、共有 UI の帯 (T09xx)。
- T0884 の別の候補「エージェントの手順で、ui-audit に `--deadline` を付けるか `--only` でアプリごとに流すよう案内する」は、T0095 で AGENTS.md の
  `just ui-audit` の行に 1 文を足して済ませた。
