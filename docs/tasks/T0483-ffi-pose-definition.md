---
id: T0483
title: "C ABI・Unity: ポーズの定義 (グループごとの要素のパーツ) を読む口"
status: in-progress
milestone: M9
crates: [ugoe-ffi, bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0483
---

# T0483: C ABI・Unity: ポーズの定義 (グループごとの要素のパーツ) を読む口

## 目的

T0478 で C ABI に表示中の要素を読む口 (`ugoe_puppet_pose_group_count`・`ugoe_puppet_pose_visible_part`) が入ったが、ポーズの定義
(各グループにどの要素があるか。wasm の `pose_json` に当たるもの) を読む口が無い (T0478 の候補)。C ABI だけのホスト (Unity など) が
ポーズを選ぶ欄を作れるように足す。

## 受け入れ条件

- [ ] C ABI にグループの要素の数と要素のパーツの番号を読む関数 (`ugoe_puppet_pose_entry_count`・`ugoe_puppet_pose_entry_part` など。範囲の外は
      `UGOE_ERROR_OUT_OF_RANGE`) がある。付けたモジュールのグループも含む。`tests/host.rs` で確かめる。関数を足すだけなら ABI の版は上げない (README の規則)
- [ ] ヘッダを作り直し (`just refs` が通る)、C の見本・crates/ugoe-ffi/README.md に足した
- [ ] Unity の `Native` の宣言と `Puppet` の読む関数 (グループごとの要素の ID の配列など) と、`HostApiTests` のテストがある
- [ ] `just ffi-check`・`just conformance`・`just unity-check`・`just ci` が通る

## 参考

- T0478 のメモ (C ABI の口と、JSON でなく番号にした理由)
- crates/ugoe-ffi、bindings/unity、docs/spec/motion.md#ポーズ

## メモ

- T0040 で起票した (元の候補: T0478 (C ABI のポーズの定義))。
