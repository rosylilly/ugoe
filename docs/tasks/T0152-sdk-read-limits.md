---
id: T0152
title: SDK とバインディングから読み込みの上限を指定できるようにする (C ABI・wasm・@ugoe/runtime)
status: in-progress
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/web, docs]
depends_on: [T0150]
assignee: claude-worker-T0152
---

# T0152: SDK とバインディングから読み込みの上限を指定できるようにする (C ABI・wasm・@ugoe/runtime)

## 目的

T0150 で `ugoe-format` に入れる読み込みの上限 (`ReadLimits`) を、組み込む側が指定できるようにする。
上限は読む側の方針なので (T0150 の「方針」)、ゲームエンジン・ブラウザ・配信アプリなど、置かれた環境ごとに利用者が決められる必要がある。
指定しなければ既定値で読み、超えたら読み込みはエラーになる。

## 受け入れ条件

- [ ] SDK (`ugoe` クレート): モデルとモジュールの読み込みで上限を指定できる (既定値で読む今の関数はそのまま残す)
- [ ] C ABI: 上限を渡して読み込む関数を足し、ヘッダと README に書いた。上限を超えたときのエラーコードがある
- [ ] wasm と `@ugoe/runtime`: 読み込みの関数のオプションで上限を指定できる。超えたときのエラーにコードがある (T0434 の読み込みのエラーのコードと揃える)
- [ ] 各入口で、小さな上限を指定して超えるファイルを読むとエラーになるテストがある
- [ ] Unity と Godot に同じ口を出すかを決め、出さないなら後続のタスクにした
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0150 (ugoe-format の `ReadLimits` と、仕様の書き直し)
- T0434 (wasm の読み込みのエラーのコード)
- docs/portability.md

## メモ
