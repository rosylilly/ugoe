---
id: T0031
title: npm のライセンスの例外 (NPM_EXCEPTIONS と LICENSE_OVERRIDES) を 1 か所にまとめる
status: done
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

- [x] 例外の一覧が `scripts/lib` の 1 つのモジュールにあり、2 つのスクリプトがそれを読む (版の確かめは今のまま)
- [x] 一覧のテストが `just script-check` にある
- [x] `just licenses` と `just ci` が通る

## 参考

- T0926 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0926)。
- やったこと:
  - 例外の一覧を `scripts/lib/npm-license-exceptions.mjs` の `NPM_LICENSE_EXCEPTIONS` (名前 -> 確かめた版・ライセンス・確かめ方。凍結したオブジェクト) に移した。
    `check-licenses.mjs` の `NPM_EXCEPTIONS` と `third-party-notices.mjs` の `LICENSE_OVERRIDES` を消し、2 つともこれを読む。
  - 版の確かめは今のまま `check-licenses.mjs` が行う (版が違えば失敗する。版を書き換えて失敗することを手で確かめた)。表記の側は版を見ずにライセンスだけ使う (従来どおり)。
  - `scripts/npm-license-exceptions.test.mjs` (一覧の形・版が 1 つの版であること・SPDX の式の形・凍結・`licenseOf` が一覧を使うこと) を `just script-check` に足した。
  - `crates:` の外の小さな変更: `justfile` の `script-check` にテストを足し、`packages/ui/README.md` の更新手順の書き先を新しいファイルに直した。
- `just licenses`・`just script-check`・`just ci` が通った。
- 起票の候補: なし。
