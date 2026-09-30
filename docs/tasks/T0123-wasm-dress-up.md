---
id: T0123
title: 着せ替え: wasm から付け外しできるようにする
status: in-progress
milestone: M4
crates: [ugoe-wasm, ugoe, docs]
depends_on: [T0121, T0113]
assignee: claude-worker-T0123
---

# T0123: 着せ替え: wasm から付け外しできるようにする

## 目的

ADR 0024。ビューアとエディタのプレビュー (どちらも wasm で描く) で着せ替えを試せるように、wasm に `.ugoemod` の読み込みと付け外しの口を足す (T0121 の後続の候補)。

## 受け入れ条件

- [ ] `.ugoemod` のバイト列を読み込み、`Puppet` の `attach_module` / `detach_module` / `set_modules` を呼べる。`DressUpChange` に従ってテクスチャ (と法線マップ) をレンダラに登録・差し替える
- [ ] 付けているモジュールの一覧と、モデルのスロットの一覧を返す
- [ ] wasm のテスト (または wasm-bindgen を通さない Rust のテスト) と `just wasm-check`。ビューアとエディタの stub.d.ts を合わせる
- [ ] `just ci` が通る

## 参考

- `docs/adr/0024-module-authoring.md`、`docs/spec/modules.md#puppet-での着せ替え`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
