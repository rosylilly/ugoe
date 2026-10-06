---
id: T0047
title: "ui-audit: 偽のホストの見本 (スタジオのポーズのグループ・プロファイルの拡張子・AI の提案の種類) を本物にそろえる"
status: in-progress
milestone: M10
crates: [tests, docs]
depends_on: [T0045]
assignee: claude-worker-T0047
---

# T0047: ui-audit: 偽のホストの見本 (スタジオのポーズのグループ・プロファイルの拡張子・AI の提案の種類) を本物にそろえる

## 目的

`just ui-audit` の偽のホストが返す見本のデータが、本物のアプリの見本とずれている (T0919 で本物の WebView と見比べて分かった。T1643)。
ずれていると、本物では出る画面 (ポーズのパネルなど) を監査が撮らず、本物では出ない画面 (表に無い種類の出し方) を撮る。

- スタジオ: 偽のホストの見本のプロファイルに、本物の見本のポーズのグループが無い・プロファイルのファイルの拡張子が本物と違う (T0919)。
- エディタ: `tests/ui-audit/lib/editor-fixtures.mjs` の AI の提案の見本が、`ugoe_editor_core::Command` に無い種類 `set_keyform_bindings` を使っている。
  そのため提案のカードの「採用で変わるもの」が、利用者の言葉ではなく表に無い種類の出し方 (`<code>`) で出る (T1643)。

## 受け入れ条件

- [ ] 偽のホストの見本と本物の見本 (スタジオの `apps/studio/scripts/sample-profile.mjs`・`ugoe sample`・エディタの見本) の違いを洗い出し、
      メモに一覧を書いた。直さないものは理由を書いた
- [ ] スタジオの偽のホストの見本に、本物と同じポーズのグループと拡張子を持たせた
- [ ] `editor-fixtures.mjs` の提案の見本が、実在する種類だけを使う (例: `set_keyform_cell`)。表に無い種類の出し方を撮る場面が要ると判断したら、
      別の場面として足した (要らないなら理由をメモに書く)
- [ ] 見本が実在する種類だけを使うことを確かめる検査 (`hosts.test.mjs` などで、`crates/ugoe-editor-core/src/command.rs` の種類と比べる) がある
- [ ] tests/ui-audit/README.md の偽のホストの説明を直した
- [ ] `just ui-audit --only editor,studio` で、増えた・変わった指摘を `--compare` で確かめ、メモに書いた
- [ ] `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」、T1643 のメモ (`set_keyform_bindings`)
- tests/ui-audit/lib (editor-fixtures.mjs・studio-host.mjs・editor-host.mjs)、tests/e2e/README.md#ヘッドレスの-chrome-just-ui-audit-との違い

## メモ

- T0046 で起票した (元の候補: T0919 (偽のホストの見本を本物に)・T1643 (`set_keyform_bindings`))。
- T0045 (作業中) が同じ tests/ui-audit/lib を触るので、その後にした。
