---
id: T0160
title: ParameterValues の並びが同じかを定数時間で見分ける口
status: todo
milestone: M9
crates: [ugoe-runtime, ugoe-physics, docs]
depends_on: []
assignee:
---

# T0160: ParameterValues の並びが同じかを定数時間で見分ける口

## 目的

`ugoe-physics` は、前のフレームで解決したパラメータの添字が使えるかを、毎フレーム ID の列を文字列で比べて確かめている
(`crates/ugoe-physics/src/lib.rs` の `resolved_ids.as_deref() == Some(parameters.ids())`)。T0146 で `ParameterValues` は内部で ID の列を
`Arc` で共有するようになったので、ポインタの比較で「同じ並び」を定数時間で返す口 (`same_layout(&self, other)`、または並びの手札を返す `layout()`) を足せる。
`ugoe-motion` の添字の確かめ (T0325) も同じ口を使う。

**判断が要る:** `ugoe-runtime` の公開 API の追加 (crates.io に出すクレート)。口の形 (比べるメソッドか、手札の型か) と、ポインタが違っても中身が同じときに
`false` を返してよいか (呼び出し側は解決し直すだけなので正しさは変わらない) を決める。

## 受け入れ条件

- [ ] 口の形を決めて `ParameterValues` に足し、doc コメントとテストを書いた (同じ値から複製したものは同じ・並びを変えたものは違う)
- [ ] `ugoe-physics` の毎フレームの比較を置き換えた。並びの違う `ParameterValues` を渡すと解決し直すテストがある
- [ ] `docs/spec/evaluation.md` (または該当の仕様) に必要なら書いた
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0146・T0319 のメモ (起票の候補)
- crates/ugoe-runtime/src/parameters.rs

## メモ

- T0019 で起票した (元の候補: T0146 の `same_layout`、T0319 の「並びの変化を O(1) で」)。
