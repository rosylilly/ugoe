---
id: T0881
title: "スタジオ: トラッキングの調整で、眉・口の形・視線の境目を向き (上げと下げ・笑みと下がり・上下左右) で分けられるようにする"
status: todo
milestone: M8
crates: [packages/face-tracking, apps/studio, tests/ui-audit, docs]
depends_on: []
assignee:
---

# T0881: 眉・口の形・視線の境目を向きで分ける

## 目的

T0879 で、トラッキングの調整に眉 (`brow`)・口の形 (`mouthForm`)・視線 (`eyeLook`) の境目を足した。どれも 1 つの境目の組を、向きの違う
ブレンドシェイプにまとめて当てている (眉は上げ (`browInnerUp`・`browOuterUp*`) と下げ (`browDown*`)、口の形は笑み (`mouthSmile*`) と
下がり (`mouthFrown*`)、視線は上下左右 (`eyeLookUp*`・`eyeLookDown*`・`eyeLookIn*`・`eyeLookOut*`))。顔やカメラによって向きごとに出方が
違う (眉は上げやすいが下げが出にくい、など) ので、向きごとに境目を持てるようにする。

## 決まっていること (利用者、2026-10-11)

- 眉・口の形・視線の境目を向きで分けられるようにする (T0879 の起票の候補「S38 で要望が出たら」は、今作ると決まった)。

## 作るもの (案。細部は作業者が決めて理由をメモに書く)

- 形は T0880 (首の感度を向きで分ける) と T0879 (瞼の左右を別々にする) にそろえる: まとまりごとに「向きで分ける」のスイッチ (既定は切) と、
  分けたときの向きごとの境目。入れた直後は今の境目を各向きに写す (入れただけでは出力が変わらない)。切ったときは向きの境目を残す (使わない)。
- 視線の向きは 4 つ (上・下・内・外) にするか、上下と左右の 2 つにするかを決める (内と外は左右の目で向きが逆になることに注意)。
- 既定値ならフレームをそのまま返す (ビット単位で同じ)・`isDefaultTuning` は分けているまとまりがあれば偽、は T0879・T0880 と同じ。

## 受け入れ条件

- [ ] `packages/face-tracking/src/tuning.ts`: 型・既定値・`sanitizeTuning` (真偽でないスイッチ・NaN・範囲の外・狭い境目)・当てる組 (`tuningRanges`)・
      `isDefaultTuning` を広げた。T0879 の形 (向きで分けない境目だけ) の `input.tuning` とビューアの localStorage はそのまま読める (分けないまとまりとして)。単体テスト
- [ ] スタジオの `input.rs` の `TrackingTuning` に同じ項目を足した (無い項目は既定値、範囲の外は寄せる。フロントエンドと同じ JSON の形のテスト)
- [ ] 画面: `TuningSection.svelte` の「眉」「口」「目」の小見出しに、まとまりごとの「向きで分ける」のスイッチと、向きごとの境目のスライダーとメーター。日本語・英語
- [ ] ui-audit: スタジオに向きで分けた場面を足し、`just ui-audit --only studio`・`--only viewer`・`--only editor` が通る。`just e2e --only studio` が通る
      (マニュアルの画像が変わるなら撮り直す)
- [ ] 文書: docs/studio.md の調整の表と例、docs/manual/studio.md、packages/face-tracking/README.md、docs/manual-check.md の S38 (向きで分けた境目の手順)
- [ ] `just ci` が通る

## 参考

- T0879・T0880 のメモ、packages/face-tracking/src/tuning.ts・TuningSection.svelte・tuning.test.ts、apps/studio/src-tauri/src/input.rs、docs/studio.md (調整の表)、docs/manual-check.md の S38

## メモ

- T0089 で起票した (元の候補: T0879 (眉・口の形・視線を向き (上げと下げ・笑みと下がり・上下左右) で分けて境目を持つか)。利用者の決定 2026-10-11: 今作る)。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。
- リリース前なので、プロファイルの `input.tuning` の形を変えても移行の書き方は要らない (利用者の方針)。
- ビューア (T0738) は `sanitizeTuning`・`isDefaultTuning` を使うので追従する。ビューアのテスト (`apps/viewer/src/lib/trackingTuning.test.ts`) が
  調整の形を書いていれば、型を合わせる小さな直しは範囲の外でも行ってメモに書く (T0879・T0880 と同じ)。
- **並行の注意:** T0882 (頬・口の左右・目の見開きにも境目を広げる) と同じ `tuning.ts`・`input.rs`・`TuningSection.svelte` を触るので、T0882 はこのタスクの後に行う (T0882 の depends_on)。
