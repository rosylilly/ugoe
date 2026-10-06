---
id: T0061
title: "e2e: 偽の AI のプロバイダが、注文に従うときに実際にリグの値を変え、ai-revision.png を撮り直す"
status: todo
milestone: M10
crates: [tests/e2e, docs]
depends_on: [T0059]
assignee:
---

# T0061: e2e: 偽の AI のプロバイダが、注文に従うときに実際にリグの値を変え、ai-revision.png を撮り直す

## 目的

`just e2e --manual` の偽の AI のプロバイダ (`tests/e2e/lib/fake-ai.mjs`) は、利用者の注文 (`<user_instructions>`) に答えるとき、
リグの値を変えずに理由の文だけを変える。そのため、注文で作り直した提案の値が最初の提案と同じで、マニュアルの画像 (ai-revision.png など) が
「注文に従って値が変わった」様子を見せられない。

## 受け入れ条件

- [ ] 偽のプロバイダが、リグの値で従える注文の文 (例: 振り向きを大きく・目を細く) に従うとき、対応する値を決まった形で変える
      (同じ注文なら同じ値。従えない文の `Not applied:` は今のまま)。変える規則を fake-ai.mjs のコメントと tests/e2e/README.md に書いた
- [ ] 偽のプロバイダの単体テストがあれば足す (無ければ、作り直した提案の値が前と違うことを筋書きの中で確かめる)
- [ ] `just e2e --manual` で ai-revision.png など注文の作り直しを写す画像を撮り直し、マニュアル (docs/manual) の説明と合うことを確かめた
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0969 のメモ (偽物の入れ方・起票の候補)、T0029 (偽の AI のプロバイダ)
- tests/e2e/lib/fake-ai.mjs、tests/e2e/specs/editor.manual.mjs、crates/ugoe-ai/src/instructions.rs、docs/manual/ (AI の節)

## メモ

- T0058 で起票した (元の候補: T0969 (偽の AI のプロバイダが注文に従った値を返す))。
- 同じ tests/e2e を触る T0059 (ビルドの形) の後にした。
