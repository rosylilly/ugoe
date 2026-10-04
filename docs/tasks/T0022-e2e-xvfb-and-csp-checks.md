---
id: T0022
title: "e2e: Xvfb の番号の衝突をなくし、3 つのアプリで CSP の違反が無いことを確かめる"
status: in-progress
milestone: M9
crates: [tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0022
---

# T0022: e2e: Xvfb の番号の衝突をなくし、3 つのアプリで CSP の違反が無いことを確かめる

## 目的

- **Xvfb の番号 (T0820):** `tests/e2e/run.mjs` は決まった番号で Xvfb を起動するので、同じ機械で並行に流すと
  `SocketCreateListener failed` で起動できない (M9 の並列のワーカーで起きた。T0820 は自分で `Xvfb :157` を立てて `--headed` で流した)。
  空いている番号を選ぶ (`Xvfb -displayfd` で番号を受け取るなど)。
- **CSP の違反 (T0686):** 3 つのアプリは CSP を持つようになった (エディタ T0686・ビューア T0727・スタジオ T0820) が、e2e は
  CSP が止めたもの (`securitypolicyviolation`) を集めていない。止まっても画面の一部が静かに動かなくなるだけで気づけない。
  共通の手順として、各アプリの筋書きの終わりに違反が 0 件であることを確かめる。

## 受け入れ条件

- [ ] `run.mjs` が空いている Xvfb の番号を使う。2 つの `just e2e` を同時に流して両方が起動することを確かめた
- [ ] 3 つのアプリの筋書きで `securitypolicyviolation` を集め、最後に 0 件であることを確かめる共通の補助がある。CSP を壊す (例: `script-src` から `'wasm-unsafe-eval'` を外す) と失敗することを確かめた
- [ ] tests/e2e の README (または docs/testing.md) に書いた
- [ ] `just e2e` と `just ci` が通る

## 参考

- T0820・T0686 のメモ (起票の候補)
- tests/e2e/run.mjs・tests/e2e/specs/

## メモ

- T0019 で起票した (元の候補: T0820 の Xvfb、T0686 の CSP の違反)。
