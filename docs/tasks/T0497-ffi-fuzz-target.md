---
id: T0497
title: "fuzz: C ABI (ugoe-ffi) の入口に直接入力を渡す fuzz のターゲットを足す (判断が要る)"
status: done
milestone: M11
crates: [fuzz, ugoe-ffi, .github/workflows, scripts, justfile, docs, AGENTS.md]
depends_on: []
assignee: claude-worker-T0497
---

# T0497: fuzz: C ABI (ugoe-ffi) の入口に直接入力を渡す fuzz のターゲットを足す (判断が要る)

## 目的

T0188 の fuzz のターゲット (`bundle`・`module`・`image`・`json`) は、Rust の API (`ugoe`・`ugoe-format`) に入力を渡す。C ABI (`ugoe-ffi`) は
読み込み (バイト列と長さ)・パラメータの名前 (C の文字列)・番号・数を受け取り、自分で確かめてから Rust の API に渡す。std の無い形では
パニックは `abort` になる (T0491・T0494) ので、C ABI の入口の確かめの漏れを fuzz で探す。

**判断が要る** (`unsafe` の扱い)。C ABI の関数は `unsafe extern "C" fn` で、呼ぶには `unsafe` が要る。`fuzz/Cargo.toml` は
`unsafe_code = "deny"` で、AGENTS.md は `unsafe` を `ugoe-ffi` と仮想カメラのクレートにだけ許している。作業者は下の案から勝手に選ばず、
利用者に確かめてから進める。

### 案

- **A. fuzz のターゲットで `unsafe` を許す:** `fuzz/` (ワークスペースの外・配布しない) を `unsafe` の例外に足し、ターゲットから C ABI の関数を
  直接呼ぶ (C のホストと同じ呼び方)。AGENTS.md と ADR 0032 に例外を書く。
- **B. `ugoe-ffi` に fuzz 用の安全な入口を置く:** `ugoe-ffi` に feature (例: `fuzzing`。ヘッダには出さない。T0494 の `panic-test` と同じ扱い) で
  安全な Rust の関数 (入力のバイト列を読み込み・問い合わせ・解放の列に分けて C ABI の関数を呼ぶ) を足し、ターゲットはそれを呼ぶ。
  `unsafe` は `ugoe-ffi` の中に留まる。
- **C. 足さない:** C ABI の入口は単体テストと適合テストに任せる。

### 推奨

B。`unsafe` の例外を増やさずに済み、入口の列 (読み込み → 問い合わせ → `update` → 解放) を `ugoe-ffi` の中で C のホストと同じ形で回せる。

## 受け入れ条件

- [x] 利用者に案を示して方針を決め、決めたことと理由をメモと docs/testing.md#fuzz (A なら AGENTS.md と ADR 0032 も) に書いた
- [x] 決めた方針で、C ABI の読み込み (`.ugoe` のバイト列。モジュールも) と、読めたものへの問い合わせ・パラメータの設定 (番号・名前・NaN と無限大) を
      入力から組み立てるターゲットを足した。std がある形のパニック (`UGOE_ERROR_PANIC` に変わる) も fuzz が見つけられること
      (libfuzzer-sys のパニックのフックが `catch_unwind` より先に `abort` するか) を確かめ、メモに書いた (C ならこの項目は不要)
- [x] fuzz.yml の matrix・`just fuzz all`・`scripts/check-licenses.mjs --fuzz` (依存が増えたら) に新しいターゲットを入れ、手元で回した時間と
      見つかったもの (直したなら、その入力を `ugoe-ffi` のテストに足したこと) をメモに書いた
- [x] `just fuzz-check` と `just ci` が通る

## 参考

- T0188 のメモ (起票の候補)、docs/adr/0032-fuzzing-with-cargo-fuzz.md、fuzz/README.md、fuzz/Cargo.toml (`unsafe_code = "deny"`)
- crates/ugoe-ffi/README.md、docs/portability.md#パニックしない

## メモ

- T0068 で起票した (元の候補: T0188 (`ugoe-ffi` の C ABI の入口に直接入力を渡す fuzz のターゲット))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 判断 (2026-10-07): 利用者が案 B (ugoe-ffi に fuzz 用の feature で安全な Rust の入口を置き、ターゲットはそれを呼ぶ。unsafe の例外を増やさない) を選んだ。
- やったこと (案 B):
  - `ugoe-ffi` に feature `fuzzing` (ヘッダに出さない。`/// cbindgen:ignore` のモジュール。`panic-test` と同じく配るライブラリでは有効にしない) と
    安全な入口 `ugoe_ffi::fuzzing::run(input, dress_up_base) -> Result<(), Failure>` を足した (`crates/ugoe-ffi/src/fuzzing.rs`)。
    入力を `[ファイル] [SCRIPT_MARKER] [台本]` に分け、ファイルをバンドル (既定の上限・小さな上限・読めなければモデルの JSON) と
    モジュール (読めたら `dress-up-base.ugoe` に取り付ける) として C ABI で読み、すべての問い合わせ (範囲の中と 1 つ外の番号) を呼んだ後、
    台本の 1 バイトごとに 24 種の操作 (パラメータの番号・名前での設定、NaN と無限大、`update`、モーション、表情、リップシンク、段、
    受け手の `follow`、トラッキング、ポーズ、当たり判定、物理とエフェクトの JSON、フレームの借用と写し (ちょうど・小さすぎ・NULL のバッファ)、
    ライティング、着せ替え、NULL の puppet など) を呼ぶ。`UgoeError` を渡す・NULL にするも台本で選ぶ。借りた文字列と配列はすべて読み
    (cargo fuzz は AddressSanitizer)、範囲の外の番号 (描画順・マスク・インデックス・ポーズのパーツ・テクスチャ) や長さの食い違いも `Failure` にする。
  - `fuzz/fuzz_targets/ffi.rs` (`fuzz/` は `unsafe_code = "deny"` のまま)、種 `fuzz/seeds/ffi/` (assets/compat のファイルに台本を付けた 5 つ)、
    `fuzz/run.mjs` の TARGETS (`just fuzz ffi`・`just fuzz all`)、fuzz.yml の matrix、docs/testing.md#fuzz・fuzz/README.md・ugoe-ffi の README・
    AGENTS.md・justfile のコメント、ci.yml の fuzz-lint のコメント (T0072 の記述に ugoe-ffi を足した)。
  - stable の再発防止: `crates/ugoe-ffi/tests/fuzzing.rs` (`required-features = ["fuzzing"]`) が assets/compat と assets/samples の全ファイルに
    決まった台本 (全操作・端の値・疑似乱数) を付けて回す。`scripts/ci-groups.mjs` の `bindings` に `ugoe-ffi/fuzzing` を足した (`just crate ugoe-ffi` と CI で流れる)。
  - 依存は増えていない (fuzz/Cargo.lock に path の `ugoe-ffi` が入っただけ。serde_json は既にあった)。`scripts/check-licenses.mjs --fuzz` は変更不要で通る。
- std がある形のパニックが fuzz で見つかるか: 確かめた。`ugoe_puppet_set_lip_sync_level` にわざと `panic!` を入れて (コミットしていない) 回すと、
  libfuzzer-sys が初期化のときに入れるパニックのフック (既定のフックを呼んでから `process::abort`) が `guard` の `catch_unwind` の内側で
  abort し、`deadly signal` のクラッシュになった (スタックは `catch_panic` → `guard` → `ugoe_puppet_set_lip_sync_level` → `fuzzing::Runner::op`)。
  フックが無い環境で `UGOE_ERROR_PANIC` が返ったときも、入口が `Failure` を返し、ターゲットがパニックする。
- 手元で回した時間: `just fuzz ffi 120` (56 万回、cov 28858) と `just fuzz ffi 300` (83 万回、cov 32369)。見つかったものは無い (fuzz/artifacts/ffi は空)。
- 範囲の外の小さな変更: `.gitattributes` に `fuzz/seeds/** binary` を足した (台本付きの種は NUL を含むバイナリ。改行の変換をさせない)。
- やらなかったこと: std の無い形 (パニックは abort) を fuzz で回すこと (libFuzzer は std で動く。std の無い形の入口の確かめは同じコードなので std の形で足りる)。
  `ugoe_image_decode` は借りたテクスチャを一辺 1024 の上限でデコードするだけ (画像そのものは `image` のターゲット)。
- 起票の候補: 無し (強いて言えば、C ABI に関数を足したときに `fuzzing.rs` の操作へ足し忘れないよう、`just refs` の C ABI の過不足の検査と同じ仕組みで
  `fuzzing.rs` が呼ぶ関数の一覧とヘッダを比べる検査)。

