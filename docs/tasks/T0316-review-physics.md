---
id: T0316
title: コードレビュー: ugoe-physics
status: done
milestone: M9
crates: [ugoe-physics, docs]
depends_on: []
assignee: claude-worker-T0316
---

# T0316: コードレビュー: ugoe-physics

## 目的

ugoe-physics のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-physics` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - `docs/spec/physics.md` との一致、`dt` が 0・巨大・負のときの安定性、NaN の伝播
  - 衝突 (T0311) とリグの順 (T0312)・加算の出力 (T0313) の正しさ
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0318〜T0322 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 確かめたこと

- 仕様 (`docs/spec/physics.md`・`spec/physics.md`) との一致: 時間の進め方・入力の正規化・振り子の式・出力・合成の方法 (T0313)・
  リグの順と中継 (T0312)・衝突体の判定と掃引 (T0311)・動的物理を、式ごとに実装と照らした。食い違いは下の表の 1 件 (静止姿勢の検査の中継) だけ。
- `dt`: 0・負・NaN・無限大は無視する (既存のテスト)。巨大な有限値で余りの時間が壊れる不具合を直した (下の表)。
- NaN: パラメータの値は `ParameterValues` が NaN を無視する (T0137) ので、入力の正規化に NaN は来ない。
  設定の値 (重力・係数・重み) は検査されていないので、無限大・NaN を入れて動かした。パニック・無限ループはなかったが、
  動的物理の出力を通して描画の頂点が NaN になる・鎖が NaN のまま戻らないことがある (T0318・T0320)。
  衝突の掃引は、区間が届く範囲の外の衝突体を調べず、区間の長さが 0 や非有限なら押し出しにするので、NaN でも止まらない。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/lib.rs` (`advance`) | ステップの上限を超えた時間を `accumulator -= whole * step` で捨てるので、巨大な `dt` で桁落ちし、余りが負や 1 ステップ超になる。`dt = f32::MAX` では余りが負の無限大になり、その後は物理が二度と進まない | 直した (上限を超えたときは剰余で余りを求める。`huge_dt_keeps_the_accumulator_within_a_step` を先に足した) |
| 中 | `src/lib.rs` (`rest_pose_collisions`) | 中継の入力を今のパラメータの値から読んでいた。最初の `update` は前のリグの静止姿勢の出力を読むので、中継で動くリグの静止姿勢の当たりを見逃す・誤って知らせる | 直した (`with_relay` を関数にして共有し、前のリグの静止姿勢の出力を写しに書く。`rest_pose_collisions_read_earlier_rest_outputs` を先に足した) |
| 中 | `ugoe-model` `validate/physics.rs` | 重力・風・粒子の係数・入力と出力の重み・`scale`・正規化の範囲が有限値か検査しない。無限大・NaN の設定で、動的物理の出力から描画の頂点が NaN になる | T0318 を起票 |
| 中 | `src/lib.rs`・`src/chain.rs` | 粒子が一度 NaN になると速度が NaN のまま戻らない (リセットまで止まる)。動的物理の加算量には NaN がそのまま足される | T0320 を起票 (どこで止めるかは仕様の判断) |
| 中 | `src/lib.rs` (`write_parameters`・`root_pose`・`Relay::new`) | 毎フレーム・毎ステップの `Vec` の確保、中継での `ParameterValues` の丸ごとの複製 (ID の `String` を含む)、出力と入力ごとのパラメータ名の線形探索。物理の計測もない | T0319 を起票 |
| 低 | `src/tests.rs` | 出力の `reflect`・重みの 0〜1 のクランプ・未知の `source` の無視にテストがなかった | 直した (`output_reflect_weight_clamp_and_unknown_source`) |
| 低 | `src/chain.rs:10`・`src/lib.rs:146` | 仕様へのリンクの見出しが古い (`#根元の回転`・`#根元の取り付け先`) | 直した |
| 低 | `src/lib.rs`・`src/chain.rs`・`src/collider.rs` | 公開の `PhysicsSystem::new`・`settings`・`Particle::position`・`Chain::particles`・`WorldCollider` の各フィールドに doc コメントがない。`Chain::step`/`settle` に `new` と違う長さの `defs` を渡すと添字が範囲外になることが書かれていない | 直した (doc コメントを足した) |
| 低 | `src/chain.rs`・`src/collider.rs` | `Chain`・`Particle`・`Segment`・`StepInput`・`WorldCollider`・`constrain_segment`・`signed_angle`・`rest_offset` が公開だが、クレートの外から使われていない | 見送り (公開の範囲を狭めるのは公開 API の変更。1.0 の API の整理で T0008 がまとめて判断する) |
| 低 | `src/collider.rs` (`nearest_free`・掃引) | 刻みが `衝突体の半径 / 区間の長さ` なので、区間の長さが極端に大きい (1e6 px など) と調べる角度の数が増える (上限は `π / 刻み`) | 見送り (現実の設定では数百回まで。極端な長さは T0318 の検査の範囲で、刻みの下限を足すと公開仕様の式が変わる) |
| 低 | `src/lib.rs` (`update_instance`) | 別のモデルのインスタンスを渡すと結び付けはし直すが、粒子は前のモデルの座標のまま続く | 見送り (`reset` を呼ぶのがホストの責任と読める。仕様は「モデル切替時に reset」と書いている) |

### 横断 (T0008 向け)

- 設定・モデルの数値の「有限か」の検査の漏れ: 検査が参照先の存在に偏り、数値が NaN・無限大でも通るものがある (物理では T0318)。モーション・表情など、ほかの設定の検査にもありそう。
- 時間の累積 (`accumulator += dt` と `-= n * step`) は巨大な `dt` で桁落ちする。固定ステップや時間の累積を持つほかの箇所 (モーションの再生時間・目パチ・呼吸) も同じ確かめが要る。
- 仕様の見出しへのリンク (`docs/spec/*.md#...`) が、見出しを変えたときに古くなる。リンク切れの検査がない。
- 毎フレームの `ParameterValues::index_of` (名前の線形探索) と `ParameterValues` の複製は、物理のほかのクレートにもありそう (T0319)。
- 公開しているがクレートの外で使われていない型・関数 (物理の `Chain` など)。1.0 の前に公開の範囲を見直す。

### `just ci`

- `just ci` と `just wasm-check` は通った (rebase 後、main は 05c82cf)。
