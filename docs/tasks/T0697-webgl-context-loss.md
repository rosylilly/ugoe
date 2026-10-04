---
id: T0697
title: WebGL2 のコンテキストの喪失を WebPuppet.is_lost で知らせる
status: done
milestone: M9
crates: [ugoe-wasm, packages/player, bindings/web, apps/editor, docs]
depends_on: [T0672]
assignee:
---

# T0697: WebGL2 のコンテキストの喪失を WebPuppet.is_lost で知らせる

## 目的

`WebPuppet.is_lost()` は wgpu のデバイスの喪失 (`set_device_lost_callback`) だけを見ている。wgpu の GL のバックエンド
(wgpu-hal 30) は WebGL2 のコンテキストの喪失 (`webglcontextlost`。GPU のリセット・ドライバの更新・タブを裏に回したときの
回収など) を知らせず、描画も失敗しない (何も描かれない) ので、`is_lost` で作り直すプレーヤー
(`packages/player/src/player.svelte.ts`) と `<ugoe-avatar>` (`bindings/web/src/avatar.ts`) は、WebGL2 で表示していると
真っ白のまま戻らない。

エディタは T0672 で canvas の `webglcontextlost` を聞いて作り直すようにした (`apps/editor/src/lib/viewport/controller.svelte.ts`
の `#onContextLost`)。これを wasm の側に寄せ、どのホストも `is_lost` だけを見ればよいようにする。

## 受け入れ条件

- [ ] `WebPuppet::create` が WebGL2 で作ったとき、canvas の `webglcontextlost` で `lost` を立てる (リスナーは `free` で外す)。`is_lost` の doc コメントと crates/ugoe-wasm/README.md に書く
- [ ] プレーヤーと `<ugoe-avatar>` が WebGL2 のコンテキストの喪失から作り直せることを確かめる (テストか、手で確かめた手順をメモに)
- [ ] エディタの `#onContextLost` を外すか、wasm の知らせと重ならないことを確かめる
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0672 のメモ (所見の表)
- crates/ugoe-wasm/src/web.rs の `lost`・`is_lost`

## メモ

- **閉じた (T0008):** T0932 にまとめた。WebGL2 のコンテキストの喪失の扱いは、WebPuppet の作成・作り直し・破棄の共有と同じコード (エディタの controller と Player) を触るので、T0932 で一緒に決める。T0932 の受け入れ条件と crates: に、wasm の `is_lost` で知らせることと `<ugoe-avatar>` を足した。
