---
id: T0030
title: ルートの [profile.dev] のデバッグ情報を line-tables-only にするか決める (判断が要る)
status: todo
milestone: M9
crates: [Cargo.toml, docs]
depends_on: []
assignee:
---

# T0030: ルートの [profile.dev] のデバッグ情報を line-tables-only にするか決める (判断が要る)

## 目的

T0021 で、`.worktrees/` の下の worktree だけ `debug = "line-tables-only"` にした (成果物が約半分、2 回目のビルドが速い)。
ルートの `Cargo.toml` の `[profile.dev]` は変えていない。main と worktree を使わない開発者にも同じ設定を使うかは、
デバッガで変数が見えなくなる (行とファイルは出る) ので利用者が決める (T0021 の候補)。

**判断が要る:** 全員の `[profile.dev]` を `line-tables-only` にするか。する場合は `.worktrees/.cargo/config.toml` (T0021) が要らなくなる。

## 受け入れ条件

- [ ] 利用者の判断を得た (案と推奨をメモに書き、判断を待つ間は `status: blocked`)
- [ ] 変えるなら `[profile.dev]` を変え、`scripts/worktree.mjs` が書く設定と docs/workflow/build-cache.md#worktree-のデバッグ情報 を合わせた。変えないなら理由を build-cache.md に書いた
- [ ] `just ci` が通る

## 参考

- T0021 のメモ (計測の表は docs/workflow/build-cache.md#計測-t0021)

## メモ

- T0026 で起票した (元の候補: T0021)。
