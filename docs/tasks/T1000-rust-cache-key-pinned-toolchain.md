---
id: T1000
title: "CI: ランナーのイメージの stable の rustc が上がっても、rust-cache のキャッシュがすべて外れないようにする"
status: in-progress
milestone: Release
crates: [.github/workflows, rust-toolchain.toml, docs]
depends_on: []
assignee: claude-worker-T1000
---

# T1000: rust-cache の鍵とランナーの stable

## 目的

`Swatinem/rust-cache` は環境のハッシュに `rustc` の版を入れる。ランナーのイメージが stable の rustc を上げると (10-11 に 1.99.0)、
`rust-toolchain.toml` で固定した版を使っていてもすべてのキャッシュが外れる (T0091 の run 38098330681 で rust (apps) が約 6 分 → 約 26 分)。
固定した版だけが鍵に入るようにし、イメージの更新でキャッシュが外れないようにする。

## 受け入れ条件

- [ ] 原因を確かめた: rust-cache が鍵に入れる版が、ランナーの既定の `rustc` (イメージの stable) か、`rust-toolchain.toml` の版か
      (rust-cache の action.yml と実装を読み、run のログの鍵で確かめる)
- [ ] 直した。案: (1) rust-cache の前に `rustup show` (か `rustup toolchain install`) で `rust-toolchain.toml` の版を入れ、作業ディレクトリでの
      `rustc` がその版になる順にする、(2) `add-rust-environment-hash-key: false` にし、`rust-toolchain.toml` の版を `prefix-key` か `key` に自分で入れる。
      どちらにしたかと理由をメモに書く
- [ ] `ci.yml`・`release-checks.yml` ほか rust-cache を使うすべてのワークフローで同じ形にした (`shared-key` を共有するジョブの鍵が食い違わない)
- [ ] actionlint が通る。実際の run で、イメージの stable と固定の版が違うときもキャッシュが当たることを確かめた (Actions が使えなければ、
      確かめていないこととしてメモに書き、ログで鍵の文字列を確かめる手順を書く)
- [ ] docs/testing.md#ci-のジョブ のキャッシュの説明を直した
- [ ] `just ci` が通る

## 参考

- T0091 のメモ (測定・キャッシュの鍵・起票の候補)、docs/testing.md#ci-のジョブ
- Swatinem/rust-cache の README (`add-rust-environment-hash-key`・`prefix-key`)

## メモ

- T0095 で起票した (元の候補: T0091 (ランナーのイメージの stable の rustc が上がると rust-cache の鍵が変わってキャッシュがすべて外れる))。
- Release のマイルストーン。T0091 と同じ。
- 番号: 基盤の帯 T00xx を使い切ったので、2 つ目の範囲 T10xx の最初。
