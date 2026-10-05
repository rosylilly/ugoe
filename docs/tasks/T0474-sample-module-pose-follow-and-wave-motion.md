---
id: T0474
title: "見本: 着せ替えのスロットのポーズの公開と追従、ポーズを要求するモーション"
status: done
milestone: M9
crates: [ugoe-cli, assets/samples, docs]
depends_on: []
assignee: claude-worker-T0474
---

# T0474: 見本: 着せ替えのスロットのポーズの公開と追従、ポーズを要求するモーション

## 目的

T0468 で見本のアバターと着せ替えの本体にパラメータのグループとポーズを持たせた。次が残っている (T0468 の候補)。

1. 着せ替えの見本のスロット `Outfit` の `pose_parts` に `Wave` を公開し、モジュール `Sailor` の袖を `links` で追従させる
   (モジュールのポーズの追従の見本。今は見本に例が無い)。
2. 見本にポーズを切り替えるモーション (例: `wave`。パーツの不透明度のカーブ) を足す。今は `ugoe render` で試すのにモーションの JSON を手で書く。

## 受け入れ条件

- [x] `dress-up` の見本の `Outfit` が `Wave` を公開し、`Sailor` を付けて `Wave` を表示すると袖が追従する。`crates/ugoe-cli/src/sample/dress_up.rs` のテストで確かめた
- [x] 見本に `wave` のモーションがあり、再生するとポーズが切り替わる。テストがある
- [x] `assets/samples` を作り直し (`committed_sample_is_up_to_date` が通る)、assets/samples/README.md と docs/manual の該当箇所に試し方を書いた
- [x] 既定の見た目 (`ugoe render` の PNG) が変わらないか、変わるなら `*.jpg` を作り直した
- [x] `just conformance --only queries` と `just ci` が通る

## 参考

- T0468 のメモ (判断と理由・起票の候補)
- docs/spec/modules.md (スロットのポーズ)、assets/samples/README.md

## メモ

- T0036 で起票した (元の候補: T0468 (`Outfit` の `pose_parts`・ポーズを要求するモーション))。

### 判断と理由

- **既定の袖を手の絵から分けた。** 振る手 `HandWave` は髪 (スロット `Hair` 420〜680) より手前の描画順 700 で、袖を同じ絵に描いたままでは、
  スロット `Outfit` (260〜340) のモジュールの袖が手の後ろに隠れる。そこで既定の袖をメッシュ `SleeveWave` (パーツ `WaveSleeve`、描画順 255) に分け、
  `Wave` の `links` で手と一緒に表示する。`WaveSleeve` は既定の服 `Clothes` の子なので、`Sailor` が `Clothes` を置き換えると一緒に隠れる
  (合成が挟む `#replaced/Clothes` の下になる)。前腕は袖の上の端 (y = 430) で切り、前腕の輪郭線が袖の上の端の輪郭線に重なるようにした。
  袖は髪より奥になるが、本体の短い髪とは重ならない (長い髪のモジュールでは、服と同じく髪の奥に見える)。
- `Sailor` の袖はパーツ `WaveSleeve` (不透明度 0) のメッシュ `SleeveWave` (白に紺の袖口と白い線、モジュールの描画順 250 = 合成で 280) で、
  `pose.links` で本体の `Wave` に追従させる。スロット `Outfit` の `pose_parts` は `Wave` だけ (`HandsDown` は中身の無いパーツなので追従させるものが無い)。
- モーション `wave` は着せ替えの本体だけに入れた (アバターに `Wave` が無いため。`sample::animations::dress_up_motions`)。
  2.5 秒で、`Wave` を 0〜2 秒、`HandsDown` を 2〜2.5 秒要求し (パーツの不透明度のカーブ。`inverse_stepped` で切り替える)、`AngleZ` −12・
  `MouthForm` 1 を足す。終わりにイベント `wave-end`。見本の手は動かない絵なので、手そのものは振らない (ポーズの切り替えの見本)。
  `sample::animations::check` はパーツの不透明度のカーブの対象がモデルのパーツにあることも確かめるようにした。

### やったこと

- `crates/ugoe-cli/src/sample/dress_up.rs`: 上の本体・スロット・`Sailor` の変更。テスト `sailor_sleeve_follows_the_waving_hand`
  (`Outfit` の `pose_parts`・本体だけでは既定の袖・`Sailor` を付けると `Sailor/SleeveWave` が `Wave` と一緒に表示され、既定の袖は隠れ、
  `HandsDown` に戻すと消える) と `wave_motion_switches_the_pose` (本体だけと `Sailor` 付きで、1.5 秒で手と袖が表示され、3.5 秒でモーションが
  終わって手を下ろす)。
- `crates/ugoe-cli/src/sample/animations.rs`・`animations/dress-up/wave.motion.json`: `wave` と `dress_up_motions`。
- `assets/samples/dress-up/` (`dress-up.ugoe` +2KB・`sailor.ugoemod` +1.5KB) を作り直した。assets/samples/README.md (モーションの表・
  ポーズの節の試し方 (`--motion wave`、手で書く JSON の例を置き換えた)・本体とスロットとモジュールの表)、docs/manual/motions.md・
  docs/manual/dress-up.md、apps/viewer/README.md・bindings/web/README.md の見本のモーションの一覧を直した。
  assets/samples/README.md の「ビューアにはモーションと表情を再生する画面がまだありません」は古くなっていたので直した。
- `crates:` の外の小さな変更: `crates/ugoe-ai/tests/module_rig_sample.rs` の `base_with_arms` が見本の本体のポーズを `SetPose` で
  腕の差分だけに置き換えており、`Outfit` の `pose_parts` (`Wave`) が何も指さなくなって合成が失敗した。見本のポーズを残して腕のグループを
  後ろに足すようにし、合成後のグループの数を本体のグループの数から数えるようにした。
- 既定の見た目: `ugoe render` の本体だけと `Sailor`+`LongHair` の PNG が前とバイト単位で同じことを確かめた (振る手と袖はどちらも
  不透明度 0 のパーツにあるため)。`dress-up.jpg`・`avatar-*.jpg` は作り直していない。`--motion wave --time 1.2` (本体だけ・`Sailor` 付き) と
  `--time 3.2` (手を下ろした後) を描いて目で確かめた。
- 確かめたこと (WSL2): `just ci` (main の 1065f4a0 の上で通った)、`just conformance` の全体 (c・wasm・wasm-webgpu・avatar・queries・unity・godot が ok)。

### やらなかったこと

- 見本の手を振る動き (手のメッシュの回転など) は入れていない。ポーズの差分を切り替える見本に留めた。
- 振る手のポーズを示す画像 (`*.jpg`) は足していない。文書のスクリーンショットも撮り直していない。

### 起票の候補

- 見本のアバター (`avatar.ugoe`) にもポーズを切り替えるモーション (耳飾りを外す `no_earwear` など) を入れるか。
- 振る手を実際に振る (手のメッシュを回転デフォーマに付け、パラメータかモーションで揺らす) と、`wave` がより見本らしくなる。
