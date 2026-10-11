---
id: T0091
title: "CI を見直す: push ごとは軽い検査だけにし、重いビルド (Windows・macOS・GPU・適合テスト・e2e・Unity・Godot など) はリリースの前と定期・手動に回す"
status: todo
milestone: Release
crates: [.github/workflows, scripts, justfile, docs]
depends_on: []
assignee:
---

# T0091: CI を、push ごとは軽く、重いものはリリースの前に

## 目的

非公開のリポジトリの GitHub Actions の無料枠 (月 3000 分) を、2026-10-11 に使い切った。1 回の push で CI・E2E・Unity が動き、Windows (×2)・macOS (×10) の倍率を含めて
約 200 分を使う (オーケストレーターの測定。main に月 60 回近く動いていた)。利用者の方針 (2026-10-11): **公開・非公開にかかわらず、重いビルドを毎 push 行わない。
リリースのパッケージのビルドの前に検査すれば足りる。** 先々はオープンソースとして公開する (公開すれば標準のランナーは無料になるが、方針は同じ)。

## 受け入れ条件

- [ ] 今の各ワークフロー・ジョブの所要時間と起動の条件を一覧にした (`gh api .../actions/runs/<id>/jobs` で測る。倍率を掛けた分も)
- [ ] **push・PR ごと (軽い):** Linux の 1 つのジョブ (か少数) で、fmt・clippy・テスト・依存方向・refs・ライセンス・フロントエンドの検査など、手元の `just ci` に相当するものだけにする。
      変わったパスで絞れるもの (docs だけの変更では Rust を動かさない、など) は `paths` で絞る。目安と、どれくらい減るかの見積もりをメモに書く
- [ ] **重いもの:** Windows・macOS・cross・GPU (vulkan・gl)・conformance・e2e・Unity・Godot・perf・fuzz・nostd-build-std・アプリのビルドなどは、push では動かさず、
      (a) 手での起動 (workflow_dispatch)、(b) 定期 (週 1 回などの頻度を決める。無料枠に収まるか見積もる)、(c) **リリースの前**、で動かす
- [ ] **リリースの前の検査:** `v*` のタグで動くリリースのワークフロー (editor-build・crates-release・web-runtime-release・ffi-release・release-notes) が、パッケージを作る前に
      重い検査一式を通すようにする (再利用できるワークフロー `workflow_call` にまとめ、リリースの各ワークフローが `needs` で待つ、など)。検査が落ちたらパッケージを作らない
- [ ] 同じコミットに同じ検査が重ならないよう `concurrency` と起動の条件を整理した。キャッシュ (rust-cache など) がジョブの分割で効かなくならないことを確かめた
- [ ] docs/testing.md#ci-のジョブ・docs/release.md・AGENTS.md (CI の説明があれば)・scripts/ci-groups.mjs の説明を、新しい分け方に合わせた
- [ ] actionlint が通る。`just ci` が通る
- [ ] (オーケストレーター) 課金が戻った後か公開の後に、push の CI と、手での起動の重い検査を 1 度ずつ流して確かめる

## 参考

- .github/workflows/*.yml、scripts/ci-groups.mjs、docs/testing.md#ci-のジョブ、docs/release.md、T0084・T0086・T1408 (リリースのワークフロー)

## メモ

- 利用者の指示 (2026-10-11) でオーケストレーターが起票した。
- 自前のランナー (self-hosted) は、このタスクでは入れない (公開すると、フォークからの PR で自前の機械が使われる危険がある)。要るなら後続の候補に。
