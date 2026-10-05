---
id: T0328
title: "見本: アバターにポーズを切り替えるモーションと、振る手を回転デフォーマで揺らす"
status: done
milestone: M9
crates: [ugoe-cli, assets, docs]
depends_on: []
assignee: claude-worker-T0328
---

# T0328: 見本: アバターにポーズを切り替えるモーションと、振る手を回転デフォーマで揺らす

## 目的

T0474 で、着せ替えの本体にポーズを切り替えるモーション `wave` を入れたが、手は動かない絵のまま表示を切り替えるだけで、
見本のアバター (`avatar.ugoe`) にはポーズを切り替えるモーションが無い (T0474 の候補 2 つ)。見本として、ポーズとデフォーマの動きを組み合わせた例にする。

## 受け入れ条件

- [x] 着せ替えの本体の振る手 (`HandWave`。袖の `SleeveWave` と `Sailor` の袖も) を回転デフォーマに付け、`wave` で実際に揺れる
      (パラメータを足すなら名前と範囲を assets/samples/README.md の表に書く)
- [x] 見本のアバターにポーズを切り替えるモーション (例: 耳飾りを外す) を足すか決めて、足したなら `sample::animations::check` を通り、足さないなら理由をメモに書いた
- [x] テスト (`crates/ugoe-cli/src/sample` の既存の形) で、`wave` の途中で手のメッシュの頂点が動くことと、ポーズの切り替えが今までどおりであることを確かめる
- [x] `assets/samples/` を作り直し、`committed_sample_is_up_to_date` が通る。assets/samples/README.md・docs/manual/motions.md の見本の説明を直した
- [x] `just ci` が通る

## 参考

- T0474 のメモ (判断と理由・起票の候補)
- crates/ugoe-cli/src/sample/dress_up.rs・sample/animations.rs、assets/samples/README.md

## メモ

- T0038 で起票した (元の候補: T0474 (2 つ))。

### 判断と理由

- **振る手を揺らすパラメータ `ArmWave` (−1〜1、既定 0、グループ `Body`) を本体に足した。** 標準パラメータに腕の振りは無く、モーションが動かせるのは
  パラメータ・パーツの不透明度・エフェクトグループだけなので、回転デフォーマを動かすにはパラメータが要る。手 `HandWave` と既定の袖 `SleeveWave` は
  肘 (キャンバスの (365, 500)) を原点にした回転デフォーマ `WaveRotation` (`BodyWarp` の子) に付け、`ArmWave` ±1 で ±12 度回る (正で画面の右へ)。
- **`Sailor` の袖:** モジュールの最上位の要素はスロットの取り付け先 (`BodyWarp`) に付き、本体の `WaveRotation` を親にはできないので、
  `Sailor` に同じキーフォームの回転デフォーマ `WaveRotation` を持たせ、袖をその子にした。`ArmWave` は標準パラメータではないので、
  スロット `Outfit` の `parameters` で公開した (`requires` は足していない。公開の検査は合成が行う)。本体の袖と `Sailor` の袖の四隅が
  どの値でも同じ位置に来ることをテストで確かめた。
- **モーション `wave`:** 手が上がった 0.4〜2 秒に `ArmWave` を 0 → 1 → −0.5 → 1 → −0.5 → 1 → 0 (0.3 秒ごと、ベジェ) と動かす。
  −1 まで倒すと手が頬に重なって見えたので、内側は −0.5 までにした。ポーズの切り替え (0〜2 秒 `Wave`、その後 `HandsDown`) は前のまま。
- **見本のアバターにはポーズを切り替えるモーションを足さなかった。** 耳飾りを外すのは付けたままにしておく着せ替えの差分で、モーションの
  パーツの不透明度のカーブはモーションが終わると効かなくなる (再生の間だけ外れて戻る) ため、モーションの見本として不自然。切り替えは
  ポーズの `show_pose` (ビューアの「ポーズ」の欄) で行うのが合っている。ポーズを切り替えるモーションの見本は着せ替えの `wave` が担う。
  あわせて `avatar.ugoe`・`avatar-lit.ugoe`・`avatar-atlas.ugoe` (と Unity の見本の写し) を作り直さずに済む。理由は assets/samples/README.md の
  モーションの節にも書いた。

### やったこと

- `crates/ugoe-cli/src/sample/dress_up.rs`: `ArmWave`・`WaveRotation`・スロット `Outfit` の `parameters`・`Sailor` の回転デフォーマ。
  テスト `arm_wave_swings_the_hand_and_the_sleeves` (値域とスロットの公開、指先が 40px 以上動き頂点の肘からの距離が変わらない、袖が動き顔は
  動かない、正で右へ、`Sailor` の袖が本体の袖と同じ位置) と、`wave_motion_switches_the_pose` に `wave` の 0.9 秒と 1.2 秒で手 (60px 超) と
  袖 (本体だけ・`Sailor` 付き) が動くことを足した (ポーズの切り替えの確かめは前のまま)。
- `crates/ugoe-cli/src/sample.rs`: `PARAMETER_GROUPS` の `Body` に `ArmWave` を足した (アバターには無いので入らない)。
- `crates/ugoe-cli/src/sample/animations/dress-up/wave.motion.json`: `ArmWave` のカーブ。
- `assets/samples/dress-up/` (`dress-up.ugoe` 83206 → 84150 バイト、`sailor.ugoemod` 16772 → 17076 バイト) を作り直した。
  `just cli validate --profile avatar` は ok。
- 文書: assets/samples/README.md (モーションの表・アバターに足さない理由・グループの表・ポーズの節・本体の要素の表・標準でないパラメータの表・
  スロットとモジュールの表)、docs/manual/motions.md・docs/manual/dress-up.md。`crates:` の外の小さな変更として apps/viewer/README.md・
  bindings/web/README.md の見本のモーションの一言を直した。
- **既定の描画の見た目は変わらない。** `ugoe render` の本体だけと `LongHair`+`Sailor` の PNG が変更の前後でバイト単位で同じことを確かめた
  (手と袖は不透明度 0 のパーツにあり、`ArmWave` = 0 では回転しない)。`dress-up.jpg`・`avatar-*.jpg` は作り直していない。
  `--motion wave --time 0.9 / 1.2` (本体だけ・`Sailor` 付き) を描いて目で確かめた。
- 確かめたこと (WSL2): `just ci` が通った (main の b1d54f2a の上)、`just conformance` の全体 (c・wasm・wasm-webgpu・avatar・queries・unity・godot が ok)、
  `cargo test -p ugoe-ai --test module_rig_sample` (9 件) が通った。

### やらなかったこと

- 振る手のポーズを示す画像 (`*.jpg`) は足していない。文書のスクリーンショットも撮り直していない。

### 起票の候補

- なし。
