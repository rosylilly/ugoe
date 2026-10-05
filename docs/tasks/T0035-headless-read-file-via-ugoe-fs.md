---
id: T0035
title: ugoe-headless の read_file を ugoe-fs に寄せるか決める (依存方向の判断が要る)
status: todo
milestone: M9
crates: [ugoe-headless, scripts, docs]
depends_on: []
assignee:
---

# T0035: ugoe-headless の read_file を ugoe-fs に寄せるか決める (依存方向の判断が要る)

## 目的

T0025 で、ファイルの読み込みを `ugoe-fs` の `read_regular_file` (通常のファイルだけを上限付きで読む) にまとめた。
`ugoe-headless` の `read_file` (`crates/ugoe-headless/src/lib.rs`) も `metadata` を見てから読む同じ形の読み込みだが、
今の依存方向 (`scripts/check-layers.mjs` の `"ugoe-headless": RUNTIME_TIER`) では、ツール層の `ugoe-fs` に依存できない (T0025 の候補)。

**判断が要る:** (a) `ugoe-headless` が `ugoe-fs` に依存してよいことにする (`ugoe-fs` は std だけに依存する葉なので、ランタイム層の制約には触れない。
`tests/conformance` も `ugoe-headless` を使う)、(b) `ugoe-headless` の中で同じ検査 (開いた後に通常のファイルかと大きさを見直す) を写す、(c) 今のまま。
推奨を書き、利用者の判断を待ってから実装する。

## 受け入れ条件

- [ ] 案と推奨をメモに書き、利用者の判断を得た
- [ ] (a) か (b) なら: `read_file` が装置・FIFO・ディレクトリを開かずに拒み、上限を超えるファイルを読まない。そのテストがある
- [ ] (a) なら `scripts/check-layers.mjs` と docs/architecture.md・crates/ugoe-fs/README.md の依存の説明を更新した
- [ ] `just ci` が通る

## 参考

- T0025 のメモ (起票の候補)
- crates/ugoe-fs/README.md、scripts/check-layers.mjs

## メモ

- T0032 で起票した (元の候補: T0025 (ugoe-headless の read_file))。
- 判断 (2026-10-05、利用者の判断): (a) `ugoe-headless` が `ugoe-fs` に依存してよいことにする (check-layers で許し、理由を docs/architecture.md に書く)。
