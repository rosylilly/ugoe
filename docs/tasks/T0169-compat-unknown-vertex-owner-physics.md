---
id: T0169
title: assets/compat の未来の版のファイルに、未知の VertexOwner を含む物理を足す
status: done
milestone: M9
crates: [assets/compat, ugoe-format, docs]
depends_on: []
assignee: claude-worker-T0169
---

# T0169: assets/compat の未来の版のファイルに、未知の VertexOwner を含む物理を足す

## 目的

T0140 で物理の未知の `VertexOwner` を読めるようにしたが、互換性テストの「未来の版を模したファイル」(`assets/compat/`) には
未知の `VertexOwner` を含む物理が無い (T0140 の候補)。今の版の読み込みが、未来の版の値で落ちないことを資料で確かめる。

## 受け入れ条件

- [x] `assets/compat/` の未来の版のファイルに、未知の `VertexOwner` を持つ物理の出力先 (または頂点の持ち主) がある
- [x] 互換性テストが、そのファイルを読み込めて、該当の物理を仕様どおりに扱う (無視する・警告する) ことを確かめる
- [x] docs/testing.md#互換性テスト の資料の説明を更新した
- [x] `just ci` が通る

## 参考

- T0140 のメモ (起票の候補)、docs/testing.md#互換性テスト

## メモ

- T0026 で起票した (元の候補: T0140 の assets/compat)。
- やったこと
  - `assets/compat/future/optional-feature.ugoemod` のリグ `Sway` に、頂点への出力 (`vertices`) で持ち主が知らない種類のものを 2 つ足した
    (中身のある `{"bone": {...}}` と、中身の無い `"lattice"`)。`dev/` と `future/` の資料は作り直した (変わったのはこのファイルだけ)。
  - `crates/ugoe-format/tests/compat.rs` の `unknown_optional_features_in_modules_fall_back` で、
    警告 (`physics.rigs[Sway].outputs[1|2].target.owner`)・`VertexOwner::Unknown` として読めること・合成 (`compose_physics`) で残ることを確かめ、
    物理を 1 秒進めた結果が、未知の出力を除いた設定で進めた結果と同じになる (= 無視する) ことを確かめた。比べる意味があるように、物理が髪を動かしていることも確かめる。
    同じモジュールの入力の種類も未知 (`acceleration`) で元のモジュールとは動きが違うので、元のモジュールではなく未知の出力を除いた設定と比べている。
  - そのため `ugoe-format` の dev-dependencies に `ugoe-physics` を足した (ワークスペースの依存。dev だけなので依存方向とランタイムの依存は変わらない)。
  - `docs/testing.md#互換性テスト` の `future/` の表 (`optional-feature.ugoemod` の行) を更新した。
- やらなかったこと
  - バンドルの `optional-feature.ugoe` には物理 (`PHYS`) が無いので足していない (元の `dev/dress-up-base.ugoe` に物理が無く、比べる相手が無いため)。
- `just ci` は通った。テストのコードと資料だけの変更なので `just wasm-check` は流していない。
