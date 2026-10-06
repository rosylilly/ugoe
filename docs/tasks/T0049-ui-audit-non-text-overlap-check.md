---
id: T0049
title: "ui-audit: 文字ではない部品どうしの重なり (選択欄の矢印が隣の欄の下に隠れるなど) を数える検査を足すか決める"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T0045]
assignee: claude-worker-T0049
---

# T0049: ui-audit: 文字ではない部品どうしの重なり (選択欄の矢印が隣の欄の下に隠れるなど) を数える検査を足すか決める

## 目的

`just ui-audit` の `overlap` は文字の重なりだけを数える。T0044 で、エディタのグルーの軸を足す行が狭い列 (`*-zoom200`) で
`Select` の矢印を右の数値の欄の下に隠しているのを画像で見つけたが、監査は出さなかった (`ja-light-zoom200-glue-edit.png`)。
押せる部品 (ボタン・欄・選択欄) どうしの重なりを数える検査を足すかを決め、足すなら実装する。

## 受け入れ条件

- [x] 足すか足さないかを決め、理由をメモと tests/ui-audit/README.md に書いた (誤検出の多さ・意図した重なり (バッジ・ビューポートの上の道具) の扱い)
- [x] 足すなら: `tests/ui-audit/lib/page-checks.js` に検査があり、単体テストで「選択欄の矢印 (か選択欄の右端) が隣の欄の下に隠れる」形を見つけ、
      意図して重ねた形 (重ねた側に印のある要素) を数えない
- [x] 足すなら: T1650 で直す前のコードで `-glue-edit` の `*-zoom200` の指摘が出ることを確かめた。3 つのアプリのほかの指摘をメモに書いた
      (直すタスクは起票の候補にする)
- [x] `just ui-audit-check` と `just ci` が通る

## 参考

- T0044 のメモ (新しい場面で見つけたもの・起票の候補)
- tests/ui-audit/README.md、tests/ui-audit/lib/page-checks.js

## メモ

- T0046 で起票した (元の候補: T0044 (文字ではない部品の重なりの検査を検討する))。
- T0045 (作業中) が同じ page-checks を触るので、その後にした。T1650 (グルーの行を直す) とは依存を付けていない。T1650 が先に入ったら、
  直す前のコミットで確かめる。

### やったこと (T0049)

- **足すと決めた。** 理由: 流れの中に置いた押せる部品どうしの重なりは、縮めすぎ・負の余白などの崩れでほぼ起き、意図して重ねる形は位置を
  流れから外す (absolute・fixed・sticky・最上層) ので見分けられる。意図した重なりを除くと、今の 3 つのアプリのすべての場面で誤検出が 0 だった。
- `lib/page-checks.js` に `controlOverlapAudit` (DOM の側) と `coveredControls` (判定の本体。Node から試せる) を足し、結果は `overlap` の検査に
  `<下の部品> is covered by <上の部品>` で入れた (報告の見出しは「文字・部品の重なり・切れ」)。部品は `target-size` と同じ (数値の欄は枠の全体)、
  矩形は送る枠と窓で切った見える部分。幅と高さがともに 2px を超える重なりの中ほどの `elementFromPoint` で上下を決める。
  数えないもの: 入れ子・2px 以下の重なり・上の点がどちらでもない (幕が覆う)・意図して重ねた (上の側から共通の祖先の手前までに absolute・fixed・
  sticky・`dialog:modal`・`:popover-open`・`data-ui-audit-overlay`)。README の「押せる部品の重なり」に書いた。
- 単体テスト (`test/checks.test.mjs`): 選択欄の右端が数値の欄の下に入る形を見つけ、直した形 (次の行) と、枠の線の 2px の重なり・入れ子・
  印のある側を上に重ねた形・幕に覆われた形を数えない。
- **T1650 の前のコードで確かめた:** `GluesPanel.svelte` の `.grow` を `min-width: 0` に戻し (コミットしない一時の変更)、
  `just ui-audit --only editor --variants zoom200` で、`button.ugoe-select__trigger ... is covered by span.ugoe-number-field__box ""` が
  `en-light-zoom200-glue-edit` と `ja-light-zoom200-glue-edit` に出た (ほかの zoom200 の 78 の場面は 0)。
- **今の main (T1650 の後) で 3 つのアプリのすべての場面を流した:** エディタは `--variants` を 2 回に分けた
  (`light-wide,dark-wide` の 163 場面と `narrow,forced-wide,zoom200` の 321 場面。600 秒の時間の上限のため。組の漏れは無い)、ビューア 89・
  スタジオ 183 の場面。部品の重なりはすべて 0。ほかの検査では、エディタの `en-dark-forced-wide-tab` で `div[tree] "Parts"` の
  `faint focus ring (8.73:1 at most)` が 1 つ出た (この検査とは別。T0045 の画像の判定。下の起票の候補)。
- 意図して重ねたものの扱いを外して (一時の変更) 英語のライトの広い幅と狭い幅を流すと、28 種類 (エディタ 15・ビューア 6・スタジオ 7) が出た。
  すべて開いたメニュー・設定のダイアログ・送っても残る頭 (Section の頭・タブの列)・タイムラインのキーの印が下の部品を覆うもので、意図したもの。

### やらなかったこと

- 部品が部品ではないもの (札・アイコン・絵) の下に隠れる形、上の側を absolute などで流れから外したための崩れは見ない (意図した重なりと
  見分けられない)。README の「見ないこと」に書いた。
- 3 つのアプリのコードに `data-ui-audit-overlay` を付けた所は無い (要る所が無かった)。

### 起票の候補

- エディタ: 強制色 (`en-dark-forced-wide-tab`) でツリー (`div[tree] "Parts"`、止まり 10) のフォーカスの印が `faint focus ring` と判定される
  (前と後の比は最大 8.73:1 だが、3:1 以上変わった画素が要る量に届かない)。印が細いのか判定の閾値の問題かを見る。
