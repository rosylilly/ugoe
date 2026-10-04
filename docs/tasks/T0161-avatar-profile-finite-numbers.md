---
id: T0161
title: アバタープロファイルの数値の有限性を準拠の検査で見る
status: in-progress
milestone: M9
crates: [ugoe-model, docs, spec]
depends_on: []
assignee: claude-worker-T0161
---

# T0161: アバタープロファイルの数値の有限性を準拠の検査で見る

## 目的

T0141 で、モデルの検証 (`validate`) は数値の有限性を見るようにした。アバタープロファイル (`scale`・`offset`・`range_x` など) は
構造の検証の対象外で (ランタイムは `avatar` を読み飛ばせる)、準拠の検査 (`avatar::check::check_profile`、`ugoe validate --profile avatar`) も
有限性を見ていない。NaN や無限大の入ったプロファイルが「準拠」になり、トラッキングの対応付けで NaN を書きうる。

## 受け入れ条件

- [ ] 準拠の検査が、プロファイルの数値の NaN・±無限大を問題として報告する (場所つき)。テストがある
- [ ] 範囲として意味の無い値 (例: `range` の下限が上限より大きい) も報告するかを決め、するなら同じく足した
- [ ] `docs/spec/avatar-profile.md#準拠の検査` と英語の公開仕様 (`spec/`) の対応する節を更新した
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0141 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-model/src/avatar/check.rs、docs/spec/model.md (「アバタープロファイルの数値はこの検証の対象外」)

## メモ

- T0019 で起票した (元の候補: T0141)。
