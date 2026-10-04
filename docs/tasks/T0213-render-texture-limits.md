---
id: T0213
title: 参照レンダラ: テクスチャの大きさと番号の上限をエラーで返す
status: in-progress
milestone: M9
crates: [ugoe-render, ugoe-headless, ugoe-wasm, docs]
depends_on: [T0211]
assignee: claude-worker-T0213
---

# T0213: 参照レンダラ: テクスチャの大きさと番号の上限をエラーで返す

## 目的

T0211 (ugoe-render のコードレビュー) の所見。`Renderer::set_texture` / `set_normal_map` は、データの長さが幅と高さに
合うかだけを確かめ、デバイスの `max_texture_dimension_2d` を超える大きさをそのまま `create_texture_with_data` に渡す。
wgpu の検証エラーは既定でパニックになるので、呼び出し側が先に確かめないとホストが止まる。
いまは `ugoe-headless` (`gpu.rs`) と `ugoe-wasm` (`web.rs` の `check_texture_sizes`) がそれぞれ同じ検査を持っている。

また、テクスチャの番号 `index` で `Vec` を `index + 1` まで伸ばすので、極端に大きな番号はメモリを使い切る。

レンダラ自身が上限を確かめてエラーで返し、呼び出し側の重複した検査を外す。

## 受け入れ条件

- [ ] `set_texture` / `set_normal_map` が、幅か高さがデバイスの `max_texture_dimension_2d` を超えるときにエラーを返す (パニックしない)
- [ ] テクスチャの番号の上限を決め (例: モデルのテクスチャ数の上限、または `u16`)、超えたらエラーを返す
- [ ] `RenderError` に新しい variant を足す。今後の追加で利用者を壊さないよう `#[non_exhaustive]` にするかを判断し、doc コメントに書く
- [ ] `ugoe-headless` と `ugoe-wasm` の重複した大きさの検査を、レンダラのエラーに置き換える (利用者に見せる文言は変えないか、同じ情報を含める)
- [ ] GPU テスト (`just test-gpu`) で、上限を超えるテクスチャがエラーになることを確かめる
- [ ] `just ci` が通る

## 参考

- docs/spec/rendering.md (GPU リソースの方針)
- crates/ugoe-render/src/lib.rs (`set_texture`・`set_normal_map`・`check_rgba8`)
- crates/ugoe-headless/src/gpu.rs、crates/ugoe-wasm/src/web.rs (`check_texture_sizes`)

## メモ
