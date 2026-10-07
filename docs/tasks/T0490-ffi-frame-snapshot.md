---
id: T0490
title: "C ABI: 毎フレームの描画の状態をまとめて受け取る形に置き換え、描画要素ごとの関数を廃止する"
status: done
milestone: M11
crates: [ugoe-ffi, ugoe, bindings/unity, tests/conformance, docs]
depends_on: [T0489]
assignee: claude-worker-T0490
---

# T0490: C ABI: 毎フレームの描画の状態をまとめて受け取る形に置き換え、描画要素ごとの関数を廃止する

## 目的

毎フレームの描画の状態は、描画要素ごとの関数 (`ugoe_puppet_drawable_state`・`ugoe_puppet_positions` など) で読んでいる。
そのため、境界を越える呼び出しが重いホスト (cgo・マネージドな言語) では、呼び出しが描画要素の数に比例して増える。
毎フレームの状態は 1 回の呼び出しでまとめて受け取る形だけにし、描画要素ごとの関数は廃止する。読み込みのときに 1 回読む情報 (UV・インデックス・
テクスチャ・マスクの参照など) は描画要素ごとのまま残す ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#毎フレームの描画の状態はまとめて渡す))。
描き方 (描画順に従って各描画要素を描く) は変えない。

## 受け入れ条件

- [x] まとめて受け取る形で、すべての描画要素の状態 (今の `UgoeDrawableState` の中身: 表示・不透明度・色・合成モード・マスクの効き具合・
      描画順の値・変更のフラグ)、描画順、描画順が変わったか、頂点の位置を読める
- [x] 頂点の位置は、描画要素ごとの範囲 (先頭と数) を持つ 1 つの連続した配列。借用の形 (次の `ugoe_puppet_update`・着せ替え・破棄まで有効) と、
      ホストのバッファへのコピーの形 (足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で要る数) の両方がある
- [x] 変更のフラグで、変わっていない描画要素の位置を写さずに済む
- [x] `ugoe_puppet_drawable_state`・`ugoe_puppet_positions`・`ugoe_puppet_copy_positions`・`ugoe_puppet_render_order`・
      `ugoe_puppet_render_order_changed` を消し、`ABI_VERSION` を上げた (T0489 と同じリリースの間なら、上げるのは 1 回でよい)
- [x] 置き換える前の関数と同じ値になることを、消す前にテストで確かめた (見本のモデルで、いくつかのフレームを比べる)
- [x] リポジトリの中の利用者を移した: C の見本 (`examples/c/sample.c`)、適合テスト (`tests/conformance/c/runner.c`)、
      Unity (`Native.cs` と毎フレームの更新。1 フレームの呼び出しの数が減ったことをメモに書く)。`just refs` が通る
- [x] SDK (`ugoe`) に連続した配列の形が要るなら足した (`ugoe-runtime` の `DrawableState` の形は変えない)
- [x] ヘッダ (`just ffi-header`)・crates/ugoe-ffi/README.md・docs/portability.md#c-abi-ugoe-ffi-の規約 (頂点データの借用とコピーの説明) を更新した。
      `just ffi-check` と Unity のテストが通る
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/src/drawable.rs、docs/spec/rendering.md#描画の最適化、
  [ADR 0016](../adr/0016-draw-batching.md)

## メモ

- 2026-10-06 起票。同日、利用者の判断で「足すだけ」から「置き換えて廃止する」に改めた。
- T0489 と同じファイル (C ABI・C の見本・適合テスト・Unity) を触るので、T0489 の後に行う。
- パーツの不透明度 (`ugoe_puppet_part_opacity` など) は描画の状態ではないので対象外。
- 2026-10-07 (claude-worker-T0490) 実装してレビューに出した。
  - 形 (crates/ugoe-ffi/src/frame.rs。crates/ugoe-ffi/README.md#毎フレームの描画の状態-t0490):
    - 借用: `ugoe_puppet_frame(UgoePuppet *p, UgoeFrame *out, UgoeError *error)`。`UgoeFrame` は `drawable_count`・`states` (`UgoeDrawableState` の並び。
      中身は前と同じ)・`render_order`・`render_order_changed`・`position_ranges` (`UgoePositionRange { size_t first; size_t count; }`。単位は float)・
      `positions`・`positions_len`。
    - コピー: `ugoe_puppet_copy_frame(UgoePuppet *p, const UgoeFrameBuffers *buffers, bool changed_positions_only, UgoeFrameCopy *out, UgoeError *error)`。
      `UgoeFrameBuffers` はホストのポインタと `drawable_capacity`・`positions_capacity`。足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で `*out`
      (要る数と `render_order_changed`) だけを書く。`changed_positions_only` で、位置は変更のフラグが立った要素の範囲だけを写す。
    - 最後の引数は T0489 の申し送りどおり `UgoeError *error`。
  - 内部: 位置をつないだ配列は `UgoePuppet` に持ち、2 つの関数を呼んだときに作る (評価の結果が同じなら作り直さない)。前に読んだ評価の
    ちょうど次の `update` なら位置の変わった要素だけを写す (`ModelInstance::id`・`update_count` で判断)。評価する関数 (`update`・`follow`・
    着せ替え・物理の差し替え) ごとに作り直す箇所を足さずに済み、漏れが起きない。そのため 2 つの関数は `UgoePuppet *` (const でない) を取る。
    描画順は `ModelInstance::render_order` をそのまま貸す。
  - SDK (`ugoe`) は変えていない (連続した配列は C ABI の側で作れば足りた)。T0180 と衝突しない。Godot は SDK を直接使うので変更なし
    (`just godot-check` は通る)。
  - `ABI_VERSION` は 2 のまま。T0489 で 1 → 2 に上げてからリリースしておらず (タグ無し)、2 の版のライブラリを配ったことがないため、受け入れ条件の
    「T0489 と同じリリースの間なら、上げるのは 1 回でよい」に当たる。理由は crates/ugoe-ffi/README.md#abi-の版 に書いた。
  - 置き換える前と同じ値になることは、消す前のコミット (a04f9116) の `crates/ugoe-ffi/tests/frame.rs` で、avatar.ugoe の 11 フレーム
    (パラメータを途中で変える・1 フレーム読まずに飛ばす) と着せ替えの前後で、借用・コピー・変わった範囲だけのコピーを、描画要素ごとの関数で読んだものと
    ビット単位で比べて確かめた。消した後は同じテストを `ugoe::Puppet` を直接読んだものとの比較に替えた。
  - 移した利用者: C の見本 (sample.c。借用とコピーの両方と BUFFER_TOO_SMALL)、適合テストの C の実行器 (runner.c)、Unity
    (`Native.cs`・`Puppet.CopyFrame` (internal)・`FrameSync.Pull`・適合テストの `Program.cs`)、crates/ugoe-ffi/tests。
  - Unity の 1 フレームの C ABI の呼び出しの数 (`update` を除く): 前は要素ごとの状態の N 回 + 位置の変わった要素の数 + 描画順が変わったかの 1 回 +
    変わったときの描画順の 1 回 (24 要素の avatar.ugoe で最大 50 回)。今は `ugoe_puppet_copy_frame` の 1 回。
    `FrameSync` の公開の形 (`Positions` は要素ごとの float[]) は変えず、つないだ配列から変わった要素だけを写す。
    `Puppet.GetDrawableState`・`CopyPositions`・`CopyRenderOrder`・`RenderOrderChanged` は消し、変更のフラグによらずすべてを読む `FrameSync.PullAll` を足した
    (適合テストが使う)。配列は呼び出しの間だけ `GCHandle` で固定する (アセンブリは unsafe を使わない)。
  - `crates:` の外の小さな変更: `scripts/check-ffi-bindings.mjs` の Unity の OMIT を `ugoe_puppet_positions` から `ugoe_puppet_frame` に替えた。
  - 通した検査: `just crate ugoe-ffi`・`just ffi-check`・`just refs`・`just unity-check`・`just conformance` (c・wasm・unity・godot すべて ok)・
    `just godot-check`・`just cross-check`・`just wasm-check`・`just ci`。
  - 確かめていないこと: 実際の Unity (IL2CPP を含む) での動作と、Windows・macOS での実行 (`just cross-check` の clippy まで)。
  - 起票の候補: なし。
