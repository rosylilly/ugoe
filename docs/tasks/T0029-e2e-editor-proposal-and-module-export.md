---
id: T0029
title: "e2e: エディタの提案のプレビューと「Export Module…」の筋書き、見本を開く手順の時間切れの調査"
status: todo
milestone: M9
crates: [tests/e2e, docs]
depends_on: []
assignee:
---

# T0029: e2e: エディタの提案のプレビューと「Export Module…」の筋書き、見本を開く手順の時間切れの調査

## 目的

T0023 でエディタの e2e に PSD の取り込み・書き出し・着せ替えのプレビュー・画像からのトラッキングを足したが、T0685 の残りの
提案のプレビュー (`get_proposal_preview_bundle`) と、モジュールのプロジェクトの「Export Module…」(`.ugoemod` の書き出し) は通していない (T0023 の候補)。

また、T0022 で、並行のビルドの直後にエディタの「opens the sample project」が 1 回だけ見本を開けずに時間切れ (20 秒) になった。原因は調べていない (T0022 の候補)。

## 受け入れ条件

- [ ] AI の提案を偽の提供者 (ネットワークなし) で出し、プレビューを開いて閉じる手順がエディタの筋書きにある
- [ ] モジュールのプロジェクトを開いて「Export Module…」で `.ugoemod` を書き出し、ファイルができて一時ファイルが残らないことを確かめる手順がある
- [ ] 「opens the sample project」の時間切れを負荷の下 (並行のビルドなど) で再現しようとし、原因を直すか、再現しなかった条件と待ちの長さの見直しをメモに書いた
- [ ] tests/e2e/README.md の筋書きの表を更新した
- [ ] `just e2e` と `just ci` が通る

## 参考

- T0023・T0022 のメモ (起票の候補)、T0685
- tests/e2e/specs/editor.test.mjs

## メモ

- T0026 で起票した (元の候補: T0023 の提案のプレビューと Export Module…、T0022 の時間切れ)。
- T0022 の候補の「エディタの筋書きにプレビューのトラッキング (画像で MediaPipe)」は T0023 で入った (`lib/tracking.mjs` の `checkImageTracking`) ので起票しない。
