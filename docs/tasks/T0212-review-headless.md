---
id: T0212
title: コードレビュー: ugoe-headless
status: done
milestone: M9
crates: [ugoe-headless, docs]
depends_on: []
assignee: claude-worker-T0212
---

# T0212: コードレビュー: ugoe-headless

## 目的

ugoe-headless のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-headless` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - GPU が無い・アダプタが選べない環境でのエラー、大きさと fit の境界 (0・巨大な大きさ)
  - CLI と MCP サーバの両方から呼ばれる前提 (スレッド・時間の進め方) が崩れていないか
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0218〜T0222 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/image.rs` (`decode_png`) | PNG の見出し (IHDR) の大きさだけで出力のバッファを確保する。65535x65535 で画素が数バイトの PNG で約 17GB を求め、環境によっては abort する (バンドルは外から来るデータ。png の `Limits` はデコーダの中のバッファにしか効かない) | 直した (一辺 `MAX_SIZE` = 16384 を超えたら確保の前にエラー。テストを追加) |
| 高 | `src/animate.rs` (`advance`) | `floor(time * fps)` 回の `update` に上限が無く、MCP サーバに `time: 1e9` を渡されると止まり続ける (`fps` が巨大でも同じ) | 直した (`MAX_UPDATES` = 60 fps で 1 時間。超えたらエラー。テストを追加) |
| 中 | `src/lib.rs` (`prepare`) | モデルの検査 (コンパイル) の前にテクスチャを揃えるので、検査していないモデル (`Scene::from_model`) の `mesh.texture = u32::MAX` で 40 億枚の仮のテクスチャを作り始める | 直した (コンパイルを先に行う。警告の順は変えない。テストを追加。`u32::MAX` の場合は直す前に流すとメモリを使い果たすので、直す前は番号 1 と `--texture` の場合で失敗を確かめた) |
| 中 | `src/view.rs` (`output_size`) | `size`・`width`・`height` を明示したときは 1..=`MAX_SIZE` を確かめない。MCP サーバは `args` を通さずに渡すので、`width: 0, height: 5` が GPU まで届く (0 の大きさの描画先・読み戻しの `chunks_exact(0)`) | 直した (どの一辺も確かめる。テストを追加) |
| 中 | `src/textures.rs`・`src/animate.rs` | `--texture`・`--physics`・モーションと表情の JSON のパスを `fs::read` でそのまま読む。`x.json` という FIFO や `/dev/zero` へのリンクで止まる・メモリを使い果たす (MCP サーバの `--root` が無いときはパスがそのまま来る。T0529 と同じ形) | 直した (`read_file`: 通常のファイルでなければ読まない。テストを追加) |
| 中 | `src/yuv.rs` (`convert`) | `Rgba8` の画素の数が大きさと合わないと添字の範囲外で panic する (公開の関数で、フィールドも公開) | 直した (エラーにする。テストを追加) |
| 中 | `tests/resident_gpu.rs` | `Resident::set_backdrop` のテストが無い (スタジオの GPU テストも通らない) | 直した (背景の画像・外す・大きさの違い。GPU で通ることを確かめた) |
| 中 | `src/lib.rs` | `prepare` のテストが無い (警告・時間を進めるか・`fps` の検査) | 直した (テストを追加) |
| 中 | `src/gpu.rs`・`src/resident.rs` | 読み戻しを `wait_indefinitely` で待つので、ドライバが止まると MCP サーバの描画が戻らない | T0218 を起票 |
| 中 | `src/resident.rs` | 計算シェーダの無いアダプタで CPU で YUV に変換する道が、どのテストでも通らない | T0220 を起票 |
| 低 | crates/ugoe-cli/README.md・crates/ugoe-mcp/src/tools.rs | 上で足した上限が `ugoe render` の規則の文書に無い。`render_png` のスキーマは `width` の `maximum: 4096` を書くがサーバは確かめない | T0219 を起票 (範囲の外) |
| 低 | `src/gpu.rs` (`new_renderer`) | テクスチャの大きさの上限の検査が ugoe-wasm と重複 | 見送り (T0213 で扱う) |
| 低 | `src/resident.rs` (`poll`) | 失敗したスロットが 1 つでもあると、読めた最新の 1 枚を捨ててエラーを返す | 見送り (失敗は珍しく、次の `poll` で次の 1 枚が来る。呼び出し側はエラーを数えて続ける) |
| 低 | `src/lib.rs`・`src/resident.rs` | `Error` の variant と一部の公開のフィールド・メソッドに doc コメントが無い | 直した |
| 低 | `src/gpu.rs` | `Gpu::new` の知らないバックエンドの検査にテストが無い | 直した (GPU を使わないテスト) |
| 低 | `src/lib.rs`・`src/view.rs` | エラーの文言が CLI のオプション名 (`--scale`・`--texture`・`--physics`) を含み、MCP サーバの利用者にも同じ文が出る | 見送り (MCP サーバはそれらの引数を渡さないので出ない) |

確かめたこと (直すものが無かったもの):

- GPU が無い・アダプタが選べないとき: `Gpu::new` は `Error::NoAdapter` / `Error::Device` を返し、インスタンスは破棄しない (dzn の SIGSEGV を避ける方針どおり)。
  知らないバックエンドの名前はインスタンスを作る前にエラー。
- スレッド: エラースコープは `Gpu::scoped` の中で積んで同じスレッドで外すので、MCP サーバ (錠の中の 1 スレッド) とスタジオ (受け手のスレッド) の
  どちらから呼んでも崩れない。`Resident` のスロットの状態は `Arc<AtomicU8>` で、`map_async` のコールバックがどのスレッドで走ってもよい。
- 時間の進め方: CLI と MCP サーバは同じ `prepare` → `advance` を通り、固定の刻みなので同じ引数から同じ絵になる。
- `width * 4` などの u32 の掛け算: どれも `new_renderer` のデバイスの上限の検査の後か、`MAX_SIZE` の検査の後なので、あふれない
  (`image::cover`・`yuv::frame_bytes` は usize で掛ける)。
- `just ci` は通った。`cargo test -p ugoe-headless --features gpu-tests`・`cargo test -p ugoe-cli -p ugoe-mcp` の `gpu-tests` も通した。
- 挙動の変更: 一辺が 16384 を超える PNG (テクスチャ・法線マップ・スタジオの背景の画像) はエラーになる。

### 横断

- **見出しの大きさで確保する PNG のデコード:** `crates/ugoe-wasm/src/load.rs` の `decode_png` は headless の写しで、同じ確保の問題がある
  (worker-T0424 に伝えた)。PNG のデコードがツール層とランタイム層の 2 か所にあるので、ランタイム層 (`ugoe-format` など) に 1 つにまとめる候補。
- **外から来るパスの読み込み:** `fs::read` / `read_to_string` に装置・FIFO を渡すと止まる形は、T0529 (ugoe-mcp) と同じ。
  パスを受け取るほかのクレート (ugoe-cli・エディタやスタジオの IPC) にもありそう。
- **時間や回数の引数の上限:** 外から秒数や fps を受け取り、その回数だけ回す処理 (モーションの書き出し・動画の書き出しなど) には上限が要る。
- **公開の関数に渡す明示の値の検査:** CLI の引数の検査 (`args`) を通らない呼び出し側 (MCP サーバ・アプリ) があるので、検査はライブラリの関数の側に置く。
