---
id: T0844
title: "スタジオ: 読み直したシーンで選んだポーズの要素から始め、follow_match にポーズの切り替えを入れる"
status: done
milestone: M9
crates: [apps/studio, packages/player, docs]
depends_on: [T0329, T0843]
assignee: claude-worker-T0844
---

# T0844: スタジオ: 読み直したシーンで選んだポーズの要素から始め、follow_match にポーズの切り替えを入れる

## 目的

T0842 で、スタジオの指揮役と受け手 (出力ウィンドウ・ブラウザソース・仮想カメラ) はシーンのポーズの要素を表示させるようになった。ただし:

- モデルを読み直したシーンでも、選んだ要素へ先頭の要素からフェードで切り替わる (T0329 でフェードを飛ばす口を足す)。
- 指揮役と受け手の一致の検査 (`apps/studio/src-tauri/tests/follow_match.rs`) にポーズの切り替えが入っていない。

(T0842 のやらなかったこと・候補)

## 受け入れ条件

- [x] 指揮役と受け手が、シーンを読み直した (モデル・着せ替えを読み込んだ) 直後はシーンの要素をフェードせずに表示し、同じモデルのままの切り替え
      (パネル・ホットキー・操作 API) は今までどおりフェードする。`@ugoe/player` の `showPose` に口を足し、`Output.svelte.test.ts` と Rust のテストで確かめる
- [x] `follow_match.rs` に、シーンの `pose` を替えた出し直しで、指揮役と受け手のパーツの不透明度が揃うことを確かめるケースがある
- [x] docs/studio.md#ポーズの要素 を直した
- [x] `just ci` が通る (仮想カメラの `Feed::set_scene` を触ったら、GPU のジョブのテスト `virtual_camera_gpu.rs` の扱いをメモに書く)

## 参考

- T0842 のメモ、T0329
- apps/studio/src-tauri/tests/follow_match.rs、apps/studio/src/Output.svelte、packages/player (`showPose`)

## メモ

- T0040 で起票した (元の候補: T0842 (フェードを飛ばす口のスタジオでの使い方)・T0842 (`follow_match.rs`))。T0843 と同じスタジオのポーズの処理を触るので、その後に着手する。

### やったこと

- **`@ugoe/player`:** `Player.showPose(part, { immediately?: boolean })` を足した (`immediately` なら wasm の `show_pose_immediately`。
  既定は今までどおりフェード)。`WebPuppet` の interface と偽物 (`testing/fakePuppet.ts`。`state.immediate` に記録) にも足した。
  `playerPose.test.ts` で確かめる。README を直した。
- **指揮役 (`conductor.rs`):** `show_pose` に `immediately` を足し、`present` は **モジュールの版がそのシーンの版になったとき** (モデルを
  読んだ・モジュールの組が変わった) にフェードせずに表示させる。同じモデルと組のまま (パネル・ホットキー・`pose.set`・同じモデルのシーンへの
  切り替え) は今までどおりフェード。単体テスト `a_reloaded_scene_shows_its_pose_without_a_fade`。
- **受け手:** 出力ウィンドウ・ブラウザソース (`Output.svelte`) は、`planScene` が `reload`・`modules` のときに `{ immediately: true }`。
  `Output.svelte.test.ts` で、読み直し・付け直しは `pose!:`、見せ方だけの出し直しは `pose:` で呼ぶことを確かめる。
  仮想カメラ (`virtual_camera.rs` の `Feed::set_scene`) は、モデルを読み直したかモジュールを付け直したときに飛ばす。
  ネイティブの受け手の規則は `follower.rs` の `show_pose(puppet, parts, immediately)` にまとめ、仮想カメラと `follow_match.rs` が使う。
- **`follow_match.rs`:** `pose_switches_of_the_scene_match_the_conductor` を足した。モデルを読んだシーン (`ArmB`)・同じモデルのまま `ArmA` を
  選んだ出し直し・モデルの読み直し (`ArmB`) を、仮想カメラと同じ規則の受け手で受け、各フレームで指揮役と受け手のポーズの不透明度と形を比べる
  (許容誤差 1e-3)。読み直しの直後は `ArmA` 0・`ArmB` 1 であること、同じモデルのままの切り替えで実際にフェードしていることも確かめる。
- 文書: docs/studio.md (ポーズの要素の「出す」・「受け手」と「フェード」の項・一致の検査の説明)、docs/manual/studio.md (ポーズの節に 1 文)。
- **`virtual_camera_gpu.rs` の扱い:** `Feed::set_scene` を触ったが、GPU のテストはポーズを使わないモデル (動的物理の髪など) なので変えていない。
  手元で `cargo test -p ugoe-studio --features gpu-tests --test virtual_camera_gpu` を流し、3 件とも通った。ポーズの規則そのものは
  `follower::show_pose` を通して `follow_match.rs` (GPU 不要) で確かめている。
- ロック (舞台 → 指揮役) の取り方は変えていない (`present` の中だけの変更)。
- `just ci`・`just vcam-check` が通った。

### やらなかったこと

- モジュールの付け直しでフェードを飛ばすことは、Rust ではポーズを持つモジュールのフィクスチャが無いので単体テストしていない
  (`Output.svelte.test.ts` では確かめた。指揮役は `modules_revision == revision` で、モデルの読み直しと同じ分岐)。
- 実機の WebView・実際の配信での見た目は確かめていない。

### 起票の候補

- ポーズを持つモジュールのフィクスチャ (`ugoe_model::fixtures`) を足し、着せ替えの直後にフェードを飛ばすことを `conductor.rs`・`follow_match.rs` で確かめる。
