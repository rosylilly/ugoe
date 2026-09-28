---
id: T0308
title: 当たり判定 (HitArea) の API
status: in-progress
milestone: M3
crates: [ugoe, ugoe-runtime, ugoe-wasm]
depends_on: []
assignee: claude-worker-T0308
---

# T0308: 当たり判定 (HitArea) の API

## 目的

`Model::hit_areas` (メッシュの変形後の形で判定する) を、アプリがキャンバス座標で問い合わせられるようにする。
アバタープロファイルの標準の名前 (`Head`・`Body`) の対応 (`Model.avatar.hit_areas`、T0110) を引けるようにもする。

## 受け入れ条件

- [ ] `Puppet` (と必要なら `ModelInstance`) に、点を含む当たり判定の ID を返す API がある (手前から順。非表示・不透明度 0 の扱いを決めて書く)
- [ ] 変形後の形で判定するテスト (パラメータを動かすと当たる場所が変わる)
- [ ] wasm (`WebPuppet`) からも呼べる
- [ ] `docs/spec/evaluation.md` か `docs/spec/model.md` に判定の規則を書く
- [ ] `just ci`・`just wasm-check` が通る

## 参考

- `crates/ugoe-wasm/src/hit.rs` (エディタの選択用の、要素のヒットテスト)

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
