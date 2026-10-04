---
id: T0428
title: 目パチが巨大な dt で止まらない (EyeBlinkPlayer::advance の無限ループ)
status: todo
milestone: M9
crates: [ugoe-motion, docs]
depends_on: [T0423]
assignee:
---

# T0428: 目パチが巨大な dt で止まらない (EyeBlinkPlayer::advance の無限ループ)

## 目的

T0423 (ugoe-ffi のレビュー) で見つけた。`EyeBlinkPlayer::advance` は `dt -= (len - t)` で段階を進めるが、`dt` が段階の長さに比べて
大きすぎると (f32 で `1e20 - 4.0 == 1e20` など) `dt` が減らず、ループが終わらない。`ugoe_puppet_update(p, 1e20)` は有限なので
C ABI の検査を通り、`fixtures::pose_and_effects` で返ってこない (60 秒で打ち切って確かめた)。`ugoe::Puppet::update`・wasm も同じ道を通る。
`dt` が 1e7 程度でも数百万回まわる。

`dt` を 1 周期 (待ち + 閉じる + 閉じた + 開く) の余りに畳む、または回数に上限を付けるなどで、どんな有限の `dt` でも有限の時間で返るようにする。
決定性 (同じ `dt` の列で同じ結果) は保つ。呼吸などほかの効果に同じ形が無いかも確かめる。

## 受け入れ条件

- [ ] 巨大な有限の `dt` (1e20 など) で `EyeBlinkPlayer::advance` がすぐ返ることをテストで確かめた (先に失敗するテストを書く)
- [ ] ふつうの `dt` での目パチの結果は変わらない (既存のテストが通る)
- [ ] ほかの効果のプレイヤーに同じ形のループが無いか確かめた
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-motion/src/effects.rs` (`EyeBlinkPlayer::advance`)
- `docs/spec/motion.md`
- T0317 (ugoe-motion のレビュー。重なっていればそちらで直してこのタスクを閉じる)

## メモ
