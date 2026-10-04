---
id: T0429
title: 受け手の follow で無限大のリップシンクの強さとパーツの不透明度を拒む (SDK と wasm)
status: done
milestone: M9
crates: [ugoe, ugoe-wasm, docs]
depends_on: [T0423]
assignee: claude-orchestrator
---

# T0429: 受け手の follow で無限大のリップシンクの強さとパーツの不透明度を拒む (SDK と wasm)

## 目的

T0423 (ugoe-ffi のレビュー) で見つけた。`ugoe::Puppet::follow` はリップシンクの音量・強さとパーツの不透明度の値・重みの **NaN** だけを拒む。
強さが無限大で、音量と口のパラメータの今の値が等しいと、`ParameterValues::blend` が `0 × ∞ = NaN` を書く
(`ParameterValues::set` の `clamp` は NaN を通す)。C ABI の `ugoe_puppet_follow(level 0, strength INFINITY)` で `MouthOpenY` が NaN に
なるのを確かめた。

C ABI は T0423 で境界で拒むようにした (当てるときの強さと、パーツの不透明度の値と重みが有限でなければ `UGOE_ERROR_INVALID_ARGUMENT`)。
SDK の `Puppet::follow` (と、それを呼ぶ wasm の `follow`) も同じ条件で拒み、`ParameterValuesError` に種類を足すか既存の種類を広げる。
C ABI の検査は SDK に寄せてよい。

## 受け入れ条件

- [ ] `Puppet::follow` が無限大の強さ・パーツの不透明度の値と重みを、何も変えずにエラーにする (先に失敗するテストを書く)
- [ ] wasm の `follow` が同じ入力でエラーを返す
- [ ] `docs/spec/motion.md#受け手として進める` と crates/ugoe-wasm/README.md・crates/ugoe-ffi/README.md の記述を揃えた
- [ ] `just ci` が通る

## 参考

- `crates/ugoe/src/follow.rs`、`crates/ugoe-wasm/src/web/follow.rs`、`crates/ugoe-ffi/src/follow.rs`
- T0139 (SDK のレビュー)・T0424 (wasm のレビュー)。重なっていればそちらで直してこのタスクを閉じる

## メモ
- **閉じた (M9 のマージ):** T0139 (SDK) で Puppet::follow が有限でない強さ・不透明度の値と重みを拒むようにした。wasm の follow は SDK を通るので同じく例外になる (T0424 で確認)。入口ごとの扱いを揃えて書くのは T0433。
