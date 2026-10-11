---
id: T0879
title: "スタジオ: トラッキングの調整を眉・視線・口の形・口の閉じに広げるか、瞼の左右を別々にするか決める (判断が要る)"
status: in-progress
milestone: M8
crates: [packages/face-tracking, apps/studio, tests/ui-audit, docs]
depends_on: []
assignee: claude-worker-T0879
---

# T0879: トラッキングの調整の範囲を広げる (判断が要る)

## 目的

T0877 で、キャリブレーションの後・平滑化の前に当てる利用者の調整 (`@ugoe/face-tracking` の `applyTuning`) を、首振り (ヨー・ピッチ・ロールの感度と遊び)・
瞼 (`eyeBlinkLeft`・`eyeBlinkRight` を一緒に)・口の開き (`jawOpen`) の 3 つで入れた (利用者の決定: まずこの 3 つ。ほかの入力に後で広げられる形にする)。
境目は `TUNING_RANGES` に組を足せば広げられる形になっている。どこまで広げるかと、瞼の左右を別々にするかを決める。

標準のパラメータとブレンドシェイプの対応 (crates/ugoe-motion/src/tracking.rs): 眉 `BrowLY`・`BrowRY` (`browOuterUp*`・`browInnerUp`・`browDown*`)、
視線 `EyeBallX`・`EyeBallY` (`eyeLook*`)、口の形 `MouthForm` (`mouthSmile*` − `mouthFrown*`)、口の開き `MouthOpenY` = `max(0, jawOpen − mouthClose)`。

## 判断が要る

**1. 広げる範囲**

- **案 A (推奨): 眉・視線・口の形・口の閉じを足す。** 眉 (`browInnerUp`・`browOuterUp*`・`browDown*`)・口の形 (`mouthSmile*`・`mouthFrown*`)・
  口の閉じ (`mouthClose`) は境目 (`low`・`high`) で、視線 (`eyeLook*`) は境目で「見きった」と見なす値を決める。画面は項目が倍になるので、
  「首」「目」「口」「眉」の小見出しで分け、開いた項目だけ中身を作る (T0877 の折りたたみと同じ)。既定値なら出力はビット単位で今と同じ。
- 案 B: 実際の配信者の要望 (docs/manual-check.md の S38 の結果) が出たものだけ足す。今は 3 つのまま。
- 案 C: すべてのブレンドシェイプに境目を持たせる (詳細の設定)。何でも合わせられるが、項目が 50 を超え、どれを動かせばよいか分からない。

推奨は A。細目の人の瞼と同じく、眉の動きが小さい人・笑っても口角が上がりにくい人は、境目が無いとモデルの表情がほとんど動かない。
視線は検出の値が 0.5 ほどまでしか上がらないことが多く、境目で引き伸ばす需要がある。`mouthClose` は T0877 で `jawOpen` だけを伸ばすと決めたが、
口を閉じたつもりで `MouthOpenY` が 0 にならない人のために「閉じたと見なす値」を足す。

**2. 瞼の左右**

- 案 A: 今のまま左右を一緒に (T0877 で決めた理由: 細目の度合いは左右でほぼ同じで、検出の左右差は首の向きで変わるので、固定の値で左右差を直すと正面で片目だけ閉じきる)。
- **案 B (推奨): 「左右を別々にする」スイッチを足す (既定は切)。** 切のときは今と同じ 1 組の境目。入れると左と右の境目を別々に持つ。
  プロファイルの `input.tuning` に `eyeBlinkRight` (と `eyeBlinkSeparate` のような真偽) を足す。左右は利用者から見た左右 (T0877 の `swapSides` との順序のまま)。
- 案 C: 常に左右を別々に持つ。スライダーが倍になり、多くの人には要らない。

推奨は B。片目だけ瞼が下がりやすい人・片目だけ眼鏡の反射で検出が弱い人は、左右が一緒だと片方に合わせると片方が閉じきらない。
既定を切にすれば、T0877 の理由 (多くの人には一緒でよい) と両立する。

## 受け入れ条件

- [ ] 1 と 2 の判断をメモに書いた
- [ ] (1-A) 足した項目の調整を `applyTuning` と `sanitizeTuning` に入れ、単体テスト (既定値で出力が変わらない・境目の引き伸ばしとクランプ・NaN と範囲の外) で確かめた
- [ ] (2-B) 左右を別々にしたときと一緒のときの両方を単体テストで確かめた (`swapSides` との順序を含む)
- [ ] スタジオのプロファイルの `input.tuning` に読み書きでき、無い項目・壊れた値は既定値に戻る (Rust の `input.rs` とフロントエンドのテスト。Rust と TS で並びが違っても送り合わない)
- [ ] 画面 (`TuningSection.svelte`) に項目とメーターを足した (日本語・英語)。部品は packages/ui のもの
- [ ] docs/studio.md#入力とホットキーの項目-t0803 の表と例、docs/manual/studio.md#調整-感度と境目、packages/face-tracking の README、docs/manual-check.md の S38 を直した
- [ ] `just ui-audit --only studio` (と、パネルが出るビューアとエディタの分) が通る
- [ ] `just e2e --only studio` が通る
- [ ] `just ci` が通る

## 参考

- T0877 のメモ (決めたこと: 遊びの形・瞼の左右は一緒・境目は 2 つ・`swapSides` との順序・記録は調整の前・口の開きは `jawOpen` だけ)
- packages/face-tracking/src/tuning.ts (`TUNING_RANGES`・`applyTuning`・`sanitizeTuning`)・TuningSection.svelte、apps/studio/src-tauri/src/input.rs (`TrackingTuning`)、
  crates/ugoe-motion/src/tracking.rs (標準のパラメータの式)、docs/spec/avatar-profile.md#実装の範囲

## メモ

- T0083 で起票した (元の候補: T0877 (調整の範囲を広げる: 眉・視線・口の形・mouthClose など。`TUNING_RANGES` に足す。瞼の左右を別々に決められるようにするか))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0880 (首振りの感度を非対称にするか) と同じ `tuning.ts`・`input.rs`・`TuningSection.svelte` を触る。並べて進めるときは片方が入ってから追従する。
- リリース前なので、プロファイルの `input.tuning` の形を変えても移行の書き方は要らない (利用者の方針)。

- 判断 (利用者、2026-10-11): 推奨の案に決まった。
