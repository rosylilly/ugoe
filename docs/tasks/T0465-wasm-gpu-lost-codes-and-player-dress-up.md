---
id: T0465
title: wasm の描画・読み戻しの GPU の喪失にコードを付けるか決め、@ugoe/player の着せ替えの失敗の訳に device_lost を足す (判断が要る)
status: in-progress
milestone: M9
crates: [ugoe-wasm, bindings/web, packages/player, docs]
depends_on: [T0934]
assignee: claude-worker-T0465
---

# T0465: wasm の描画・読み戻しの GPU の喪失にコードを付けるか決め、@ugoe/player の着せ替えの失敗の訳に device_lost を足す (判断が要る)

## 目的

T0457 で wasm の着せ替えの GPU の喪失に `device_lost` のコードを付けた。次が残っている (T0457 の候補)。

1. `@ugoe/player` の着せ替えの失敗の訳 (`packages/player/src/dressUp.ts`) に `device_lost` が無い (読み込みの `loadError.deviceLost` と同じ文でよい)。
2. **判断が要る:** wasm の `render`・`read_pixels` など、ほかの GPU の喪失の例外にもコード (と JS のエラーの名前) を付けるか。付けると `@ugoe/runtime` の公開の型が増える。

訳の表を触る T0934 の後に着手する。

## 受け入れ条件

- [ ] `@ugoe/player` の着せ替えの失敗の訳に `device_lost` があり、テストがある
- [ ] ほかの GPU の喪失の例外にコードを付けるかの案と推奨をメモに書き、利用者の判断を得た。付けるなら crates/ugoe-wasm/README.md・`bindings/web` の型・`codes.json` を揃え、テストがある
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0457 のメモ (起票の候補)
- crates/ugoe-wasm/README.md、packages/player/src/loadError.ts

## メモ

- T0032 で起票した (元の候補: T0457 (player・wasm の GPU の喪失))。
- 判断 (2026-10-05、利用者の判断): wasm の `render`・`read_pixels` などの GPU の喪失の例外にもコードと JS のエラーの名前を付ける (`@ugoe/runtime` の型も足す)。
