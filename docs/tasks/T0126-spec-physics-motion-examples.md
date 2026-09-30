---
id: T0126
title: 公開仕様: 物理・モーション・効果の例のファイルと、独立の実装での検証
status: done
milestone: M5
crates: [spec, tools/spec-check, docs]
depends_on: [T0120]
assignee: claude-worker-T0126
---

# T0126: 公開仕様: 物理・モーション・効果の例のファイルと、独立の実装での検証

## 目的

T0120 の後続の候補。独立の読み込み器 (`tools/spec-check`) で確かめたのは読み込み・検証・変形の評価・着せ替えの取り付けまでで、
物理 (spec/physics.md)・モーションと効果 (spec/motion.md) は例のファイルも独立の実装もない。

## 受け入れ条件

- [x] 作業者は T0120 と同じく `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する (手順は `tools/spec-check/README.md`)
- [x] `spec/examples/` に物理・モーション・効果 (目パチ・呼吸・リップシンク) の例と期待値 (時刻ごとのパラメータの値など) を足す。例の形式を `spec/examples/README.md` に書く
- [x] 参照実装がそれらの例を満たすことをテストで確かめる (`crates/` 側のテストの追加は team-lead に相談する。作業者はテストを実行して結果だけを見る)
- [x] `tools/spec-check` に物理・モーション・効果の独立の実装を足し、例の期待値と一致させる
- [x] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md`・`docs/adr/0021-public-specification.md`

## メモ

### 独立性を保った手順

- 作業者 (claude-worker-T0126) は `crates/` と `docs/spec/` を読まず、`spec/`・`tools/spec-check/`・ワークフローの文書だけを参照した (T0120 と同じ手順。`tools/spec-check/README.md#独立性を保つ手順`)。
- 期待値は参照実装の出力を写していない。仕様から別に書いた Python の転記 (作業者の scratchpad。複素数で 2 次元のベクトルを表す、TS の実装とは別の書き方) で出し、
  静止の姿勢・閉じた式で書ける値 (当たり判定の接触の角度、重力の座標系の角度、Bézier、目パチの SplitMix64 など) は手で確かめた。
  TS の独立の実装は、それとは別に書いて一致を確かめた。
- 参照実装との照合は team-lead が行った (crates/ 側のテスト f34a858。作業者は読んでいない)。10 例すべてが許容誤差の範囲で一致した (offsets と events を含む)。

### やったこと

- `spec/examples/` に時間の例 10 個 (physics-pendulum・physics-relay・physics-dynamic・physics-frames・physics-colliders・motion-curves・motion-crossfade・expressions・effects・pose)。
  形式 (`timeline.json`: ホストの操作・dt・フレームごとの期待値) と許容誤差を `spec/examples/README.md#timeline-examples` に書いた。物理の設定は `model.physics.json`、モーションと表情は `*.motion.json`・`*.expression.json` で、JSON Schema を満たす。
- `tools/spec-check`: 物理 (`physics.ts`)・モーションと効果とポーズ (`motion.ts`)・1 フレームの順 (`frame.ts`)・時間の例の読み込みと比較 (`timeline.ts`、`test/timeline.test.ts`)。
  評価に動的な物理のオフセット・パーツの不透明度の上書き・ヤコビアンを足した。目パチの seed を `bigint` で読むようにした。CLI に時間の例を走らせる使い方を足した。

### 見つけた仕様の抜け・曖昧さ

| # | 箇所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 1 | physics.md Rig order and relays | 中継でない入力 (後ろのリグの出力を読む) は「前の更新で書いた出力を読む」とあるが、motion.md のフレームの順では手順 1 で基準の値に戻るので、読むのは基準の値 | フレームの順では基準の値を読み、物理だけを単体で回すホストは前の出力を読む、と明記 (参照実装も同じ) |
| 2 | motion.md Motion playback | 終わりに達したフレームで、終わったモーションは何も適用しない (終わりの時刻の曲線の値は適用されず、前のフレームの値が残る) ことが読み取りにくい | 明記 (参照実装も同じ) |
| 3 | motion.md Eye blink | 最初の間隔を引く時点 (「目パチが始まるとき」) が曖昧 | ランタイムを作るとき (最初のフレームの前) と、設定を差し替えたとき、と明記 |
| 4 | motion.md Part opacity | 「ポーズが有効」が定義されていない | 「pose の段が on」と明記し、ポーズがすべての要素とリンクに毎フレーム書くこと、空のポーズは何も書かないことを書いた |
| 5 | physics.md Angle outputs | 鏡映の親 (det J < 0) での符号の反転が、粒子 1 だけかすべての粒子か | すべての粒子 (J は出力先のデフォーマで決まる) と明記 |
| 6 | physics.md Colliders | 「速い動きは当たり判定を通り抜けない」とあるが、掃引するのは親のまわりの回転だけで、親の移動で運ばれる線分は調べない | 明記 (4820299。docs/spec の同期は e2268a3) |
| 7 | model.md Motions と motion.md Curves | 最初のセグメントの規則を破る曲線を、model.md は「ランタイムは無視してよい (MAY)」、motion.md は「値を持たない (無視する)」としていて強さが食い違う | 未修正。参照実装がどちらかを確かめてから揃える (後続) |
| 8 | motion.md Expressions | 表情をすべて止めるホストの操作が無い (set で置き換えるだけ) | 未修正 (必要なら仕様に足す) |

- spec/ を直した分 (1〜5) の `docs/spec/` の同期は未了 (team-lead に依頼)。6 は同期済み。

### 確かめていないこと・残した課題

- 追従 (トラッキング) を含む時間の例は作っていない (`timeline.json` にトラッキングの入力の形が無い)。
- ホストがポーズの表示を直接選ぶ操作・効果の設定の差し替えは、例の操作に入れていない。
- 期待値の導出に使った Python の転記はコミットしていない (作業者の scratchpad)。再生成が要るなら tools/ に置くことを検討する。

### 後続の候補

- 上の 7・8 の仕様の整理。
- トラッキング (avatar-profile.md#computing-the-parameter-values) の時間の例と独立の実装。
- 描画 (rendering.md) の例と独立の実装、書き出しの独立の実装 (T0120 の後続の候補のまま)。
