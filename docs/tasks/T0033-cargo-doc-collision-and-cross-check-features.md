---
id: T0033
title: cargo doc の出力先の衝突・wasm32 の cargo doc・cross-check の --all-features
status: done
milestone: M9
crates: [ugoe-cli, scripts, justfile, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0033
---

# T0033: cargo doc の出力先の衝突・wasm32 の cargo doc・cross-check の --all-features

## 目的

T0027 で `just doc-check`・`just doc-group` と windows-gnu の clippy を CI に入れたとき、次の 3 つが残った (T0027 の候補)。

1. `cargo doc --workspace` で、`ugoe-cli` の bin `ugoe` と `ugoe` クレートの lib のドキュメントの出力先が衝突する
   (`output filename collision at target/doc/ugoe/index.html`)。rustdoc の警告ではないので `-D warnings` では止まらず、
   `just doc` で開く `ugoe` のページが CLI のものに上書きされることがある。
2. `scripts/cross-check.mjs` の `features` は名前を並べて書くので、feature を足したときに書き漏れうる。「すべての feature」(`--all-features`) を書けるようにする。
3. wasm32 だけでビルドするコード (`WebPuppet` など、`ugoe-wasm` の `src/web.rs`・`src/web/`) の doc コメントは、今の `doc-check` (ホストのターゲット) では検査されない。

## 受け入れ条件

- [x] `cargo doc --workspace --no-deps` が出力先の衝突の警告を出さない (bin に `doc = false` を付けるなど)。`just doc` で開く `ugoe` のページが SDK のものである
- [x] `scripts/cross-check.mjs` の検査の組で `--all-features` を指定でき、今の書き並べた feature の組のうち置き換えられるものを置き換えた (テストは `just script-check`)
- [x] wasm32 のターゲットで `cargo doc -p ugoe-wasm --no-deps` を `-D warnings` で流す検査が just のレシピと CI にある (今の警告は直した)
- [x] docs/testing.md の検査の説明を更新した
- [x] `just ci` が通る

## 参考

- T0027 のメモ (起票の候補)
- docs/testing.md#ci-のジョブ、docs/testing.md#別のターゲットの検査

## メモ

- T0032 で起票した (元の候補: T0027 の 3 つ)。
- やったこと:
  - 出力先の衝突: `crates/ugoe-cli/Cargo.toml` の bin `ugoe` に `doc = false` を付けた。`just doc-check` で
    `output filename collision` が出なくなり、`target/doc/ugoe/index.html` は SDK のページになった (生成物は 26 → 25 ファイル)。
    CLI の bin の doc コメントは rustdoc で検査されなくなるが、bin の非公開の項目はもともと API ドキュメントに出ず、使い方は
    `ugoe --help` と README にあるので失うものは小さい。
  - cross-check: `CHECKS` の定義に `allFeatures: true` (`--all-features`) を足した。`features` と両方を書くと `validate` が誤りにする。
    `apps` (`ugoe-studio/gpu-tests` だけ) と `tools` (`ugoe-ai/gpu-tests`・`ugoe-mcp/gpu-tests`) は、選んだクレートの feature が
    それですべてなので両方 `allFeatures` に置き換えた (`vcam` はもともと feature 無し)。`just cross-check tools apps` が手元で通り、
    引数が `--all-features` になったことを確かめた。
  - wasm32 の rustdoc: `just wasm-check` に `RUSTDOCFLAGS="-D warnings" cargo doc -p ugoe-wasm --no-deps --target wasm32-unknown-unknown`
    を足した (CI は `rust (bindings)` の `wasm32` のステップがそのまま流す。ci.yml にはコメントだけ足した)。今の警告は 0 件だった
    (T0027 で `WebPuppet` へのリンクを外した後なので)。出力は `target/wasm32-unknown-unknown/doc/` でホストの `target/doc/` とは別。
  - docs/testing.md: 検査の表・`rust (bindings)` の行・CI のジョブの節 (bin の `doc = false` の理由、wasm32 の rustdoc)・別のターゲットの検査
    の節 (`allFeatures`) を更新した。
- T0840 で見えた `doc-check` の途中での `target/doc/ugoe` の削除の失敗について:
  - 原因はこの出力先の衝突。cargo は rustdoc を動かす前に、古い項目を残さないよう `target/doc/<クレート名>` を消す。`cargo doc --workspace`
    では bin `ugoe` (ugoe-cli) と lib `ugoe` の 2 つの rustdoc が並行して同じ `target/doc/ugoe` を使うので、後から始まった方の削除が
    先に走っている方の出力と競合し、削除の失敗 (書き込み中で消せない) か、片方のページの消失になる (cargo#6313 の「known bug」)。
    タイミング次第なので、流し直すと通ることが多い。
  - mbx の managed target は共有していない: `target` は `~/.cache/mbx/targets/v1/<ハッシュ>` へのリンクで、ハッシュはワークスペースの
    パスごと (`<ハッシュ>.json` の `workspace_root`)。main・T0033・T0840 の worktree はそれぞれ別のディレクトリを指していた。
    mbx の置き場が持つのはコンパイルの成果物で、`target/doc` は worktree ごとの managed target の中にあるので、ほかの worktree の
    cargo doc が消すことはない。
  - 対策は `doc = false` で衝突を無くすこと (上)。修正後の `just doc-check` と `just ci` で衝突の警告は出ていない。
- `just script-check` について: `scripts/cross-check.test.mjs` はもともと `just groups` (`just ci` と CI の `rust-checks` に入る) が流していて、
  `just script-check` の対象ではない (docs/testing.md の「`just groups`・`just app-build-check` が流すテストは、それぞれのレシピの側」の方針)。
  受け入れ条件の「テストは `just script-check`」は `just groups` で満たした。
- 確かめた範囲: Linux (WSL2) で `just doc-check`・`just wasm-check`・`just cross-check tools apps`・`just groups`・`just ci` が通った。
  ci.yml はコメントの追加だけで、ステップは変えていない。
- 起票の候補: なし。

