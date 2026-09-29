---
id: T0511
title: ヘッドレス描画を CLI と MCP サーバで共有する
status: done
milestone: M6
crates: [ugoe-cli, ugoe-mcp, ugoe-headless, scripts, Cargo.toml, commitlint.config.mjs, docs, AGENTS.md]
depends_on: [T0508, T0114]
assignee: claude-worker-T0511
---

# T0511: ヘッドレス描画を CLI と MCP サーバで共有する

## 目的

`ugoe render` の中身 (ヘッドレスの GPU の用意・出力の大きさと `fit`・PNG の書き出し・時間を進める処理) を
ライブラリとして切り出し、`ugoe-mcp` の `render_png` が子プロセスではなく同じ関数を呼ぶようにする。

## 受け入れ条件

- [x] 共有の置き場所 (ugoe-cli をライブラリにするか、新しいツール層のクレートか) を決めて書く
- [x] `ugoe render` の出力が変わらない (既存の GPU テストが通る)
- [x] `ugoe-mcp` が `ugoe` のバイナリなしで PNG を描ける。GPU のドライバの不調でサーバが落ちない手当てを残す
- [x] `just ci`・`just layers`・`just licenses` が通る

## 参考

- `docs/tasks/T0508-mcp-server.md` のメモ
- `crates/ugoe-mcp/README.md`

## メモ

- **置き場所:** 新しいツール層のクレート `ugoe-headless` を作った (ランタイム層だけに依存。`scripts/check-layers.mjs`・
  `commitlint.config.mjs` の `headless`・`docs/architecture.md`・AGENTS.md に登録。team-lead の承認で `crates:` に
  `ugoe-headless` と AGENTS.md を足した)。ugoe-cli をライブラリにしなかった理由: `ugoe-mcp` が clap・PSD 取り込み・
  見本のジェネレータ (`sample.rs`) などの CLI の中身まで連れてくることになり、ライブラリの誤りを `anyhow` で返す形も
  AGENTS.md の規約に合わない。新しいクレートのエラーは `thiserror` の `ugoe_headless::Error` (原因をたどると以前の
  `anyhow` の `Caused by` と同じ文になる)。
- **切り出したもの:** `ugoe-cli` の `render.rs`・`image.rs`・`animate.rs` と `main.rs` の `resolve_textures` をそのまま
  移し、`RenderOptions` (`ugoe render` のオプションと 1 対 1)・`Scene`・`prepare` (GPU を使わずに大きさ・ビュー・
  テクスチャ・パラメータ・時間を用意する)・`Prepared::render` にまとめた。`ugoe render` はオプションを
  `RenderOptions` に詰めて呼ぶだけ。引数の文字列の読み取り (`args`) も共有し、`Fit` は feature `clap` で `ValueEnum`。
  validate まわり (T0315) は触っていない (`main.rs` の変更は mod 宣言・use・`RenderArgs` の value_parser・`run_render`・
  移したテストだけ)。
- **出力が変わらないこと:** 変更前の HEAD でビルドした `ugoe` と、変更後の `ugoe` で同じ引数の PNG をバイト単位で比べて一致を
  確かめた (single_quad にパラメータと背景・nested_masks の scale/stretch・`--texture` の差し替え・見本のアバターの
  `--stats` と `--time 0.7`・`--time 1.3 --fps 30 --no-effects`)。標準出力と標準エラーも同じ (`--stats` の JSON を含む)。
  `cargo test -p ugoe-cli --features gpu-tests` は 34 件すべて通った (llvmpipe、Vulkan)。
- **描画中の wgpu のエラー:** `Gpu::render` を検証・メモリ不足・内部のエラースコープで囲み、`Error::Gpu` で返すように
  した (wgpu の既定の panic の代わり)。出力の画素には影響しない。
- **MCP:** `render_png` は子プロセスをやめ、`ugoe_headless` を直接呼ぶ。`--ugoe-bin`・`UGOE_BIN`・版ずれの警告を消し、
  代わりに `--backend` を足した (README を更新)。モデルは `export_bundle` → `read_bundle` をメモリで行って `Scene` にする
  (書き出した `.ugoe` を `ugoe render` で描くのと同じ中身。`Project::texture_bytes` が非公開で、editor-core は範囲外のため)。
  - 落ちない手当て: GPU は最初の描画で用意して使い回し、破棄しない (`ManuallyDrop`。dzn の破棄時の SIGSEGV を避ける)。
    アダプタ・デバイスの失敗はツールのエラーにして次の呼び出しで再試行。描画中の wgpu のエラーはエラースコープで
    エラーに。panic は `catch_unwind` で受けてツールのエラーにし、その GPU を捨てて次で作り直す。
  - 子プロセスとの比較: 子プロセスはドライバの SIGSEGV やプロセスの終了時のクラッシュからもサーバを守れたが、`ugoe` の
    実行ファイルを探す手間・版ずれ・一時ファイル・毎回の GPU の初期化 (lavapipe で数百 ms) があった。同じプロセスでは
    これらが無くなる代わりに、ドライバの中の SIGSEGV はサーバごと落ちる (MCP クライアントが起動し直し、未保存の変更は
    失われる)。ドライバがプロセスの標準出力に書くと MCP のメッセージが壊れるが、確かめた範囲 (llvmpipe の Vulkan と GL、
    dzn) では標準エラーだった。
  - 確かめたこと: `cargo test -p ugoe-mcp --features gpu-tests` の `renders_png_without_the_cli` (`PATH` を空にして起動し、
    描画を 2 回、標準入力を閉じて正常終了) が llvmpipe の Vulkan と GL で通る。dzn (`VK_DRIVER_FILES=dzn_icd.json`) では
    この環境ではアダプタが見つからず、CLI と同じ文のツールのエラーが返り、サーバは落ちなかった。
- 後続の候補: `render_png` から `FrameStats` を返す (いまは捨てている)。ドライバの不調でも落ちない形を強く求めるなら、
  描画だけを長く生きる子プロセス (同じバイナリの隠しサブコマンド) に分ける。
