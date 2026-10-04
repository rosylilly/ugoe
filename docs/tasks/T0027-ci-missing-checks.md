---
id: T0027
title: CI に入っていない検査を足す (script-check・cargo doc の警告・ugoe-ai と ugoe-mcp の windows-gnu の clippy)
status: done
milestone: M9
crates: [.github/workflows, justfile, scripts, ugoe-model, docs]
depends_on: []
assignee: claude-worker-T0027
---

# T0027: CI に入っていない検査を足す (script-check・cargo doc の警告・ugoe-ai と ugoe-mcp の windows-gnu の clippy)

## 目的

手元の `just ci` にはあるが、CI (`.github/workflows/`) のどのジョブでも流れていない、または CI に無い検査がある。

- `just script-check` (scripts/ の単体テスト) が `.github/workflows/ci.yml` のどのジョブにも無い (T0435 の候補)。
- `cargo doc` の警告 (`rustdoc::broken_intra_doc_links` など) を止めていない。今も `ugoe-model` の `deformer.rs:54` の `[0,1]²` が
  intra-doc link と読まれて警告になる (T0017 の候補)。
- `ugoe-ai`・`ugoe-mcp` の Windows のコードは `rust (windows)` のランナーの clippy に任せていて、`rust (cross)` の windows-gnu では検査していない (T0013 の候補)。

## 受け入れ条件

- [x] CI のいずれかのジョブで `just script-check` が流れる (どのジョブに入れたかと理由を docs/testing.md#ci-のジョブ に書いた)
- [x] `cargo doc --workspace --no-deps` の警告を拒む検査 (`RUSTDOCFLAGS=-D warnings` など) が `just` のレシピと CI にあり、今の警告 (`deformer.rs:54` ほか) を直した
- [x] `scripts/cross-check.mjs` の `CHECKS` に `ugoe-ai`・`ugoe-mcp` の windows-gnu の clippy を足した (足せない依存があれば理由をメモに書いた)
- [x] `just ci` が通る

## 参考

- T0435・T0017・T0013 のメモ (起票の候補)
- scripts/cross-check.mjs、scripts/ci-groups.mjs、docs/testing.md

## メモ

- T0026 で起票した (元の候補: T0435 の script-check、T0017 の cargo doc の警告、T0013 の windows-gnu の clippy)。
- やったこと:
  - script-check: `.github/workflows/ci.yml` の `rust-checks` (ワークスペースをビルドしない、Node と pnpm のあるジョブ) に `just script-check` を足した。
    Node だけで動くので、ビルドするジョブに入れる理由が無い。docs/testing.md#ci-のジョブ に理由を書いた。
  - cargo doc: `just doc-check` (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) を足して `just ci` に入れた。CI は
    `rust (<グループ>)` の各ジョブで `just doc-group <グループ>` (グループのクレートと feature だけ。ワークスペース全体を 1 つのジョブで
    ビルドしない方針に揃えた)。5 つのグループすべてで手元で通ることを確かめた。
  - 警告は `deformer.rs:54` のほかに 28 件あった (`--keep-going` で全部を出した)。直したのは doc コメントだけ:
    `[0,1]²` をバッククォートで囲む (ugoe-model・ugoe-editor-core の overflow.rs)、公開の文書から非公開の項目へのリンク
    (`sanitize_dt`・`ModuleContext::normalize`・`target_id`・`conductor_effects`・`wardrobe`・`route`・`Studio::sync_listener`・`Slot`・`Limits`・
    `check_derived_size`・`autorig`・`Extra`・`Tools::with`) をバッククォートだけにする、wasm32 でだけある `WebPuppet` のリンクを外す、
    モジュールと primitive の両方に読める `reference` を `mod@reference` にする、冗長なリンク先 (ugoe-motion の curve.rs) を外す、
    ugoe-mcp の裸の URL を `<…>` で囲む。
  - **範囲外の小さな変更:** 上の doc コメントの修正は ugoe-motion・ugoe・ugoe-editor-core・ugoe-wasm・ugoe-mcp・ugoe-conformance・
    ugoe-studio に及ぶ (コードの振る舞いは変えていない)。doc コメントは ts-rs と schemars の生成物に入るので、`just gen` で
    `apps/editor/src/lib/generated/types.ts` と `crates/ugoe-editor-core/schema/command.schema.json` を作り直した。
  - windows-gnu: `scripts/cross-check.mjs` の `CHECKS` に `tools` (`ugoe-ai`・`ugoe-mcp`、`x86_64-pc-windows-gnu`、feature は
    `gpu-tests` の 2 つで `--all-features` に揃えた、要るのは ring の C のための MinGW の gcc) を足した。足せない依存は無かった。
    `just cross-check tools` が手元で通った。`scripts/cross-check.test.mjs` に定義のテストを足した。
- 確かめた範囲: GitHub Actions には push できないので、ci.yml は `actionlint` (1.7.12) で検査した。手元の shellcheck との連携が
  壊れていて (`shellcheck 0.9.0` の出力を JSON と読めない) `-shellcheck=` で shellcheck を外して流した (変更前の ci.yml も同じ)。
  追加したステップと同じコマンド (`just script-check`・`just doc-group <5 つのグループ>`・`node scripts/cross-check.mjs run tools`・
  `node scripts/cross-check.mjs apt`/`targets`) は手元で通った。
- やらなかったこと:
  - `cargo doc --workspace` で、`ugoe-cli` の bin `ugoe` と `ugoe` クレートの lib のドキュメントの出力先が衝突する cargo の警告
    (`output filename collision at target/doc/ugoe/index.html`) は rustdoc の警告ではなく `-D warnings` で止まらないので、そのまま。
    `just doc` で開く `ugoe` のページが CLI のものに上書きされることがある。CI の `doc-group` はグループに分かれていて衝突しない。
  - wasm32 のターゲットでの cargo doc (`WebPuppet` などの wasm32 だけのコード) は入れていない。
  - `CHECKS.tools` の feature は手で書いたので、ugoe-ai・ugoe-mcp に feature が増えたら足す必要がある。
- 起票の候補:
  - `ugoe-cli` の bin の名前 `ugoe` と `ugoe` クレートの doc の出力先の衝突を解く (bin に `doc = false` を付けるなど)。
  - `cross-check.mjs` の `features` に「すべての feature」(`--all-features`) を書けるようにする (feature の書き漏れを無くす)。
  - wasm32 のターゲットでの `cargo doc -p ugoe-wasm` の警告の検査。
