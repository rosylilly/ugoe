---
id: T0434
title: wasm の読み込みのエラーにコードを付ける
status: todo
milestone: M9
crates: [ugoe-wasm, packages, bindings/web, docs]
depends_on: [T0424]
assignee:
---

# T0434: wasm の読み込みのエラーにコードを付ける

## 目的

`WebPuppet::load_json` / `load_bundle` の失敗は `JsError` (英語の文だけ) で、`code` が無い。ビューア (`packages/player/src/player.svelte.ts` の `load`) は
その英文をそのまま利用者に見せている (ADR 0029 の「利用者に見せる文言はコードから訳す」に反する)。着せ替えの `DressUpError` (`name` と `code`) と
同じ形で、読み込みの失敗にも機械が読むコードを付け、ビューアがコードから訳せるようにする。T0424 (ugoe-wasm のレビュー) で見つけた。

## 受け入れ条件

- [ ] `load_json` / `load_bundle` の失敗が、`name` と `code` を持つ `Error` になる (例: `invalid_file`・`unsupported_version`・`compile`・`texture`・`texture_too_large`・`device_lost`。C ABI の `ugoe_last_error_kind` と揃えられるものは揃える)
- [ ] コードの一覧を `crates/ugoe-wasm/README.md` に書き、ネイティブのテストで `LoadError` → コードの対応を確かめる
- [ ] ビューア (`@ugoe/player`) が読み込みの失敗をコードから訳して出す (知らないコードは英文のまま)
- [ ] `bindings/web` の型の写し (`stub/ugoe_wasm.d.ts`) と README を直す (変わる場合)
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-wasm/src/web.rs` (`load_json`・`load_bundle`・`load`)、`crates/ugoe-wasm/src/web/dress_up.rs` (`dress_up_error`)
- docs/adr/0029-shared-ui-and-i18n.md

## メモ
