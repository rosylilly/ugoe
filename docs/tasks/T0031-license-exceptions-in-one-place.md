---
id: T0031
title: npm のライセンスの例外 (NPM_EXCEPTIONS と LICENSE_OVERRIDES) を 1 か所にまとめる
status: in-progress
milestone: M9
crates: [scripts]
depends_on: []
assignee: claude-worker-T0031
---

# T0031: npm のライセンスの例外 (NPM_EXCEPTIONS と LICENSE_OVERRIDES) を 1 か所にまとめる

## 目的

`scripts/check-licenses.mjs` の `NPM_EXCEPTIONS` (確かめた版と理由) と、T0926 で足した `scripts/third-party-notices.mjs` の `LICENSE_OVERRIDES` が、
同じパッケージ (`svelte-toolbelt` など) のライセンスの扱いを別々に持つ。片方だけ直る食い違いを避けるため、`scripts/lib` の 1 か所にまとめる (T0926 の候補)。

## 受け入れ条件

- [ ] 例外の一覧が `scripts/lib` の 1 つのモジュールにあり、2 つのスクリプトがそれを読む (版の確かめは今のまま)
- [ ] 一覧のテストが `just script-check` にある
- [ ] `just licenses` と `just ci` が通る

## 参考

- T0926 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0926)。
