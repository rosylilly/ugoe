---
id: T0326
title: "公開仕様の小さな直し: 物理の時間を進める手順の剰余と、検証で拒まれるモーションの例"
status: in-progress
milestone: M9
crates: [spec, docs]
depends_on: []
assignee: claude-worker-T0326
---

# T0326: 公開仕様の小さな直し: 物理の時間を進める手順の剰余と、検証で拒まれるモーションの例

## 目的

- `spec/physics.md` の「Advancing time」の手順 3 が、1 フレームの刻みの上限を超えたときの剰余の扱い (T0316 で直した) を反映していない (T0320 の候補)。
  日本語の `docs/spec/physics.md` と食い違っていないかも確かめる (食い違えば英語版が正。ADR 0021)。
- T0323 でモーションと表情の数値 (NaN・無限大) を検証で拒むようにしたが、`spec/examples` に拒まれる例 (`error: validation`) が無い。

## 受け入れ条件

- [ ] `spec/physics.md` の手順 3 を実装 (`ugoe-physics`) と合わせ、`docs/spec/physics.md` との食い違いが無い
- [ ] `spec/examples` に、検証で拒まれるモーション (と表情) の例を足し、独立の読み込み器の検証 (T0120・T0126) が拒むことを確かめる
- [ ] 公開仕様の同期の検査 (`docs/spec/public-spec.md`) が通る
- [ ] `just ci` が通る

## 参考

- T0320・T0323 のメモ (起票の候補)
- spec/physics.md・spec/examples/、docs/spec/public-spec.md

## メモ

- T0019 で起票した (元の候補: T0320 の剰余、T0323 の拒まれる例)。
