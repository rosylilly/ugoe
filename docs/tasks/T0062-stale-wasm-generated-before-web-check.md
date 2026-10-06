---
id: T0062
title: "web-check (pre-push と just ci) の前に、アプリの wasm の生成物が古いかを確かめ、古ければ作り直すか分かるエラーで止める"
status: todo
milestone: M10
crates: [scripts, justfile, lefthook.yml, docs]
depends_on: []
assignee:
---

# T0062: web-check (pre-push と just ci) の前に、アプリの wasm の生成物が古いかを確かめ、古ければ作り直すか分かるエラーで止める

## 目的

エディタ・ビューア・スタジオの wasm の生成物 (`apps/editor/src/lib/viewport/generated`・`apps/viewer/src/lib/wasm/generated`・
`apps/studio/src/lib/wasm/generated`) は git の管理外で、`just wasm <out>` で作る。ugoe-wasm の API が変わった後も作業ツリーに古い生成物が残るので、
pull や merge の後の pre-push (lefthook の web-check = 各パッケージの `svelte-check`) や `just ci` の `wasm-types` が、
コードの誤りでなく生成物の古さで落ちる (2026-10-06〜07 に何度も起きた。例: T1660 の `rotation_local_keyform` が無いと svelte-check が言う)。
利用者 (人) にも原因が分かりにくい。

## 受け入れ条件

- [ ] 生成物が今の ugoe-wasm (と wasm-bindgen の版) から作られたものかを確かめる仕組みを足した。例: `just wasm` が生成物の横に入力 (crates/ugoe-wasm と依存するクレートのソース・Cargo.lock・wasm-bindgen の版) の指紋を書き、確かめる側が今の指紋と比べる。安く (ビルドせずに) 確かめられること
- [ ] lefthook の pre-push の web-check と、`just ci` の web-check (または wasm-types) の前で確かめ、古ければ作り直す。作り直しが重すぎるなら「`just wasm …` で作り直す」と分かるエラーで止める。どちらにしたかと理由をメモに書く
- [ ] 生成物が無い場合 (作ったことが無い) と、mbx のある機械・無い機械の両方で動く
- [ ] 古い生成物を置いて (指紋を書き換えるなど) 確かめる側が見つけることを、スクリプトのテストで確かめた
- [ ] docs/testing.md (と必要なら CONTRIBUTING・apps の README) に書いた
- [ ] `just ci` が通る

## 参考

- justfile の `wasm`・`wasm-types`・`web-check`、lefthook.yml の pre-push、scripts/check-wasm-types.mjs

## メモ

- 利用者の依頼で起票した (2026-10-07)。
