---
id: T0160
title: ParameterValues の並びが同じかを定数時間で見分ける口
status: done
milestone: M9
crates: [ugoe-runtime, ugoe-physics, docs]
depends_on: []
assignee: claude-worker-T0160
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

- [x] 口の形を決めて `ParameterValues` に足し、doc コメントとテストを書いた (同じ値から複製したものは同じ・並びを変えたものは違う)
- [x] `ugoe-physics` の毎フレームの比較を置き換えた。並びの違う `ParameterValues` を渡すと解決し直すテストがある
- [x] `docs/spec/evaluation.md` (または該当の仕様) に必要なら書いた
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0146・T0319 のメモ (起票の候補)
- crates/ugoe-runtime/src/parameters.rs

## メモ

- T0019 で起票した (元の候補: T0146 の `same_layout`、T0319 の「並びの変化を O(1) で」)。
- 口の形: 手札の型 `ParameterLayout` (`Arc` で並びを共有して持つ。`ids()` を持つ) と、`ParameterValues::layout()`・
  `ParameterValues::has_layout(&ParameterLayout) -> bool` (ポインタの比較・定数時間) を足した。物理やモーションは
  `ParameterValues` を持たずに手札だけ控えるので、`same_layout(&self, other)` より手札の型が合う。手札が `Arc` を持ち続けるので、
  並びが解放されて同じアドレスに別の並びができる取り違えも起きない (ポインタの値だけを控える案ではこれが起きる)。
- ポインタが違って中身が同じとき: `has_layout` は `false` を返す (doc に明記)。ugoe-physics は `false` のとき ID の列を比べ、
  同じなら解決し直さずに手札だけ取り替える。これで毎フレーム `from_model` で作り直すホストでも、従来 (文字列の比較) より遅くならない。
- 追加だけで既存の API は変えていない (`ids()` は残した)。`ParamId` の import が ugoe-physics で不要になったので外した。
- 仕様: `docs/spec/evaluation.md` の「デフォーマのワールド変換を読む」の ID からの引き当ての段落の後に 1 段落足した。
  英語の公開仕様 (`spec/`) はランタイムの API を書いていないので変えていない。
- テスト: `parameters.rs` の `has_layout_follows_shared_layout`、ugoe-physics の `resolves_parameters_again_when_layout_changes`
  (並びを逆にすると解決し直す・作り直した同じ並びは手札だけ取り替える)。既存の golden の記録ハッシュは変わらない。
- `just ci`・`just wasm-check` は通った。
- 起票の候補: なし (ugoe-motion の T0325 がこの口を使う)。
