---
id: T0114
title: ビルド時のテクスチャのアトラス化
status: done
milestone: M4
crates: [ugoe-editor-core, ugoe-cli]
depends_on: [T0604]
assignee: claude-worker-T0114
---

# T0114: ビルド時のテクスチャのアトラス化

## 目的

`.ugoe` の書き出し時に、描画順で隣り合う要素が同じテクスチャを使うように、テクスチャをアトラスにまとめる。

## 受け入れ条件

- [x] アトラス化で UV を書き換え、見た目が変わらないことを `ugoe render` の画像比較で確認するテストがある
- [x] 描画の状態の切り替え回数が減ることを `FrameStats` で確認する
- [x] `ugoe build` (または書き出しのオプション) から使える
- [x] `just ci` が通る

## 参考

- `docs/adr/0016-draw-batching.md`

## メモ

- `ugoe_editor_core::atlas` (`build_atlas`・`AtlasOptions`・`Project::with_atlas`) を追加した。規則は
  `docs/spec/file-formats.md#テクスチャのアトラス化`。既定のパラメータでの描画順で最初に使われる順に棚詰めし、
  収まらなくなったら次のアトラスへ。端の画素を `padding` (既定 2) だけ複製し、UV が 0〜1 の外のテクスチャと
  大きすぎるテクスチャは残し、使われないテクスチャは取り除く。`Command` は増やしていない (ビルド時の変換のため)。
- CLI: `ugoe build <.ugoeproj|.ugoe> -o out.ugoe [--atlas --atlas-max-size --atlas-padding]` と `ugoe render --stats`
  (FrameStats を JSON で 1 行) を追加した。既定の書き出しは変えていない (見本のアバターの生成結果もそのまま)。
- テスト: CPU のバイリニア・クランプの読み出しで前後が一致すること (`atlas_tests.rs`)。GPU (`--features gpu-tests`) の
  `crates/ugoe-cli/tests/atlas_render_gpu.rs` で、見本のアバター (24 枚 → 1 枚) と端の鋭い不透明なテクスチャのモデルを
  描いて乗算済みの画素を比べる (padding 0 にすると後者は失敗することを確かめた)。`state_changes` は見本で 20 → 0、
  `draw_calls` は 21 → 1 (llvmpipe で確認)。GPU のテストは `just ci` には含まれない。
- やらなかったこと・後続の候補:
  - エディタの書き出し (Tauri の `export_bundle`) からアトラス化を選べるようにする (apps/editor は範囲外)。
  - 同じ画像の重複の除去、画像の中の使われない (UV が覆わない) 領域の切り取り。
  - ミップマップを使うレンダラ向けの、padding とアトラス内の配置の揃え (今の参照レンダラはミップマップなし)。
  - モジュール (T0111 の着せ替え) のテクスチャのアトラス化 (モデル本体とは別のファイルに持つ)。
  - KTX2 のテクスチャを含むバンドルは `ugoe build` がエラーにする。
