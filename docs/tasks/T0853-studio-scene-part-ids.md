---
id: T0853
title: "スタジオ: シーンの通知にパーツの ID の並びを載せ、着せ替えの直後に前のフレームのパーツの番号がずれないようにする"
status: in-progress
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

- [ ] `stream.rs` の `Scene` に `partIds` (合成したモデルの `parts` の ID の順。着せ替えのモジュールのパーツは合成の ID) を足し、
      指揮役がシーンを出すときに埋める。フレームのバイト列の形は変えない (番号は指揮役の並びのまま)
- [ ] 受け手が、フレームの `revision` に対応するシーンの `partIds` で番号を ID に直し、今のモデルの番号に引き直す。今のモデルに無い ID は
      捨てる。毎フレームの確保を増やさない (シーンを受けたときに引き直しの表を作る):
  - Rust の受け手 (`apps/studio/src-tauri/src/follower.rs`。仮想カメラもここを通る)
  - TS の受け手 (`packages/player/src/follower.ts`。出力ウィンドウとブラウザソース)
- [ ] テスト: モジュールのパーツを切り替えるモーションを再生しながら着せ替えたとき、前のシーンのフレームを組み替えたモデルに当てても、
      指揮役と同じパーツに当たること (前の実装では別のパーツに当たる入力で、先に失敗するテストを書く)。Rust と TS の両方。
      `apps/studio/src-tauri/tests/follow_match.rs` の一致の検査が通る
- [ ] docs/studio.md のフレームの配信 (`scene` の中身の表・JSON の例・「着せ替えの直後に…ずれうる」の行) を直した
- [ ] `just ci` が通る

## 参考

- T0135 のメモ (制限・後続の候補)、docs/studio.md#フレームの配信-内部の形・#シーンの切り替えと受け手
- apps/studio/src-tauri/src/stream.rs・conductor.rs・follower.rs、packages/player/src/follower.ts、apps/studio/src/lib/outputLayout.ts

## メモ

- T0075 で起票した (元: docs/roadmap.md の M8 の「起票していない後続の候補」の「シーンの通知にパーツの ID を載せて着せ替え直後の番号の
  ずれを無くす (T0135)」)。
