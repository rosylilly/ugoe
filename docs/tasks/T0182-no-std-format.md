---
id: T0182
title: ugoe-format を no_std + alloc にする (rmp-serde・ruzstd・std::io の扱い)
status: done
milestone: M11
crates: [ugoe-format, Cargo.toml, Cargo.lock, scripts, docs]
depends_on: [T0179]
assignee: claude-worker-T0182
---

# T0182: ugoe-format を no_std + alloc にする (rmp-serde・ruzstd・std::io の扱い)

## 目的

`.ugoe` バンドルと JSON の読み書きを `no_std + alloc` でビルドできるようにする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。
ゲーム機でもホストはバンドルのバイト列を渡すだけで読み込めるようにする。

## 受け入れ条件

- [x] `rmp-serde` が no_std で使えるかを確かめた。使えなければ、MessagePack の読み書きを `rmp` などで書き直すか別のクレートに替え、
      書き出すバイト列が今と同じであることを、既存のバンドル (`assets/samples`・`assets/compat`) の読み書きで確かめた
- [x] `ruzstd` を no_std で使い、`std::io::Read` に頼っている読み込み (`bundle.rs`・`image.rs`) をバイト列のスライスの読み込みにした
- [x] PNG のデコード (`png`) と BCn のデコード (`bcdec_rs`) は `std` の feature の側に置いた (no_std では KTX2 の圧縮していない形と zstd だけを読む。
      デコードはもともとホストの役目)。`std` を外したときに使えない関数を README と doc コメントに書いた
- [x] 読み込みの上限 (`ReadLimits`) が no_std でも同じに効く
- [x] `ugoe-format` を `just nostd-check` の対象に足した
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、[ADR 0008](../adr/0008-bundle-payload-encoding.md)、[ADR 0030](../adr/0030-compressed-ktx2-decoding.md)、
  docs/spec/file-formats.md#読み込みの上限

## メモ

- 2026-10-06 起票。依存を替えるなら ADR 0031 の「見直すきっかけ」に当たるので、ADR に結果を書き足す。
- 2026-10-07 (claude-worker-T0182) やったこと:
  - `rmp-serde` 1.3 は `std::io` を前提にし、`std` を外す feature を持たない (中の `rmp` は外せる) ので使えなかった。
    `ugoe-format` の中に MessagePack の serde の実装 `src/msgpack.rs` (非公開。テストを含めて 1,300 行ほど) を書いて置き換えた。
    新しい依存は無い (`rmp` も使わない。`rmp` は自分の `std` の有無で公開 API の形が変わり、ワークスペースでは開発時の依存の
    `rmp-serde` が `std` を有効にするので、両方の形で書く手間のほうが大きい)。ライセンスの変化も無い (`rmp-serde` (MIT) は
    開発時の依存に移しただけ)。
  - 書き出すバイト列と読み込みの結果は `rmp-serde` の `to_vec_named`・`from_read_ref` と同じにした。単体テストで `rmp-serde` と比べる:
    整数・文字列・バイナリ・配列・マップの長さの境目、serde の形 (外部・内部タグ付き・untagged の enum、flatten、単位構造体など)、
    書き出しでは作られない形 (拡張型・`{名前: nil}` など)、入れ子の深さの上限、壊したバイト列 5,000 通り、
    `assets/samples` と `assets/compat` のすべての MessagePack のチャンク。さらに、`assets/` のすべてのバンドルとモジュールを
    読み直して書き出したバイト列 (無圧縮と zstd) と誤りの文を、置き換えの前後で比べて同じだった (一時的なテストで確かめ、コミットしていない)。
  - `FormatError::MsgPackEncode`・`MsgPackDecode` の中身は `rmp_serde` の誤りの型から `MsgPackEncodeError`・`MsgPackDecodeError`
    (クレートの根から公開) に替わった (公開 API の変更。中身の型を見ている利用者はリポジトリに無かった)。
  - zstd の展開 (`bundle.rs`・`image.rs`) は `ruzstd::io::Read` で読む (`std` があれば `std::io::Read` そのもの)。`ruzstd` の `std` は
    自分の `std` の feature から有効にする。
  - `image` モジュールを常に置き、PNG の関数 (`decode_png`・`decode_png_premultiplied`・`decode_image`・`decode_image_premultiplied`) と
    BCn の展開を feature `png` (`std` を含むようにした) の側に置いた。feature が無ければ BCn の KTX2 は `ImageError::UnsupportedKtx2`。
    README・doc コメント・docs/portability.md に書いた。
  - 読み込みの上限: 一時的に開発時の依存の自分 (`png`) を外して `cargo test -p ugoe-format --no-default-features --lib` を流し
    (`ruzstd` の `std` が無い形)、`ReadLimits` と KTX2 の zstd のテストを含む 88 のテストが通ることを確かめた。そのために PNG と BCn を
    使う単体テストに `#[cfg(feature = "png")]` を付け、`png` が無いときの BCn のテストを足した (普段のテストでは `png` が有効なので流れない)。
  - ワークスペースの `ugoe-format` の既定の feature を切り、`png` を使わない利用者 (`ugoe`・`ugoe-cli`・`ugoe-mcp`・エディタ) に
    `features = ["std"]` を書いた。`NOSTD_CRATES` に `ugoe-format` を足した。`features.rs` の `HashSet` は `BTreeSet` にした (結果は宣言順の `Vec`)。
  - ADR 0031 に「追記: ugoe-format の MessagePack (T0182)」を書き、ADR 0008 と docs/spec/file-formats.md からその旨を参照した。
  - 範囲 (`crates:`) の外: `crates/ugoe`・`ugoe-cli`・`ugoe-mcp`・`apps/editor/src-tauri` の `Cargo.toml` (依存の feature の行だけ)、
    `bindings/godot/Cargo.lock` (`rmp-serde`・`rmp`・`num-traits`・`autocfg` が消えた。godot のビルドは `--locked`)。
  - 検査: `just nostd-check`・`just wasm-check`・`just licenses`・`just publish-check`・`just groups`・`just refs`・`just ci`、
    `cargo clippy`/`cargo test --manifest-path bindings/godot/Cargo.toml --locked`・`node scripts/check-licenses.mjs --godot` が通った。
- やらなかったこと: `i128`・`u128` と MessagePack の拡張型の書き出し (モデルの型は使わない)。`png` が無い形の単体テストを CI で流すこと
  (開発時の依存の自分が `png` を有効にするため。上の手順で一度だけ確かめた)。
- 起票の候補: `png` を外した `ugoe-format` の単体テストを CI で流す仕組み (結合テストを `required-features` にして開発時の依存の自分を
  外すなど)。T0183 で `ugoe` (SDK) を no_std にするとき、`ugoe-format` の依存に `default-features = false` を書き、`std` の feature から有効にする。
