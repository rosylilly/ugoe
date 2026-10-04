---
id: T0320
title: 物理の状態が有限でなくなったときの扱いを決める
status: todo
milestone: M9
crates: [ugoe-physics, docs]
depends_on: [T0316, T0318]
assignee:
---

# T0320: 物理の状態が有限でなくなったときの扱いを決める

## 目的

検査 (T0318) を通る有限の設定でも、極端な値 (巨大な `weight` や `radius`) では粒子の位置があふれて無限大・NaN になりうる。
いまは一度 NaN になった粒子は速度が NaN のまま戻らず、`reset` まで鎖が止まる。パラメータへの出力は `ParameterValues` が NaN を
無視するので害がないが、動的物理の加算量 (角度・原点・頂点) には NaN がそのまま足され、描画の頂点が NaN になる (T0316 で確認)。
ランタイムのどこで止めるかを決めて仕様に書く。

## 受け入れ条件

- [ ] 次のどれにするかを決めて `docs/spec/physics.md` と英語の公開仕様に書く: ステップの後に有限でない粒子があればそのリグを静止姿勢に置き直す /
      有限でない出力値を捨てる (前の値を使う・0 にする) / 何もしない (検査に任せる)
- [ ] 決めた扱いを実装し、NaN を作る設定 (`gravity` が NaN・`radius` が無限大など) でテストする
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- `docs/tasks/T0316-review-physics.md` のメモ
- `docs/spec/physics.md#時間の進め方`、`spec/physics.md`

## メモ
