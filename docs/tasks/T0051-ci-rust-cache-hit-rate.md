---
id: T0051
title: "CI: rust-cache が失敗したジョブで保存されないことと、容量の上限を超えて消されることを直す"
status: todo
milestone: M10
crates: [.github/workflows, docs]
depends_on: []
assignee:
---

# T0051: CI の rust-cache の当たりを良くする

## 目的

GitHub Actions の CI で、依存のクレートのキャッシュが効かず、毎回ゼロからコンパイルするジョブがある。
run 37468408049 (2026-10-06、main) のジョブごとのコンパイル数は次のとおり。

| ジョブ | キャッシュ | コンパイル数 (うち依存) |
| --- | --- | --- |
| rust (runtime/tools/apps/assets)・gpu・conformance | 完全一致 | 13〜26 (0) |
| rust (windows) | No cache found | 203 (186) |
| rust (macos) | No cache found | 373 (350) |

Linux のジョブの再コンパイルはワークスペースのメンバー (`ugoe-*`) だけで、rust-cache の想定どおり。
問題は次の 2 つ。

1. **失敗したジョブではキャッシュが保存されない。** `Swatinem/rust-cache` は既定で成功したジョブでだけ保存する。
   `rust (windows)` は 2026-10-06 の main で 6 回続けてテストで落ち (37429758787〜37464594287)、`rust (macos)` も 37464594287 で落ちたので、
   キャッシュが一度も作られず、毎回 186〜350 個の依存をビルドしていた (Windows は 1 回 16 分ほど)。
   依存のビルドはテストの成否と関係ないので、テストが落ちても依存のキャッシュは残したい。
2. **キャッシュの合計が上限を超え、古いものから消される。** リポジトリのキャッシュは 10.8GB (27 個) で、既定の上限 10GB を超えている。
   rust-cache のキャッシュは約 20 個 (unity だけで 8 個) あり、1 組で約 8GB になる。キーの末尾のハッシュは
   すべての `Cargo.toml` と `Cargo.lock` から作られ、`Cargo.toml` は 1 日に何度も変わるので、変わるたびに新しい組が保存され、
   2 組で上限を超える。Linux のジョブは前のキーから部分的に戻せるが、前の組が消えた Windows・macOS のジョブはゼロからになる。
   **上限は引き上げない** (利用者の判断)。キャッシュの数と大きさを減らして、1 組が上限に十分収まるようにする。

## 受け入れ条件

- [ ] `rust-os` (Windows・macOS) のジョブは、テストや clippy が落ちても依存のキャッシュを保存する (`cache-on-failure: true` など)。
      ほかのジョブにも付けるかを判断し、付けないならその理由をメモに書く
- [ ] PR でキャッシュを保存しているジョブ (`gpu`・`conformance`・`godot`・`web`、unity.yml の `check`・`native` など、`save-if` の無いもの) は、
      main でだけ保存する (`save-if: ${{ github.ref == 'refs/heads/main' }}`)。PR のキャッシュは main から見えず、容量を食うだけなので
- [ ] キャッシュの数を減らす。候補: unity.yml の `native` の matrix (7 個)・`check` の分け方、`gpu`・`conformance`・`godot` のように
      同じ OS で依存の重なるジョブのキーの共有、editor-build.yml・release 系のワークフローのキー。
      共有するとジョブのディスクやキャッシュ 1 個の大きさが増えるので、計測して判断する (ランナーのディスクは約 14GB。T0002)
- [ ] キャッシュ 1 個の大きさを減らせるか調べる (例: unity の release ビルドの成果物、`cache-targets`・`cache-directories` の範囲)
- [ ] 変更後に main で 1 組のキャッシュの合計を `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list` で測り、
      上限 10GB の半分 (5GB) 程度に収まるか、収まらないならどこまで減ったかをメモに書く (2 組が並んでも消されないように)
- [ ] actionlint で検査する
- [ ] docs/testing.md の CI のキャッシュの説明 (「`Swatinem/rust-cache` の key は〜」の段落) を、キャッシュの方針 (失敗時の保存・main だけで保存・キーの共有と容量の目安) に合わせて更新する
- [ ] `just ci` が通る。実際の CI のキャッシュの当たりは利用者の push の後に確かめる

## 参考

- `.github/workflows/ci.yml`・`unity.yml`・`e2e.yml`・`editor-build.yml`・`crates-release.yml`・`web-runtime-release.yml`
- [docs/testing.md#ci-のジョブ](../testing.md#ci-のジョブ)、T0002 (CI のジョブの分け方とディスクの対策)
- rust-cache の入力 (`cache-on-failure`・`save-if`・`shared-key`): https://github.com/Swatinem/rust-cache

## メモ
