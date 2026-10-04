---
id: T0150
title: ugoe-format: 読み込みの資源の上限を利用者が指定できるようにする (展開の合計・1 チャンク)
status: done
milestone: M9
crates: [ugoe-format, docs, spec]
depends_on: [T0138]
assignee: claude-worker-T0150
---

# T0150: ugoe-format: 読み込みの資源の上限を利用者が指定できるようにする (展開の合計・1 チャンク)

## 目的

T0138 (ugoe-format のレビュー) で見つけた、展開の爆発でメモリを使い切る問題を直す。

圧縮したチャンクの展開後の上限は 1 チャンクあたり 256 MiB (`MAX_DECOMPRESSED_CHUNK_LEN`) だが、ファイル全体の合計には上限がない。
zstd は同じバイトの並びを数バイトのブロックで表せるので、256 MiB に展開されるチャンクは数 KiB で作れる。
`MOTN` (名前が 256 MiB の文字列のモーション) や、zstd のフラグを立てた `TEXR` / `NRML` (読み込みは画像のチャンクも展開する) を
数百個並べると、1 MiB に満たないファイルで、読み込みの途中で数十 GiB を確保して止まる
(テクスチャの数の検査は、すべての `TEXR` を読んだ後に行う)。外から来るファイルでクラッシュするので重大度は高。

### 方針 (2026-10-04、利用者の判断)

**上限はファイル形式に決めず、読む側の方針として持つ。** ファイルそのものに上限を定めると、使える範囲を狭めて可用性を下げるだけで益がない。
読み込み器 (SDK など) が展開の上限を持ち、超えそうなら展開の途中でやめて、エラーとして返す。上限の値は利用者が指定でき、既定値を持つ。

## 受け入れ条件

- [x] `ugoe-format` に読み込みの上限の型 (例: `ReadLimits { max_chunk_decompressed, max_total_decompressed }`) と、それを受け取る読み込みの関数を足した
  - 既定値 (`ReadLimits::default()`) を持ち、今の `read_bundle` などは既定値で読む (今の呼び出し側は変えなくてよい)
  - 1 チャンクの上限は今の `MAX_DECOMPRESSED_CHUNK_LEN` の代わりになる。合計は、展開したバイトを数えながら上限を超えたら途中でやめる (先に確保しない)
  - 圧縮していないチャンクも合計に数えるかを決めて doc に書いた (ファイルの大きさ自体は呼び出し側が分かるので、数えなくてもよい)
- [x] 上限を超えたときのエラー (`FormatError` の種類) を足し、どの上限を超えたか (チャンク・合計) と、そのときの値を持つ。ADR 0029 のコードを付けた
- [x] 公開仕様 (`spec/bundle.md` の Compression) と `docs/spec/file-formats.md` を、この方針に合わせて書き直した
  - ファイル全体の展開後の大きさに上限を設けない
  - 読み込み器は資源の上限を設けてよい (MAY)。超えたら、落ちたりメモリを使い切ったりせずにエラーとして読み込みをやめなければならない (MUST)
  - 今の「読み込み器は 1 チャンク少なくとも 256 MiB に対応しなければならない (MUST)」は、どの読み込み器でも読める下限の保証として残すかを決める
  - 今の「書き出しは展開後 256 MiB を超えるチャンクを圧縮してはならない (MUST NOT)」は、ファイルを狭める制約なので SHOULD NOT に緩める案で検討する
    (緩めると、既定の上限の読み込み器では読めないファイルが仕様に適合しうる。その場合も上のとおりエラーになる)
  - `docs/spec/file-formats.md` の「展開後の大きさの上限は 1 チャンクあたり 256 MiB です」を、参照実装の既定値の説明に直す
- [x] 小さなファイルが合計の上限を超えて展開されることを、エラーで拒否するテストがある (上限は引数で小さくし、大きな確保をしない)
- [x] `just ci` と `just spec` が通る

## 参考

- docs/spec/file-formats.md#ペイロードのエンコード、spec/bundle.md#compression
- `crates/ugoe-format/src/bundle.rs` の `Entry::payload` / `payload_within` と `read_with_known`
- docs/tasks/T0138-review-format.md のメモ
- 後続: SDK・C ABI・wasm・バインディングから上限を指定する口は T0152

## メモ

- 2026-10-04: 当初は「仕様でファイル全体の上限を決める」案だったが、利用者の判断で「読む側の上限を指定できるようにする」方針に書き直した。
- 2026-10-04 (claude-worker-T0150): 実装した。
  - **やったこと**
    - `ugoe-format`: `ReadLimits { max_chunk_decompressed, max_total_decompressed }` (`#[non_exhaustive]`、`u64`。既定は 256 MiB・1 GiB。`with_*` で既定値から変える) と
      `read_bundle_with(bytes, limits)` / `read_module_with(bytes, limits)` を足した。`read_bundle` などは既定値で読む (呼び出し側は変えていない)。
      展開は内部の `Payloads` が、1 チャンクの上限と合計の残りの小さいほう + 1 バイトまでしか読まず、超えたら途中でやめる。機能の宣言・本体の先読み・各チャンクのすべてが同じ合計を使う。
    - 合計に数えるのは **圧縮したチャンクを展開したバイトだけ** (圧縮していないチャンクは入力の中を指すだけで、入力の長さで抑えられる)。展開に失敗したチャンクも数えない。doc と仕様書に書いた。
    - `FormatError::DecompressedTooLarge { tag }` を `ReadLimitExceeded { tag, limit: ReadLimit, max }` に置き換えた (`ReadLimit` は `ChunkDecompressed` / `TotalDecompressed`、`max` は超えた上限の値)。
      使っていたのはクレートの中と `tests/spec_bundles.rs` だけ。展開の途中でやめるので、実際の大きさは持てない。
    - ADR 0029 のコード: `format.chunk_limit_exceeded` / `format.total_limit_exceeded` (引数 `chunk`・`max`)。どの上限かで文が変わるので、引数でなくコードを 2 つにした。
      ファイルが壊れているとは限らないので `format.corrupt` と分けた。
    - 仕様: `spec/bundle.md` に Resource limits の節を足し、Compression を書き直した (形式は上限を決めない・読み込み器は MAY・超えたらエラーで止まる MUST・展開しながら気づく SHOULD・
      1 チャンクの上限の既定値は 256 MiB 以上 MUST (下限の保証として残した)・合計の既定値もそれ以上 SHOULD・書き出しの 256 MiB 超の圧縮は MUST NOT → SHOULD NOT・参照実装の既定値は Informative)。
      Reading の手順 5 に上限を超える展開を足した。`docs/spec/file-formats.md` に「読み込みの上限」の節を足し、256 MiB の文を参照実装の既定値の説明に直した。`docs/spec/public-spec.md` の対応表も直した。
    - テスト: 1 チャンクの上限・合計の上限 (残りで判定・失敗は数えない・無圧縮は数えない)・**64 KiB 未満のファイルが合計 1 MiB の上限で `TEXR` の途中で拒否される** (上限は引数で小さくした)・
      圧縮したバンドルが合計ちょうどで読めて 1 少ないと拒否される・無圧縮のバンドルは上限 0 でも読める・モジュールにも効く・メッセージのコード。
  - **範囲外の変更 (小さいので行った)**: `ugoe-editor-core` の `message_codes.rs` / `message_tests.rs` (コードの対応)、`docs/editor.md` のコードの一覧、`apps/editor/locales/{en,ja}.json` の訳。
  - **確かめたこと**: `just ci`・`just spec`・`just wasm-check` が通った。
  - **やらなかったこと**: SDK・C ABI・wasm・`@ugoe/runtime` から上限を指定する口 (T0152)。ビューア・スタジオの画面の訳 (いまは `format.*` のコードをエディタだけが訳す)。
    PNG / KTX2 のデコードの大きさの上限 (T0138 の横断の所見。ugoe-format は画像をデコードしない)。
  - **起票の候補**: なし (T0152 が既にある。T0152 では `ReadLimits` が `#[non_exhaustive]` なので、`with_*` か既定値からの書き換えで組み立てる)。
