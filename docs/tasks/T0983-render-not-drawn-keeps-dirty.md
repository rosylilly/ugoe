---
id: T0983
title: "描かずに返った render (サーフェスの作り直し・時間切れ) の後も、描き直しの印を残して次のフレームで描く (Player・<ugoe-avatar>・エディタ)"
status: done
milestone: M8
crates: [ugoe-wasm, packages/player, bindings/web, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0983
---

# T0983: 描かずに返った render の後も、描き直しの印を残して次のフレームで描く

## 目的

T0857・T1402・T0982 で、Player (スタジオの受け手・ビューア)・`<ugoe-avatar>`・エディタのプレビューは、変化があったときだけ `render()` を呼ぶようにした。
しかし wasm の `render` は、サーフェスを作り直したときや時間切れのときに描かずに返ることがある。そのフレームで描き直しの印 (`#dirty` など) を下ろしてしまうと、
形も見せ方も変わらない限り次のフレームでも描かないので、古い画 (や空の画) が残る (T0982 の候補。バグ寄り)。

## 受け入れ条件

- [x] 原因の経路 (wasm の `render` が描かずに返る場合の一覧) を確かめ、メモに書いた
- [x] `render` が描けたかを呼び出し側が知れる形にした (例: 描けたかを返す。wasm の口を変えるなら T0492 の方針に従い、型の写し (`just wasm-types`) とそろえる)。描けなかったときは描き直しの印を残し、次のフレームで描く
- [x] Player・`<ugoe-avatar>`・エディタのビューポートのすべてで直した
- [x] 部品のテスト (偽の wasm で、描かずに返った後の次のフレームで描く) を足した。直す前に落ちることを確かめた
- [x] `just player-browser`・`just editor-browser`・`just conformance` (avatar)・`just e2e --only editor,viewer,studio`・`just ci` が通る

## 参考

- T0857・T1402・T0982 のメモ、crates/ugoe-wasm (render)、packages/player/src/player.svelte.ts・surface.ts、bindings/web/src/avatar.ts、apps/editor の ViewportController

## メモ

- T0982 のワーカーの報告から起票した (2026-10-08)。
- 原因の経路 (`crates/ugoe-wasm/src/web.rs` の `WebPuppet::render`): `get_current_texture` が
  `Outdated`・`Lost` (サーフェスを `configure` し直して返る)・`Timeout`・`Occluded` を返したとき、何も描かず `Ok` で返っていた。
  `shape_changed` は下ろしていなかったので形の変化は次のフレームで描けたが、呼び出し側の見せ方の印 (Player の `#dirty`・
  `<ugoe-avatar>` の `#needsRender`・エディタの `requestRender` の 1 回きりの描画) は下りて、古い画が残っていた。
  GPU の喪失 (`device_lost`)・`Validation`・描画の失敗は従来どおり例外 (失敗してもクリアした画は出す)。
- やったこと: `render` を `Result<bool, JsValue>` (描けたか) にし、型の写し (`bindings/web/stub/ugoe_wasm.d.ts`・
  `packages/player/src/wasm.ts`) を `render(): boolean` にした (`just wasm` → `just wasm-types` が通る)。
  - Player: `#tick` で描けなかったら `#dirty` を立て直す (ふつうのモード・受け手のモードとも)。受け手の報告の `drawn` は実際に描いた回だけ。
  - `<ugoe-avatar>`: 描けなかったら `#needsRender` を立て直す (止めている間も次のフレームを頼む)。
  - エディタの ViewportController: `#render` で描けなかったら `requestRender` を頼み直す。プレビューの形の変化は `shape_changed` が残る。
  - 偽の wasm に「次の n 回は描かずに `false` を返す」口を足し、playerDraw・playerFollower・avatar・controller の各テストを足した。
    直す前に 4 つとも落ちることを確かめた。
  - 文書: crates/ugoe-wasm/README.md・bindings/web/README.md・packages/player/README.md・docs/editor.md。
  - 範囲の外の小さな変更: `tests/conformance/web/avatar.js` の `render` の包みが結果を返し、描けなかった回は写さないようにした
    (`undefined` を返すと要素が描けなかったとみなして描き直すため)。
- 確かめたこと: `just player-browser` (25/25)・`just editor-browser` (4/4)・`just conformance` (avatar を含めすべて ok)・
  `just e2e --only editor,viewer,studio`・`just wasm-check`・`just ci`。実際のブラウザで `Outdated`・`Timeout` を起こすことは
  できないので、描かずに返る経路は偽の wasm の部品のテストだけで確かめた。
- やらなかったこと: 描けなかった試みの CPU の時間は受け手の負荷の報告 (`renderMs`) に足していない (描けた回の時間だけ)。
