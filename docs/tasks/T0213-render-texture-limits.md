---
id: T0213
title: 参照レンダラ: テクスチャの大きさと番号の上限をエラーで返す
status: done
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

- [x] `set_texture` / `set_normal_map` が、幅か高さがデバイスの `max_texture_dimension_2d` を超えるときにエラーを返す (パニックしない)
- [x] テクスチャの番号の上限を決め (例: モデルのテクスチャ数の上限、または `u16`)、超えたらエラーを返す
- [x] `RenderError` に新しい variant を足す。今後の追加で利用者を壊さないよう `#[non_exhaustive]` にするかを判断し、doc コメントに書く
- [x] `ugoe-headless` と `ugoe-wasm` の重複した大きさの検査を、レンダラのエラーに置き換える (利用者に見せる文言は変えないか、同じ情報を含める)
- [x] GPU テスト (`just test-gpu`) で、上限を超えるテクスチャがエラーになることを確かめる
- [x] `just ci` が通る

## 参考

- docs/spec/rendering.md (GPU リソースの方針)
- crates/ugoe-render/src/lib.rs (`set_texture`・`set_normal_map`・`check_rgba8`)
- crates/ugoe-headless/src/gpu.rs、crates/ugoe-wasm/src/web.rs (`check_texture_sizes`)

## メモ

- やったこと:
  - `ugoe-render`: `RenderError` に `TextureTooLarge` / `NormalMapTooLarge` (番号・幅・高さ・上限) と `TextureIndexOutOfRange(u32)` を足した。
    `TextureTooLarge` / `NormalMapTooLarge` の文言は、これまで `ugoe-headless`・`ugoe-wasm` が出していた文 (`texture {i} is WxH, which exceeds the device limit {max}` /
    `the normal map of texture {i} is ...`) と同じにした。
  - `RenderError` は `#[non_exhaustive]` にした (doc コメントに理由。今回の追加で網羅的な `match` はどのみち壊れるので、ここで付けて以後の追加を非破壊にする。
    ワークスペースに網羅的な `match` は無かった)。
  - 番号の上限は `pub const MAX_TEXTURES: u32 = 1 << 16` (番号は `0..MAX_TEXTURES`)。モデルのテクスチャ数の上限はモデルにもファイル形式にも無いので、
    `u16` の範囲にした (表は高々 65536 要素)。
  - デバイスの上限はレンダラを作るときに `device.limits().max_texture_dimension_2d` を覚える。`Renderer::check_texture` / `check_normal_map` (番号と大きさだけ。
    何も変えない) を公開し、`set_texture` / `set_normal_map` もそれを使う。
  - `ugoe-headless` (`gpu.rs` の `new_renderer`): 重複した検査を外し、`set_texture` / `set_normal_map` のエラー (`Error::Render`、`transparent` なので文は同じ) に任せた。
    出力の大きさの検査は描画先なので残した。
  - `ugoe-wasm`: `check_texture_sizes` を残し、中身を `Renderer::check_texture` / `check_normal_map` の呼び出しにした。読み込み (`load`・`load_module`) が
    何かを変える前に失敗する性質 (モデルの読み直しの途中でテクスチャだけ置き換わらない・ライブラリに入らない) を保つため、事前の検査は呼び出し側に残し、検査の中身だけをレンダラに 1 か所にまとめた。
  - テスト: GPU テスト `textures_beyond_the_limits_are_errors` (上限 +1 の幅と高さ・`MAX_TEXTURES` と `u32::MAX` の番号がエラーで、失敗した登録は何も変えない。
    `MAX_TEXTURES - 1` は受け付ける) と単体テスト `texture_index_must_be_below_the_limit`。`just test-gpu` はこの環境 (WSL2 の dzn) で通った。
  - `docs/spec/rendering.md` の「GPU リソースの方針」に登録のエラーを書いた。英語の公開仕様 (`spec/rendering.md`) は参照レンダラの API を書いていないので変えていない。
- やらなかったこと: `ugoe-headless` の大きすぎるテクスチャの GPU テストは足していない (文言はレンダラのエラーそのまま。レンダラの GPU テストで確かめた)。
  `ugoe-wasm` のモジュールのテクスチャは、事前の検査ではモジュール内の番号で確かめ、実際に登録する番号 (本体の後ろ) の上限は付けるとき (`WardrobeError::Texture`) に分かる。
- `just ci` は通った (main に rebase した後)。`just wasm-check` と `cargo clippy -p ugoe-render --features gpu-tests --all-targets -D warnings` も通った。
- 起票の候補: なし。
