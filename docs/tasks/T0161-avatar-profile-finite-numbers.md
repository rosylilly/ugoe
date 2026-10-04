---
id: T0161
title: アバタープロファイルの数値の有限性を準拠の検査で見る
status: done
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

- [x] 準拠の検査が、プロファイルの数値の NaN・±無限大を問題として報告する (場所つき)。テストがある
- [x] 範囲として意味の無い値 (例: `range` の下限が上限より大きい) も報告するかを決め、するなら同じく足した
- [x] `docs/spec/avatar-profile.md#準拠の検査` と英語の公開仕様 (`spec/`) の対応する節を更新した
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0141 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-model/src/avatar/check.rs、docs/spec/model.md (「アバタープロファイルの数値はこの検証の対象外」)

## メモ

- T0019 で起票した (元の候補: T0141)。

### 決めたこと (方針は `docs/spec/model.md#数値の検証の方針`、T0141・T0323 に揃えた)

- 着手時点で、`source` が知られている上書きの `scale`・`offset` と `look_at` の `range_x`・`range_y` は既に有限性を見ていた。
  抜けていたのは **`source` が未知の上書き** で、`scale` が NaN でも準拠になった (既存のテストがそれを期待していた)。
  T0323 が表情の `op` が未知の項目の `value` も検査するのと同じ理由 (`f32` が有限値である規則は新しい版でも変わらない) で、未知の上書きの数値も検査するようにした。
  入力と対象は従来どおり検査しない (その上書きは無視されるので)。
- 範囲: プロファイルには下限と上限の組の値が無いので、順序の検査は足さない。`range_x`・`range_y` は既に「有限で 0 より大きい」。
  `scale`・`offset` には範囲を定めない (0 や負の係数で固定・反転する上書きにも意味がある)。仕様に書いた。
- 標準パラメータの値域が NaN のときは、従来どおり値域の不一致の警告 (`ParameterRange`) になる。パラメータの数値は構造の検証 (`InvalidValue`) が見るので、ここでは変えない。

### やったこと

- `avatar/check.rs`: `check_tracking` が未知の `source` でも `check_numbers` (`scale`・`offset` の有限性) を呼ぶ。
  テスト `non_finite_numbers_are_errors` (NaN・+無限大・-無限大を `scale`・`offset`・`range_x`・`range_y` と未知の上書きで。0 と負の係数はエラーにならない) を足し、
  `tracking_overrides_are_checked` の未知の上書きの期待に `InvalidValue` を足した (`ProfileIssue` の `PartialEq` で NaN が等しくならないので、その値は無限大に替えた)。
- 仕様: `docs/spec/avatar-profile.md#準拠の検査` (表の行と数値の方針の段落)、`docs/spec/model.md#数値の検証の方針` (対象外の一文を準拠の検査への参照に)、
  `spec/avatar-profile.md` (Conformance checks の行と段落、Overrides の validator の一文)、`spec/model.md` (Data conventions)。- `just ci` は通った (exit 0)。`just wasm-check` も通った。

### やらなかったこと・起票の候補

- ランタイムの対応付け (`ugoe-motion` のトラッキング) が有限でない `scale`・`offset` の上書きを無視することは仕様にあるが、このタスクでは確かめていない (範囲外)。
- 起票の候補: なし。
