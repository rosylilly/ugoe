---
id: T0839
title: スタジオの操作 API の -32010 の error.data に args と ADR 0029 のコードを足すか決める (判断が要る)
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0838]
assignee:
---

# T0839: スタジオの操作 API の -32010 の error.data に args と ADR 0029 のコードを足すか決める (判断が要る)

## 目的

T0835 でスタジオのコマンドのエラーを `{ code, args, message }` にしたが、操作 API の `-32010` の `error.data` は外のツールとの公開の約束なので、
平らな引数と camelCase のコード (`unknownScene` など) のまま残した (T0835 の候補)。`args` と ADR 0029 のコード (`scene.unknown` など) を足せば、
外のツールがスタジオと同じ訳を使える。足すだけなら互換を壊さない。

**判断が要る:** 外部 API (docs/studio-api.md) の公開の約束の変更。足すか、足すならフィールドの名前。

## 受け入れ条件

- [ ] 利用者の判断を得た (待つ間は blocked にしてメモに案を書く)
- [ ] 足すなら、`-32010` の `error.data` に新しいフィールドがあり、今のフィールドは変わらないテストがある。docs/studio-api.md を更新した
- [ ] `just ci` が通る

## 参考

- T0835 のメモ (起票の候補)、ADR 0029、apps/studio/README.md (「コマンドのエラー」)

## メモ

- T0026 で起票した (元の候補: T0835)。オーケストレーターの集計には無かったが、M9 の done のタスクのメモの起票の候補にあった。
