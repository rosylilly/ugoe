---
id: T1621
title: "editor-core: keep_look で名前も変えたとき、check_reparented_pairs が前のモデルと突き合わせられるようにする"
status: in-progress
milestone: M9
crates: [ugoe-editor-core]
depends_on: []
assignee: claude-worker-T1621
---

# T1621: editor-core: keep_look で名前も変えたとき、check_reparented_pairs が前のモデルと突き合わせられるようにする

## 目的

T1609 でモジュールの適用のエラーに直し方の案内を出し、`keep_look` の作り直しの警告を捨てないようにした。
`keep_look` で接頭辞 (名前) も同時に変えると、`check_reparented_pairs` が前のモデルと ID で突き合わせられず、付け替えた組を見逃す (T1609 の候補)。
接頭辞を付け替えてから比べるかを検討し、直す。

## 受け入れ条件

- [ ] 名前を同時に変えた `keep_look` で、付け替えた組が前のモデルと突き合わされることを確かめる、先に失敗するテストがある
- [ ] 接頭辞を付け替えてから比べる (か、別の方法で突き合わせる) ように直し、テストが通る。採った方法をメモに書いた
- [ ] `just ci` が通る

## 参考

- T1609 のメモ (起票の候補)

## メモ

- T0032 で起票した (元の候補: T1609 (keep_look))。
