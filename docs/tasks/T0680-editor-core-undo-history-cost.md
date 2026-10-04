---
id: T0680
title: Undo の履歴の大きさと、適用ごとの検証の重複を抑える
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0667]
assignee: claude-worker-T0680
---

# T0680: Undo の履歴の大きさと、適用ごとの検証の重複を抑える

## 目的

T0667 のレビューの所見。`Document` はスナップショット方式の Undo (ADR 0005 の暫定の実装) で、

- `undo` の履歴に上限がない。コマンドごとに `Model`・物理・モーションと表情を丸ごと複製して積むので、長い編集の作業でメモリが増え続ける
  (テクスチャの画像は `Arc` で共有するので含まない)。
- `Document::apply_with_report` は、適用のたびに適用前のモデルも `validate` し直す (`validate(&self.current.model)`)。適用前の状態の検証の結果は
  状態が変わらない限り同じなので、スナップショットに覚えておける。物理の検証 (`validate_physics`) も同じ。

履歴の上限 (数か、おおよその大きさ) を決めて古いものから捨てる (保存済みの印 `saved` との関係に注意する)、適用前の検証の結果を覚える、を行う。
上限の値と、上限を超えたときの画面の扱い (知らせるか) は `docs/editor.md#undo` に書く。逆操作の方式への移行 (ADR 0005) はこのタスクではしない。

## 受け入れ条件

- [x] 履歴が上限を超えると古いものから捨てられ、`is_modified` が正しいまま (テストで確かめる)
- [x] 適用前の状態の検証を、同じ状態で 2 度行わない
- [x] 大きなモデル (`ugoe_model::fixtures` の最大のもの) で、上限まで編集したときのメモリのおおよその量を測ってメモに書いた
- [x] `just ci` が通る

## 参考

- docs/adr/0005-editor-command-model.md
- docs/editor.md#undo
- T0667 のメモ

## メモ

### やったこと

- 履歴を `src/document/history.rs` の `History` に分けた。Undo と Redo を合わせて上限 (`DEFAULT_UNDO_LIMIT` = 100) 個までで、超えると最も古い Undo から捨てる。
  `Document::undo_limit`・`set_undo_limit`・`with_undo_limit` を足した (下げると古い Undo から、尽きたら遠い Redo から捨てる。0 なら履歴なし)。
  `is_modified` は状態の番号で比べるので、保存した状態を捨てても正しいまま (戻れなくなるだけ)。テスト `src/undo_tests.rs`。
- 状態 (`Snapshot`) に検証の結果 (`validate` と `validate_physics`) を `OnceLock` で覚えるようにした。適用で作った状態は適用後の検証の結果を持ち、
  作った状態・読み直した状態・`with_physics` の後は初めて要ったときに 1 度だけ求める。`SetModuleSettings` の `keep_look` の検査も同じ記憶を使う。
  テスト `a_state_is_validated_once` (テストのときだけ数える `checks()`) で、Undo で戻った状態や拒否の後に検証し直さないことを確かめた。
- `docs/editor.md#undo` に上限・画面の扱い・未保存の印・メモリの目安・検証の記憶を書いた。

### 判断したこと (推奨で進めた)

- **上限は数で 100 個** (おおよその大きさではなく)。大きさで測るにはモデルの大きさを見積もる処理が要り、逆操作の方式 (ADR 0005) に移れば不要になるため。
- **利用者の設定にはまだしない。** API (`set_undo_limit`) だけを用意し、エディタの設定に出すかは後で決める (起票の候補)。
- **上限を超えても画面では知らせない。** 一般的なエディタと同じく、最も古い状態の後の Undo は何もしない (今の画面は Undo のボタンを常に押せ、`false` を無視する)。

### メモリの測り方と結果

一時的な example (コミットしていない) で、数える `GlobalAlloc` を使い、`SetParameterRange` を上限 + 20 回適用した後の生きている確保の増分を測った (リリースビルド)。

| モデル | `model.json` の大きさ | 1 状態 | 100 状態 |
| --- | --- | --- | --- |
| `fixtures::avatar_profile` (`ugoe_model::fixtures` で最大) | 約 7 KB | 約 8.5 KB | 約 0.8 MB |
| `fixtures::dynamic_hair` (物理つき) | 約 4 KB | 約 7.6 KB | 約 0.7 MB |
| `assets/fixtures/character.psd` を取り込んだもの (13 メッシュ) | 約 27 KB | 約 22 KB | 約 2.2 MB |
| `assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd` を取り込んだもの (25 メッシュ) | 約 81 KB | 約 54 KB | 約 5.3 MB |

`ugoe_model::fixtures` はどれも小さいので、実際の PSD の取り込みも測った。キーフォームの格子はメッシュの頂点をセルの数だけ持つので、リギングが進むと
ほぼセルの数に比例して増える (例: 頂点 200 のメッシュ 100 個に 9 セルずつなら 1 状態 約 1.4 MB、100 状態で約 140 MB の見積もり。測ってはいない)。

### やらなかったこと

- 逆操作の方式への移行 (ADR 0005。タスクの範囲外)。
- T0667 のメモの「`keep_look` の `rederive` の警告を `ApplyReport` で返す」(見送り。ミラーのやり直しが失敗する `keep_look` の組み立てが大きい)。

### 起票の候補

- エディタの設定に Undo の上限を出す (`set_undo_limit` を Tauri のコマンドと設定画面から)。大きなモデルで上限を下げたい利用者向け。
- Undo / Redo のボタンを、戻れない・進めないときに押せなくする (`Document` に `can_undo`・`can_redo` を足し、状態とともに画面へ送る)。
