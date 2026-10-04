---
id: T0156
title: ugoe クレートの参照レンダラを feature で外せるようにする
status: done
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, ugoe-headless, docs]
depends_on: [T0139]
assignee: claude-worker-T0156
---

# T0156: ugoe クレートの参照レンダラを feature で外せるようにする

## 目的

`ugoe` (crates.io に出すランタイム SDK) は `ugoe-render` (wgpu・naga) に無条件に依存し、`ugoe::render` として再エクスポートする。
エンジン独自のレンダラで描く組み込み先 (Unity・Godot・自前のエンジン) も wgpu をビルドすることになる。クレートの doc コメントは
「`render` feature 相当」と書いていたが、そうした feature は無い (T0139 で文言だけ直した)。

`render` feature (既定で有効) にして外せるようにするかを決め、するなら実装する。公開 API の形 (`ugoe::render` の有無) が変わるので、
ADR 0027 (crates.io への公開) との関係も確かめる。

## 受け入れ条件

- [x] `render` を feature にするかを決め、理由をこのタスクのメモか ADR に書いた
- [x] するなら: `default-features = false` の `ugoe` が wgpu に依存しないこと (`cargo tree`) と、ffi (`ugoe::render::MAX_LIGHTS` を使う)・wasm・headless がビルドできることを確かめた
- [x] crates/ugoe/README.md と docs/portability.md を更新した
- [x] `just ci` が通る

## 参考

- crates/ugoe/src/lib.rs・Cargo.toml
- docs/adr/0027-crates-io-publishing.md
- docs/portability.md

## メモ

- **決めたこと:** `render` を feature にし、既定で有効にした (`default = ["render"]`、`render = ["dep:ugoe-render"]`)。
  理由: 段階 B (エンジン独自のレンダラ) の Rust の組み込み先が wgpu・naga をビルドしなくて済む。既定で有効なので、
  `cargo add ugoe` だけで参照レンダラまで使え、今の利用者 (ugoe-ffi・ugoe-wasm・ugoe-headless・bindings/godot・エディタなど) の
  ビルドと API は変わらない。ADR 0027 とは衝突しない (optional な依存も crates.io に要るので、`ugoe-render` はこれまでどおり公開する。
  `just publish-check` は通る)。ADR は書かず、理由は docs/portability.md#ugoe-クレートの-feature とこのメモに書いた。
- **確かめたこと:** `cargo tree -p ugoe --no-default-features -e normal,build` に wgpu・naga が無い (既定の feature では出る)。
  `--no-default-features` の clippy (`--all-targets`) とテストが通る。ffi (`ugoe::render::MAX_LIGHTS`)・headless の clippy と
  `just wasm-check` が通る。`cargo doc -p ugoe --no-default-features` で壊れたリンクが出ない (lighting.rs の `ugoe_render::Lighting`
  へのリンクを文字だけにした。出る警告 1 件 `sanitize_dt` は main からあるもの)。
- **CI:** `--no-default-features` の検査を入れた。`just sdk-no-render-check` (clippy・テスト・`cargo tree` に wgpu・naga が無いこと) を
  足し、`just ci` と ci.yml の `rust (runtime)` ジョブで流す。docs/testing.md の CI のジョブの表も直した。
  `crates:` の外 (justfile・.github/workflows/ci.yml・docs/testing.md) を小さく触った。
- **やらなかったこと:** ugoe-ffi・bindings/godot は `ugoe::render` の光源の型 (`MAX_LIGHTS`・`Light`・`Lighting`) を使うので、
  既定の feature のままにした (外すと wgpu を省けるが、光源の型の置き場を変える必要がある)。
- `just ci` は rebase 後に通った (main で落ちていた ToolOverlay.test.ts も今回は通った)。
- **起票の候補:** 光源の型 (`Light`・`Lighting`・`MAX_LIGHTS`) を GPU に依存しないクレート (ugoe-core か `ugoe::lighting`) へ移し、
  ugoe-ffi と bindings/godot を `default-features = false` にして C ABI の共有ライブラリと GDExtension から wgpu を外す
  (Unity・Godot の配布物が小さくなる。ugoe-render は再エクスポートで互換を保つ)。
