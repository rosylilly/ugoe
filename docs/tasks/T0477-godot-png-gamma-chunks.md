---
id: T0477
title: "Godot: PNG のガンマや色のチャンクで画素が参照と変わらないようにし、png-16bit に gAMA を戻す"
status: in-progress
milestone: M9
crates: [bindings/godot, tests/conformance, docs]
depends_on: [T0173]
assignee: claude-worker-T0477
---

# T0477: Godot: PNG のガンマや色のチャンクで画素が参照と変わらないようにし、png-16bit に gAMA を戻す

## 目的

T0473 で、適合テストの `png-16bit` に `gAMA` (ガンマ 1.0) を書くと、Godot の画像が参照と合わなかった (87.5% の画素が一致、最大 72/255)。
Godot の PNG の読み込みが `gAMA` (おそらく `sRGB`・`iCCP`・`cHRM` も) で値を変えるため。参照 (`ugoe_format::image`) はこれらを無視する
(T0173 で仕様に書く)。そこで `gAMA` を外したケースで通している (T0473 の候補)。

## 受け入れ条件

- [ ] Godot の PNG のテクスチャが、ガンマや色のチャンクを持つ PNG でも参照と同じ画素になる (C ABI のデコーダで読む・読み込みの前にチャンクを除く、など。選んだ方法と理由をメモと bindings/godot/README.md に書く)
- [ ] 適合テストの `png-16bit` に `gAMA` を戻す (または `gAMA`・`sRGB` を持つケースを足す) と、Godot の実行器が参照と一致する
- [ ] tests/conformance/README.md のケースの説明を直した
- [ ] `just godot-check`・`just conformance`・`just ci` が通る

## 参考

- T0473 のメモ (やらなかったこと・起票の候補)、T0173
- bindings/godot/README.md、tests/conformance/src/bundles.rs・cases.json

## メモ

- T0038 で起票した (元の候補: T0473 (Godot の gAMA))。
