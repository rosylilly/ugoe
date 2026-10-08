---
id: T1400
title: "ugoe-wasm: WebPuppet に合成したモデルのパーツの ID の並び (part_ids) を読む口を足し、wasm の受け手は自分のモデルの並びに引き直す"
status: in-progress
milestone: M8
crates: [ugoe-wasm, bindings/web, packages/player, apps/studio, docs]
depends_on: []
assignee: claude-worker-T1400
---

# T1400: ugoe-wasm: WebPuppet の part_ids と wasm の受け手の引き直し

## 目的

スタジオのフレームのパーツの不透明度は指揮役の合成したモデルの `parts` の番号で持ち、受け手はシーンの `partIds` で ID に直して自分のモデルの
番号に引き直す (T0853)。ネイティブの受け手 (`follower.rs`) は自分のモデルの並びを読めるのでずれないが、wasm の受け手 (`@ugoe/player` の
`follower.ts`。出力ウィンドウとブラウザソース) は `WebPuppet` に合成したモデルのパーツの ID を読む口が無いため、**今のシーンの `partIds` を
自分のモデルの並びとみなす**。モジュールを読めずに飛ばしたとき (壊れたファイル・読めない場所) は並びが食い違い、そのモジュールより後の
パーツの番号がずれて、別のパーツの不透明度が変わりうる (T0853 の「制限」)。

`WebPuppet` に合成したモデルの `parts` の ID の並びを返す `part_ids()` を足し、wasm の受け手も自分のモデルの並びに引き直す。

## 受け入れ条件

- [ ] `WebPuppet::part_ids()` を足した (合成したモデルの `parts` の ID の順。着せ替えのモジュールのパーツは合成の ID。モデルが無ければ空)。
      doc コメントに `@internal` を付け、crates/ugoe-wasm/README.md の「エディタ向けの問い合わせ」の一覧に足した (節の名前と説明を、
      スタジオの受け手も使うことに合わせて直す)。`bindings/web/stub/ugoe_wasm.d.ts` の写しも足し、`just wasm-types` と
      stub の検査 (bindings/web/stub/ugoe_wasm.test.mjs) が通る
- [ ] `packages/player` の受け手 (`follower.ts` の `PartRemap`・`FrameQueue`、`Player.followScene`・`continueScene`) が、引き直し先に
      シーンの `partIds` ではなく `WebPuppet.part_ids()` (モデルを読み・付け直したときに 1 回読む) を使う。毎フレームの確保は増やさない
- [ ] テスト: モジュールを 1 つ読めずに飛ばしたシーンで、そのモジュールより後のパーツの不透明度のフレームが、指揮役と同じパーツに当たる
      (前の実装では別のパーツに当たる入力で、先に失敗するテストを書く)。偽の WebPuppet の `part_ids` で packages/player のテストに足した。
      `apps/studio/src-tauri/tests/follow_match.rs` の一致の検査が通る
- [ ] docs/studio.md#フレームの配信-内部の形 の「wasm の受け手は今のシーンの `partIds` を自分の並びとみなす」制限の記述を直した。
      packages/player/README.md の受け手のモードの説明も直した
- [ ] `just wasm-check` と `just ci` が通る

## 参考

- T0853 のメモ (制限・起票の候補)、crates/ugoe-wasm/README.md#エディタ向けの問い合わせ-ギズモ物理の表示 (`@internal` の規約。T0492)
- crates/ugoe-wasm/src/web.rs (`deformer_ids` と同じ置き方)、packages/player/src/follower.ts・player.svelte.ts、apps/studio/src/Output.svelte

## メモ

- T0077 で起票した (元の候補: T0853 (`WebPuppet` に `part_ids()` を足し、wasm の受け手も自分のモデルに引き直す。ugoe-wasm の `@internal` の口に足す))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- `@internal` (公開の API の外) に足すので、公開 API の判断は要らない。公開の API にしたくなったら別に判断する。
- `apps/studio` は `Output.svelte` がシーンの `partIds` を渡す所を合わせるときだけ触る (受け手の口の形が変わらなければ触らない)。
- T1401・T1402 も bindings/web を触るが、このタスクが触るのは stub の型の写しだけなので依存は付けていない。
