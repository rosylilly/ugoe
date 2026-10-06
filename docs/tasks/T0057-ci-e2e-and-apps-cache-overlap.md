---
id: T0057
title: "CI: e2e と rust (apps) のキャッシュの重なり (Tauri の依存) を調べ、1 組を 5GB に収めるか決める"
status: in-progress
milestone: M10
crates: [.github/workflows, docs]
depends_on: [T0056]
assignee: claude-worker-T0057
---

# T0057: CI: e2e と rust (apps) のキャッシュの重なり (Tauri の依存) を調べ、1 組を 5GB に収めるか決める

## 目的

T0051 の見積もりで、変えた後のキャッシュの 1 組は約 5.3GB で、目標の 5GB を少し超える。大きいのは e2e (約 1.1GB) と rust (apps) (約 0.94GB) で、
どちらも Tauri の依存を持つ。e2e は release と dev の両方をビルドするので、そのまま共有はできない。重なりを調べ、減らせるなら減らす。

## 受け入れ条件

- [ ] e2e と rust (apps) が作る依存の成果物 (プロファイル・feature・環境のハッシュ) の違いを調べ、共有できる部分と大きさの見込みをメモに書いた
- [ ] 減らす手 (キーの共有・e2e の dev のビルドをやめる・保存する target の絞り込みなど) を選んで行うか、行わない理由を書いた
      (行うなら ci.yml・e2e.yml を直し、docs/testing.md の「CI のキャッシュ」を直した)
- [ ] actionlint で増えた指摘が無い。`just ci` が通る
- 1 組の実際の合計は push の後に `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list` で測る。測れなければ「CI で確かめること」としてメモに書く

## 参考

- T0051 のメモ (見積もり・CI で確かめること・起票の候補)、docs/testing.md の「CI のキャッシュ」
- .github/workflows/ci.yml (`rust-group` の apps)・e2e.yml

## メモ

- T0052 で起票した (元の候補: T0051 (e2e と rust (apps) の重なり))。
- T0051 の「CI で確かめること」(rust (windows)・rust (macos) の完全一致・gpu と web が依存をコンパイルしない・1 組の合計) は起票していない
  (push の後に確かめる)。その結果が出ていれば、ここで使う。
- 同じワークフローを触る T0056 の後にした。
