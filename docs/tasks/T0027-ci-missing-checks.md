---
id: T0027
title: CI に入っていない検査を足す (script-check・cargo doc の警告・ugoe-ai と ugoe-mcp の windows-gnu の clippy)
status: in-progress
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

- [ ] CI のいずれかのジョブで `just script-check` が流れる (どのジョブに入れたかと理由を docs/testing.md#ci-のジョブ に書いた)
- [ ] `cargo doc --workspace --no-deps` の警告を拒む検査 (`RUSTDOCFLAGS=-D warnings` など) が `just` のレシピと CI にあり、今の警告 (`deformer.rs:54` ほか) を直した
- [ ] `scripts/cross-check.mjs` の `CHECKS` に `ugoe-ai`・`ugoe-mcp` の windows-gnu の clippy を足した (足せない依存があれば理由をメモに書いた)
- [ ] `just ci` が通る

## 参考

- T0435・T0017・T0013 のメモ (起票の候補)
- scripts/cross-check.mjs、scripts/ci-groups.mjs、docs/testing.md

## メモ

- T0026 で起票した (元の候補: T0435 の script-check、T0017 の cargo doc の警告、T0013 の windows-gnu の clippy)。
