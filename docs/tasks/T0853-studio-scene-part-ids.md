---
id: T0853
title: "スタジオ: シーンの通知にパーツの ID の並びを載せ、着せ替えの直後に前のフレームのパーツの番号がずれないようにする"
status: done
milestone: M8
crates: [apps/studio, packages/player, docs]
depends_on: []
assignee: claude-worker-T0853
---

# T0853: スタジオ: シーンの通知にパーツの ID を載せる

## 目的

フレームの `part_opacities` (モーションのパーツの不透明度のカーブ) は、パーツを **指揮役のモデル (着せ替えたら合成したモデル) の
`parts` の番号** で持つ (T0135、docs/studio.md#フレームの配信-内部の形)。受け手は着せ替えの直後に、前のシーンのフレームを
組み替えたモデルに当て続ける (`continueScene`) ことがあり、そのときモジュールのパーツの番号は前と今の合成でずれうる
(本体のパーツは合成でも番号が変わらないので、モーションが本体のパーツを切り替える限り影響は無い。モデルに無い番号は受け手が無視する)。
ずれると、別のパーツの不透明度が数フレームだけ変わって見えうる。

`scene` の通知にパーツの ID の並び (指揮役の合成したモデルの `parts` の順) を載せ、受け手はフレームの `revision` のシーンの並びから
ID を引き、自分の今のモデルの番号に引き直してから `Puppet::follow` に渡す。

フレームの配信は内部の形 (版を上げて変えてよい。スタジオが配るページといつも揃う) なので、利用者の判断は要らない。

## 受け入れ条件

- [x] `stream.rs` の `Scene` に `partIds` (合成したモデルの `parts` の ID の順。着せ替えのモジュールのパーツは合成の ID) を足し、
      指揮役がシーンを出すときに埋める。フレームのバイト列の形は変えない (番号は指揮役の並びのまま)
- [x] 受け手が、フレームの `revision` に対応するシーンの `partIds` で番号を ID に直し、今のモデルの番号に引き直す。今のモデルに無い ID は
      捨てる。毎フレームの確保を増やさない (シーンを受けたときに引き直しの表を作る):
  - Rust の受け手 (`apps/studio/src-tauri/src/follower.rs`。仮想カメラもここを通る)
  - TS の受け手 (`packages/player/src/follower.ts`。出力ウィンドウとブラウザソース)
- [x] テスト: モジュールのパーツを切り替えるモーションを再生しながら着せ替えたとき、前のシーンのフレームを組み替えたモデルに当てても、
      指揮役と同じパーツに当たること (前の実装では別のパーツに当たる入力で、先に失敗するテストを書く)。Rust と TS の両方。
      `apps/studio/src-tauri/tests/follow_match.rs` の一致の検査が通る
- [x] docs/studio.md のフレームの配信 (`scene` の中身の表・JSON の例・「着せ替えの直後に…ずれうる」の行) を直した
- [x] `just ci` が通る

## 参考

- T0135 のメモ (制限・後続の候補)、docs/studio.md#フレームの配信-内部の形・#シーンの切り替えと受け手
- apps/studio/src-tauri/src/stream.rs・conductor.rs・follower.rs、packages/player/src/follower.ts、apps/studio/src/lib/outputLayout.ts

## メモ

- T0075 で起票した (元: docs/roadmap.md の M8 の「起票していない後続の候補」の「シーンの通知にパーツの ID を載せて着せ替え直後の番号の
  ずれを無くす (T0135)」)。
- **やったこと:**
  - `stream.rs` の `Scene` に `part_ids` (`partIds`) を足し、`Conductor::present` が `conductor::part_ids` (合成したモデルの
    `parts` の ID の順) で埋める (`clear` は空)。フレームのバイト列は変えていない。
  - Rust の受け手 (`follower.rs`): `set_scene(revision, puppet, part_ids)`・`continue_scene(revision, part_ids)` にし、シーンごとに
    番号 → 今のモデルの番号の表 (`ScenePartMap`。並びがモデルと同じなら表を持たず引き直さない) を作る。`continue_scene` では
    (モジュールを付け直した後に呼ぶので) 持っている表を今のモデルで作り直す。`receive` はフレームの `revision` 以下で最も新しいシーンの
    表で引き直し (モデルに無い ID は捨てる)、使い回す `Vec` に入れて `apply_frame` に渡す。適用したフレームのシーンより前の表は捨てる。
    仮想カメラ (`virtual_camera.rs`) はシーンの `part_ids` を渡す。
  - TS の受け手 (`follower.ts`): `PartRemap` (表と使い回す入れ物) を足し、`FrameQueue.setScene`・`continueScene` に `partIds` を、
    `take` の結果に `parts` (引き直しが要らなければ `null`) を足した。`applyFrame(puppet, frame, gap, parts)` で引き直す。
    `Player.followScene`・`continueScene` に `partIds` を足し、出力ウィンドウ (`Output.svelte`) がシーンの `partIds` を渡す。
    `WebPuppet` に合成したモデルのパーツの ID を読む口が無いので、TS では今のシーンの `partIds` を受け手のモデルの並びとみなす
    (シーンのモデルとモジュールを同じ順に付けるので同じ並び)。
  - テスト (先に失敗することを確かめた): `follow_match.rs` の `frames_of_the_previous_scene_hit_the_same_parts_after_dressing_up`
    (`[Dress]` → `[LongHair, Dress]` で `Dress/Dress` の番号がずれる。前の実装では `LongHair/Hair` に当たって失敗)、`follower.rs` の
    単体テスト、`follower.test.ts` の `FrameQueue part numbers (T0853)`、`playerFollower.test.ts` の着せ替えの後の引き直し、
    `Output.svelte.test.ts` で `partIds` を渡すこと。
  - docs/studio.md のフレームの配信 (`scene` の表・JSON の例・項目の表・「ずれうる」の行) を直した。
  - `just e2e --only studio` は 13 件すべて通った。`just ci` は通った。
- **範囲の外の小さな変更:** `tests/ui-audit/lib/studio-host.mjs` の偽のシーンに `partIds: []` を足した (1 行。シーンの形を揃えるため)。
- **制限 (やらなかったこと):**
  - wasm の受け手は今のシーンの `partIds` を自分の並びとみなすので、モジュールを読めずに飛ばしたときは並びが食い違い、そのモジュールより後の
    パーツの番号はずれうる (次のシーンの通知で付け直すまで。前からある制限)。ネイティブの受け手は自分のモデルに引き直すのでずれない。
  - 受け手が飛ばしたシーン (続けて届いたもの) のフレームは、その前に受けたシーンの並びで引き直す。飛ばしたシーンで組が変わっていたら、
    そのフレームのモジュールのパーツはずれうる。
- **起票の候補:**
  - `WebPuppet` に合成したモデルのパーツの ID の並びを読む口 (`part_ids()`) を足し、wasm の受け手も自分のモデルに引き直す
    (モジュールを読めなかったときのずれを無くす。ugoe-wasm の公開 API なので別タスク)。

