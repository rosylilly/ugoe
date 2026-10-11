---
id: T0197
title: "#[non_exhaustive] を、誤り・警告の enum、設定の struct、モデルのデータの enum に付ける"
status: todo
milestone: M9
crates: [ugoe-core, ugoe-model, ugoe-runtime, ugoe-format, ugoe-physics, ugoe-motion, ugoe-render, ugoe, ugoe-ffi, ugoe-wasm, docs, spec]
depends_on: [T0196]
assignee:
---

# T0197: #[non_exhaustive] を付ける

## 目的

`#[non_exhaustive]` が format と render の一部にしか付いておらず、揃っていない (T0195 の所見)。後から種類やフィールドを足しやすくするため、付ける。

## 決まっていること (利用者、2026-10-11)

- 誤り・警告の enum、設定の struct、モデルのデータの enum (デフォーマの種類・ブレンドなど) に付ける。
- 「安定に神経質にならない」方針 (AGENTS.md) は「保証したいくらいの意味でも付けてよい」という意味合い。厳密な semver の約束のための作業はしない。付けて害の無い所に付ける。

## 受け入れ条件

- [ ] 公開の enum と struct を洗い出し、付けるもの・付けないもの (付けると使う側が不便になるだけのもの。例: 中身を全部作って渡す小さな値の struct) を決めて、理由をメモに書いた
- [ ] 付けた。クレートの外 (ワークスペースの別のクレート・bindings/godot・fuzz・tests/conformance) の網羅の match や構造体の式が壊れたら、`_ =>` や作る関数で直した
- [ ] モデルのデータの enum に付けたことを docs/spec/model.md に一言書いた (英語の spec/ は Rust の型の話を持たないなら不要。持つなら同じく)
- [ ] docs/release.md「公開の範囲」の #[non_exhaustive] の記述を、付けた形に合わせた
- [ ] `just ci`・`just wasm-check`・`just nostd-check`・`just conformance`・`just godot-check` が通る

## 参考

- T0195 のメモ (公開 API の洗い出し)、docs/release.md「公開の範囲」、T0196 (内部の整理。同じクレートを触るので後にする)

## メモ

- 利用者の決定 (2026-10-11) を受けてオーケストレーターが起票した。
