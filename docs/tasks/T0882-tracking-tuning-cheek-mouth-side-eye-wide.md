---
id: T0882
title: "スタジオ: トラッキングの調整の境目を、頬 (cheekPuff)・口の左右 (mouthLeft・mouthRight)・目の見開き (eyeWide*) に広げる"
status: todo
milestone: M8
crates: [packages/face-tracking, apps/studio, tests/ui-audit, docs]
depends_on: [T0881]
assignee:
---

# T0882: 頬・口の左右・目の見開きの境目

## 目的

トラッキングの調整の境目 (`TUNING_RANGES`) は、瞼・口の開き (T0877)、視線・口の閉じ・口の形・眉 (T0879) にある。頬の膨らみ (`cheekPuff`)・
口の左右 (`mouthLeft`・`mouthRight`)・目の見開き (`eyeWideLeft`・`eyeWideRight`) は境目が無く、出にくい顔やカメラでは引き伸ばせない。
これらにも境目を広げる。

## 決まっていること (利用者、2026-10-11)

- 頬・口の左右・目の見開きにも境目を広げる (T0879 の起票の候補「S38 の結果で」は、今作ると決まった)。

## 受け入れ条件

- [ ] `packages/face-tracking/src/tuning.ts`: `TUNING_RANGES` に頬・口の左右・目の見開きのまとまりを足した (まとめ方 (口の左右を 1 つにするか、
      目の見開きを瞼と同じく左右を別々にできるようにするか) は作業者が決めてメモに書く。T0881 の「向きで分ける」を当てるかも)。
      `sanitizeTuning`・`isDefaultTuning`・既定値ならフレームをそのまま返すこと、の単体テスト。前の形の `input.tuning` とビューアの localStorage がそのまま読める
- [ ] スタジオの `input.rs` の `TrackingTuning` に同じ項目を足した (無い項目は既定値、範囲の外は寄せる。フロントエンドと同じ JSON の形のテスト)
- [ ] 画面: `TuningSection.svelte` の「目」「口」の小見出し (頬をどこに置くかも決める) に、境目のスライダーとブレンドシェイプごとのメーター。日本語・英語
- [ ] ui-audit: スタジオの `-inputs-tuning` (小見出しをすべて開いた場面) が新しいスライダーを含み、`just ui-audit --only studio`・`--only viewer`・
      `--only editor` が通る。`just e2e --only studio` が通る (マニュアルの画像が変わるなら撮り直す)
- [ ] 文書: docs/studio.md の調整の表、docs/manual/studio.md、packages/face-tracking/README.md、docs/manual-check.md の S38
- [ ] `just ci` が通る

## 参考

- T0879 のメモ (`TUNING_RANGES` の足し方・`tuningRanges`・小見出し)、packages/face-tracking/src/tuning.ts・TuningSection.svelte、apps/studio/src-tauri/src/input.rs、docs/manual-check.md の S38

## メモ

- T0089 で起票した (元の候補: T0879 (頬 (`cheekPuff`)・口の左右 (`mouthLeft`・`mouthRight`)・目の見開き (`eyeWide*`) などにも境目を広げるか)。利用者の決定 2026-10-11: 今作る)。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。
- リリース前なので、プロファイルの `input.tuning` の形を変えても移行の書き方は要らない (利用者の方針)。
- ビューアのテストの型を合わせる小さな直しは、範囲の外でも行ってメモに書く (T0881 と同じ)。
- **並行の注意:** T0881 (眉・口の形・視線を向きで分ける) と同じ `tuning.ts`・`input.rs`・`TuningSection.svelte` を触るので、T0881 の後に行う (depends_on)。
