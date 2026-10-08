---
id: T1400
title: "ugoe-wasm: WebPuppet に合成したモデルのパーツの ID の並び (part_ids) を読む口を足し、wasm の受け手は自分のモデルの並びに引き直す"
status: done
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

- [x] `WebPuppet::part_ids()` を足した (合成したモデルの `parts` の ID の順。着せ替えのモジュールのパーツは合成の ID。モデルが無ければ空)。
      doc コメントに `@internal` を付け、crates/ugoe-wasm/README.md の「エディタ向けの問い合わせ」の一覧に足した (節の名前と説明を、
      スタジオの受け手も使うことに合わせて直す)。`bindings/web/stub/ugoe_wasm.d.ts` の写しも足し、`just wasm-types` と
      stub の検査 (bindings/web/stub/ugoe_wasm.test.mjs) が通る
- [x] `packages/player` の受け手 (`follower.ts` の `PartRemap`・`FrameQueue`、`Player.followScene`・`continueScene`) が、引き直し先に
      シーンの `partIds` ではなく `WebPuppet.part_ids()` (モデルを読み・付け直したときに 1 回読む) を使う。毎フレームの確保は増やさない
- [x] テスト: モジュールを 1 つ読めずに飛ばしたシーンで、そのモジュールより後のパーツの不透明度のフレームが、指揮役と同じパーツに当たる
      (前の実装では別のパーツに当たる入力で、先に失敗するテストを書く)。偽の WebPuppet の `part_ids` で packages/player のテストに足した。
      `apps/studio/src-tauri/tests/follow_match.rs` の一致の検査が通る
- [x] docs/studio.md#フレームの配信-内部の形 の「wasm の受け手は今のシーンの `partIds` を自分の並びとみなす」制限の記述を直した。
      packages/player/README.md の受け手のモードの説明も直した
- [x] `just wasm-check` と `just ci` が通る

## 参考

- T0853 のメモ (制限・起票の候補)、crates/ugoe-wasm/README.md#エディタ向けの問い合わせ-ギズモ物理の表示 (`@internal` の規約。T0492)
- crates/ugoe-wasm/src/web.rs (`deformer_ids` と同じ置き方)、packages/player/src/follower.ts・player.svelte.ts、apps/studio/src/Output.svelte

## メモ

- T0077 で起票した (元の候補: T0853 (`WebPuppet` に `part_ids()` を足し、wasm の受け手も自分のモデルに引き直す。ugoe-wasm の `@internal` の口に足す))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- `@internal` (公開の API の外) に足すので、公開 API の判断は要らない。公開の API にしたくなったら別に判断する。
- `apps/studio` は `Output.svelte` がシーンの `partIds` を渡す所を合わせるときだけ触る (受け手の口の形が変わらなければ触らない)。
- T1401・T1402 も bindings/web を触るが、このタスクが触るのは stub の型の写しだけなので依存は付けていない。
- **やったこと:**
  - `ugoe-wasm`: `dress_up::part_ids(puppet)` (合成したモデルの `parts` の ID の順。モデルが無ければ空) と、それを返す
    `WebPuppet::part_ids()` (`src/web/dress_up.rs`。`attached_modules` の隣) を足した。doc コメントに `@internal`。単体テスト
    `part_ids_follow_the_composed_model` (付ける・外すと `Dress/Dress` が増減する)。
  - 「エディタ向けの問い合わせ (ギズモ・物理の表示)」の節を「アプリ向けの問い合わせ (ギズモ・物理の表示・パーツの並び)」に改め、説明を
    エディタとスタジオの受け手の両方に合わせた。各 `@internal` の doc コメントの文 (Rust・stub の写し) も「アプリ向け。…「アプリ向けの問い合わせ」」
    に揃え、bindings/web/README.md の見出しへのリンクと一覧 (`part_ids` を足した)、scripts/check-wasm-types.mjs と stub の検査のコメントも直した。
    stub の写しに `part_ids` を足し、stub の検査は `part_ids` が `@internal` であることも確かめる。
  - `packages/player`: `FrameQueue.setModelParts(model)` を足し、引き直し先をシーンの `partIds` ではなく受け手のモデルの並びにした
    (渡されていなければ前と同じくシーンの `partIds` をモデルの並びとみなす。`FrameQueue` の単体テストの互換のため)。`Player` は
    `#syncDressUp` (モデルを読んだ・付け外しのたび。フレームごとではない) と `startFollowing` で `WebPuppet.part_ids()` を 1 回読んで渡す。
    付け外しのたびに受けたシーンの表を作り直すので、`continueScene` を待つ間 (見せ方の画像を取っている間) に届いた前のシーンのフレームも
    今のモデルに当たる。`wasm.ts` の写しに `part_ids()` (必須) を足した。`Player.followScene`・`continueScene` の引数の形は変えていないので
    apps/studio (`Output.svelte`) は触っていない。
  - テスト (先に失敗することを確かめた): `playerFollower.test.ts` の「モジュールを読めずに飛ばした」(LongHair を飛ばし、指揮役の
    `Dress/Dress` (2) が受け手の 1 に当たる。前の実装では `[2,1]` のまま当てて `Hat/Hat` と `Dress/Dress` に当たり失敗)。
    `follower.test.ts` に `setModelParts` の単体テスト。偽の WebPuppet に `parts`・`moduleParts` の設定と `part_ids` を足し、T0853 の
    着せ替えのテストは偽物で実際に付け直すように直した。
  - docs/studio.md#フレームの配信-内部の形 の制限の記述と、packages/player/README.md の受け手のモードの説明を直した。
  - 検査: `just wasm-check`・`just wasm`・`just wasm-types`・stub の検査・`just player-browser` (24/24)・`just e2e --only studio` (13/13)・
    `just ci` が通った (`follow_match.rs` の一致の検査も通った)。
- **やらなかったこと:** scripts/check-wasm-types.test.mjs の検査用の入力に書いた「@internal エディタ向け。」の文はそのまま
  (印の有無だけを見る入力なので)。docs/roadmap.md の M8 の後続の一覧の T1400 の行はそのまま (完了の印はオーケストレーターに任せる)。
- **起票の候補:** なし。
