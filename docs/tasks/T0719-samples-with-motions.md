---
id: T0719
title: 見本: 見本のモデルにモーションと表情を入れる・npm パッケージの serde_json の見直し・平滑化の係数
status: in-progress
milestone: M4
crates: [ugoe-cli, assets, bindings/web, ugoe-wasm, docs]
depends_on: [T0413]
assignee: claude-worker-T0719
---

# T0719: 見本のモーションと npm パッケージの残り

## 目的

T0413 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 見本のモデル (assets/samples) にモーションと表情を入れる (ugoe sample の生成を直し、コミットした見本を作り直す。ビューアのサンプルで再生できる)
- [ ] npm パッケージの wasm の serde_json (約 0.37MB) を見直す
- [ ] トラッキングの平滑化の既定の係数を、実際の顔で調整する (できなければ利用者の確認に残す)
- [ ] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0413-web-runtime-size-and-tracking.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
