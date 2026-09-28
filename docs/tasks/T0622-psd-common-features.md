---
id: T0622
title: PSD のよく使われる未対応機能 (レイヤーマスク・Shift-JIS の名前・ZIP・16 ビット)
status: todo
milestone: M3
crates: [ugoe-psd]
depends_on: [T0620]
assignee:
---

# T0622: PSD のよく使われる未対応機能

## 目的

T0620 の調査 (`docs/tasks/T0620-psd-real-world.md` のメモ) で、実際のペイントツールがよく使うのに未対応の機能が分かった。優先度の高いものから対応する。

## 受け入れ条件

- [ ] レイヤーマスク (チャンネル -2 / -3) をアルファに適用する (優先度: 高)
- [ ] `luni` を持たない PSD のレイヤー名を Shift-JIS として読む (SAI などを想定。判定方法を決める) (優先度: 高)
- [ ] ZIP 圧縮・予測付き ZIP のチャンネルを展開する (寛容なライセンスの展開ライブラリ、例: `miniz_oxide`) (優先度: 中)
- [ ] 16 ビットの PSD (`Lr16` を含む) を 8 ビットに変換して取り込む (優先度: 中)
- [ ] 実物の PSD (`assets/fixtures/psd-real/`) が手に入ったものから、テストに加える
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0620-psd-real-world.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
