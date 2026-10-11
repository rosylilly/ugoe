---
id: T0198
title: "SDK の誤りと警告の種類ごとの見本を SDK の中の網羅の match で作り、wasm・C ABI の写しの検査が新しい種類に気づけるようにする"
status: done
milestone: M9
crates: [ugoe, ugoe-wasm, ugoe-ffi, docs]
depends_on: []
assignee: claude-worker-T0198
---

# T0198: SDK の誤りと警告のコードの網羅

## 目的

T0197 で SDK の誤りと警告の enum に `#[non_exhaustive]` を付けたため、`ugoe-wasm` の `codes.rs` の `guard` は、SDK の種類が増えたことを網羅の match で気づけなくなった (クレートの外から網羅できない)。網羅の検査を SDK の中に移し、wasm (と C ABI の誤りのコードの写しがあればそれも) が使う。

## 受け入れ条件

- [x] `ugoe` (か、enum を持つクレート) に、誤りと警告の種類ごとに見本を 1 つずつ返す関数を置いた (`#[doc(hidden)]`。網羅の match で書くので、種類を足すとコンパイルが落ちて見本の追加に気づく)
- [x] `ugoe-wasm` の `codes.rs` の `guard` (と、写しがあれば C ABI の側) が、この見本をすべてコードに変換できることをテストで確かめる形にした (知らない種類のコードの扱いも)
- [x] T0197 で足した「SDK に種類を足したら見本も足す」のコメントを、新しい形に合わせて直した
- [x] `just ci`・`just wasm-check`・`just nostd-check`・`just conformance` が通る

## 参考

- T0197 のメモ (弱くなった点)、crates/ugoe-wasm/src/codes.rs、ADR 0029 (誤りのコード)

## メモ

- 元の候補: T0197。オーケストレーターが起票した (2026-10-12)。

### やったこと (claude-worker-T0198)

- 見本の関数 (`#[doc(hidden)] pub fn samples() -> Vec<Self>`。中で網羅の `match` をし、種類を足すとコンパイルが落ちる):
  `ugoe::LoadError`・`DressUpError`・`LoadWarning` と、それらが包むランタイムの `CompileError`・`ComposeError`
  (`#[non_exhaustive]` なので `ugoe` からは網羅できず、enum を持つ `ugoe-runtime` に置いた。`crates:` の外だが、
  受け入れ条件の「enum を持つクレート」に当たる小さな追加)。`LoadError` のファイルの失敗 (`FormatError`) は種類が多く
  ほとんどが `invalid_file` にまとまるので、`code` ごとに 1 つだけ (`LoadError::code` で新しい `code` を割り当てたら足すと書いた)。
  テスト: `LoadError` の見本が説明にある 6 つの `code` をすべて含む、`ComposeError` の見本の `code` が種類ごとに違う。
- wasm (`crates/ugoe-wasm/src/codes.rs`): 手書きの SDK の見本を消して `samples()` を使う形にした。`guard` は wasm の種類だけを網羅する。
  新しいテスト `sdk_samples_keep_their_codes` で、すべての SDK の見本が SDK の `code` と補足の値のキーのまま JS の値になることを確かめる。
  知らない種類 (SDK に足した種類) は `codes.json` に無いので `codes_json_lists_every_code_and_field` が落ちる。`codes.json` は変わらない。
- C ABI (`crates/ugoe-ffi/src/codes.rs`。新規、テストだけ): すべての見本が `ugoe_error_kind`・補足の値・戻り値になり、README の表
  (読み込みの失敗・着せ替え・読み込みの警告の節) に載っていることを確かめる。読み込みの失敗の戻り値は種類ごとの表で決め、
  知らない種類は「戻り値を決める」とパニックで落とす。テストのため `Warning::new` を `pub(crate)` にし、テストだけの読み出しを足した。
- コメント: `codes.rs` の頭と `guard` の「SDK に種類を足したら見本も足す」を、`samples()` を使う形に直した。
  文書: `docs/release.md` の `#[non_exhaustive]` の項に `samples()` の一言、`crates/ugoe-ffi/README.md` にテストの置き場所。
- 検査: `just ci`・`just wasm-check`・`just nostd-check`・`just conformance` が通った。

### やらなかったこと

- `FormatError` (`ugoe-format`) と wasm の `RenderError` の見本は網羅にしていない (ほかの種類が `invalid_file`・`texture` にまとまるため。前と同じ)。
- Unity (`UgoeException.cs`)・Godot・`@ugoe/runtime` の TS の写しは触っていない (TS は前から `codes.json` で検査される)。

### 起票の候補

- なし。
