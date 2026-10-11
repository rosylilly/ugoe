---
id: T0880
title: "スタジオ: 首振りの感度を左右・上下で別々にする (非対称にする) か決める (判断が要る)"
status: done
milestone: M8
crates: [packages/face-tracking, apps/studio, tests/ui-audit, docs]
depends_on: []
assignee: claude-worker-T0880
---

# T0880: 首振りの感度を非対称にする (判断が要る)

## 目的

T0877 の首振りの調整は、軸 (ヨー・ピッチ・ロール) ごとに 1 つの感度 (倍率) と遊び (中立のまわりの角度) で、正の向きと負の向きに同じ値を使う。
カメラを画面の横に置いていると、カメラから遠い側に首を振ったときの検出の角度が小さく出る (右だけ振りにくい) ことがある。
キャリブレーションは中立をずらすだけで、片側の振れ幅は直せない。向きごとに感度を分けるかを決める。

## 判断が要る

- **案 A (推奨): 軸ごとに「向きで分ける」スイッチを足し、入れたら正の向きと負の向きの感度を別々に持つ (既定は切。遊びは共通のまま)。**
  切のときは今と同じ 1 つの感度。プロファイルの `input.tuning.yaw` などに `gainNegative` (と分けるかの真偽) を足す。遊びの境で値が飛ばない
  (遊びの外は遊びの分を引いてから向きの感度を掛ける。0 をまたいで続く) ことは今と同じ。
- 案 B: 常に正と負の感度を持つ (スライダーが倍。多くの人には要らない)。
- 案 C: 入れない。カメラを正面に置くよう案内する (docs/manual/studio.md)。

推奨は A。配信の机ではカメラをモニターの横や上の端に置くことが多く、片側の振れ幅を直したい要望は出やすい。既定を切にすれば画面は今と同じで、
T0879 (瞼の左右を別々にする) と同じ「分ける」スイッチの形にそろえられる。遊びまで分ける需要は、遊びは中立のまわりの揺れを止めるもので向きに依らないので、無いと見る。

## 受け入れ条件

- [x] 判断をメモに書いた
- [x] (A・B) 向きで分けた感度を `applyTuning`・`tuneAngle`・`sanitizeTuning` に入れ、単体テスト (既定値で出力が変わらない・正と負で別の倍率・遊びの境で続く・
      NaN と範囲の外・分けないときは今と同じ) で確かめた。ヨーとロールは利用者から見た左右 (`swapSides` の後) に当たることを確かめた
- [x] (A・B) スタジオのプロファイルの `input.tuning` に読み書きでき、無い項目・壊れた値は既定値に戻る (Rust の `input.rs` とフロントエンドのテスト)
- [x] (A・B) 画面 (`TuningSection.svelte`) に項目を足し、メーターに正と負の向きが分かる形にした (日本語・英語)
- [ ] (C) docs/manual/studio.md#調整-感度と境目 にカメラの置き方の案内を足した
- [x] docs/studio.md#入力とホットキーの項目-t0803 の表と例、docs/manual/studio.md、packages/face-tracking の README、docs/manual-check.md の S38 を直した
- [x] `just ui-audit --only studio` (と、パネルが出るビューアとエディタの分) が通る
- [x] `just e2e --only studio` が通る
- [x] `just ci` が通る

## 参考

- T0877 のメモ (遊びの形・感度は遊びを引いた後に掛ける・感度 0〜4・遊び 0〜20°・`swapSides` との順序)
- packages/face-tracking/src/tuning.ts (`AxisTuning`・`tuneAngle`)・TuningSection.svelte、apps/studio/src-tauri/src/input.rs (`TrackingTuning`)

## メモ

- T0083 で起票した (元の候補: T0877 (首振りの感度を左右・上下で非対称にする (右だけ振りにくいカメラの位置など) かどうか))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0879 と同じ `tuning.ts`・`input.rs`・`TuningSection.svelte` を触る。並べて進めるときは片方が入ってから追従する。
- リリース前なので、プロファイルの `input.tuning` の形を変えても移行の書き方は要らない (利用者の方針)。

- 判断 (利用者、2026-10-11): 推奨の案に決まった。
- やったこと (案 A):
  - `AxisTuning` に `separate` (真偽。既定 偽) と `gainNegative` (0〜4。既定 1) を足した。`tuneAngle` は `separate` が真で角度が負のときだけ
    `gainNegative` を掛ける (遊びは共通。遊びの中は 0 なので 0 をまたいで続く)。分けていても両方の感度が 1・遊び 0 なら角度をそのまま返し、
    既定値ならフレームをそのまま返す (ビット単位で同じ)。`setAxisSeparate` (入れたときに今の感度を負の向きに写す・切ったときは残す) を足した。
    `isDefaultTuning` は分けている軸があれば偽 (「既定に戻す」で戻す)。
  - 向きは「向きの約束」の符号で、`swapSides` の後の利用者から見た向き (ヨーの正は自分の左、ピッチの正は上、ロールの正は自分の左へ傾ける)。
    入れ替えの前後で効く感度が替わることを単体テストで確かめた。
  - `sanitizeTuning` は真偽でない `separate` を偽、NaN・範囲の外の `gainNegative` を既定・端にする。T0877 の形 (`gain`・`deadZone` だけ) の
    `input.tuning` とビューアの localStorage はそのまま読める (分けない軸として)。
  - スタジオの `input.rs` の `AxisTuning` に同じ項目を足した (無い項目は既定値、範囲の外は寄せる)。
  - 画面: 軸ごとに「向きで感度を分ける」のスイッチ。入れると「感度 (左へ)」「感度 (右へ)」(ピッチは上へ・下へ) の 2 つのスライダーと、
    メーターの両端の向き (負の向きが左。読み上げはスライダーの名前で足りるので aria-hidden) を出す。日本語・英語。
  - ui-audit: スタジオに `-inputs-tuning-directions` (首の左右を分けた) を足した。マニュアルの画像 `studio-inputs-tuning.png` は首の左右を
    分けた様子で撮り直した (`studio-inputs.png` は時刻の表示だけの差だったので戻した)。
  - 文書: docs/studio.md の表と例、docs/manual/studio.md の表と画像の説明、packages/face-tracking/README.md、docs/manual-check.md の S38 (手順 7 を足した)。
- `crates:` の外: apps/viewer/src/lib/trackingTuning.test.ts の軸の値を新しい形にした (型を合わせるだけの小さな変更。ビューアの実装は変えていない)。
- 確かめたこと: `just ui-audit` (editor・viewer は全体の実行で 0 件。スタジオは全体の実行で強制色の場面の自動の保存の失敗の知らせが出ない
  という場面の失敗が 1 件出て時間切れになったが、`just ui-audit --only studio` を流し直して 0 件)、`just e2e --only studio`・`--only viewer`、`just ci`。
- 確かめていないこと: 実際のカメラを横に置いたときの振れ幅の合い方 (S38 の手順 7 で見る)。
- (C) の案内は入れていない (案 A に決まったため)。
