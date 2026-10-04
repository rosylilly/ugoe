---
id: T0530
title: コードレビュー: ugoe-cli
status: done
milestone: M9
crates: [ugoe-cli, docs]
depends_on: []
assignee: claude-worker-T0530
---

# T0530: コードレビュー: ugoe-cli

## 目的

ugoe-cli のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-cli` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 編集のロジックが CLI に直接書かれていないか (`Command` を通す)
  - 引数の検証・終了コード・エラーの文言、ヘルプと docs の一致
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0556〜T0560 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

`crates/ugoe-cli` の `src/` (main・fixtures・physics・sample とその下の animations・lit・paint・dress_up、`animations/` の JSON) と
`tests/` (cli と GPU の 5 本) をすべて読んだ。CLI の編集は、`import-psd` も `sample avatar` (`--lit` も) も `ugoe_editor_core::Command`
を通していた。`sample dress-up` はモデルを直接組み立てるが、新しいモデルをコードで作る見本 (フィクスチャと同じ扱い) で、ドキュメントの
編集ではないので違反とはしない。`--help` と README・docs の `ugoe` のオプションの記述を突き合わせた (docs にある全オプションが実在する)。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/main.rs` の `read_container` | バンドルの物理が検証に通らない (`FormatError::InvalidPhysics`) と、`validate` は「physics settings failed validation with N error(s)」だけを出し、モデルやモジュールと違って各エラーを書かない (README は「バンドルも同じ」) | 直した (`InvalidPhysics` も各エラーを `error: ...` で書く。`tests/cli.rs` にテストを追加) |
| 中 | `src/main.rs` の `run_render` | `render --module ... --physics <json> --time ...` で、モジュールの物理のリグが消える (モジュールを付けた後で `ugoe_headless::prepare` が物理を `--physics` に丸ごと差し替える)。`validate` は `--physics` を先に当ててからモジュールを付けるので食い違う | 直した (時間を進めるときは `--physics` を先に読んでからモジュールを付ける。準備を `prepare_render` に分け、単体テストを追加。README の `--physics` に追記) |
| 低 | `src/fixtures.rs` | `fixture:<name>` が受け付けるのは `ugoe_model::fixtures` のモデルの一部だけ (`dynamic_hair`・`physics_hair`・`blend_shapes`・`dress_up_base` など 8 つが無い)。README は「関数名」と書く | 直した (`Model` を返すものをすべて載せ、テストに追加) |
| 低 | `README.md` | `sample avatar --atlas` がオプションの表に無い | 直した |
| 低 | `src/sample.rs` の先頭 | 「`Model.avatar` と当たり判定は設定するコマンドがまだないので直接書く」は古い (今は `SetAvatarProfile`・`SetHitAreas` を使う) | 直した |
| 低 | `src/main.rs` の `load`・`read_modules`・`run_build`・`run_import_psd`・`run_sample_avatar` | 外から渡したパスを `fs::read` で読むので、`/dev/zero` を渡すと読み続けてメモリを使い果たす (T0529 の ugoe-mcp と同じ形) | 見送り (CLI は利用者が自分で起動してパスを渡すもので、Ctrl-C で止められる。`/dev/stdin` や `<(...)` の FIFO を入力にする使い方を壊したくない。サーバとして外からパスを受ける ugoe-mcp とは事情が違う) |
| 低 | `src/main.rs` の `attach_modules` | `--module` は「入力は `.ugoe` バンドル」だが、テクスチャを持たない JSON のモデルにはそのまま付けられる (テクスチャの数でしか判別しない) | 見送り (付けた結果は正しく描け、害がない) |
| 低 | `src/main.rs` の `run_render` | `-o` の書き込み先 (無いディレクトリなど) の誤りは、描き終えてから分かる | 見送り (GPU の無い環境でもアダプタの誤りより先に出したいなら、出力の前検査を足す。今は軽微) |
| 低 | `src/main.rs` の `INPUT_HELP`・`MODULE_INPUT_HELP` | `INPUT` の説明だけ英語で、ほかのヘルプ (doc コメント) は日本語 | 見送り (CLI の文言の言語は決まっていない。好みの範囲) |
| 低 | `tests/m2_render_gpu.rs`・`m3_render_gpu.rs`・`lighting_render_gpu.rs` | 出力を一時ディレクトリの固定の名前 (`ugoe-m2-render` など) に書くので、同じマシンで並行して流すと上書きし合う | 見送り (目で確かめるために固定の場所に残す意図で、`gpu-tests` は手で流す) |
| 低 | `src/sample/paint.rs` の `Shape::polygon` | 頂点が 0 個だと `points.len() - 1` があふれてパニックする | 見送り (見本の絵を描く内部の関数で、空の多角形を渡す所はない) |

起票したタスクは無い (直さない所見はどれも小さく、範囲の外の変更や設計の判断を要しない)。T0556〜T0560 の枠は使っていない。

`just ci` は通った (2026-10-04)。

### 横断

- **外から読むバンドルの検証エラーの出し方:** `FormatError` には `Invalid`・`InvalidModule`・`InvalidPhysics` の 3 つの「エラーの列」を持つ
  バリアントがある。読む側 (CLI・エディタ・ビューア・MCP) が一部のバリアントだけを展開して、残りを件数だけで見せていないか。
  `FormatError` に「エラーの列を返す」メソッドを持たせれば取りこぼしが減る (ugoe-format の側の判断)。
- **物理の差し替えとモジュールの合成の順:** `--physics` (や同じ意味の設定) で本体の物理を差し替える所と、モジュールのリグを加える所が
  別々にあると、順によってモジュールのリグが消える。ugoe-headless の `prepare` は `Scene` の物理を `--physics` で丸ごと置き換えるので、
  ほかに `prepare` を使う所 (ugoe-mcp の `render_png`) でモジュールを付けるなら同じ形に注意する。
- **非正規ファイルの読み込み:** T0529 と同じく `fs::read` で外から渡したパスを読む所は多い (ugoe-headless の `--texture`・`--motion`・
  `--expression`・`--physics` も)。サーバやアプリの IPC として外から受けるパスは正規ファイルだけにし、利用者が自分で渡す CLI は
  FIFO を読めるままにする、という線引きを揃えるとよい。
