---
id: T0176
title: "CLI: ugoe build に META と物理・モーション・表情の JSON を添える口を足し、Unity の資料の作り直しを寄せる"
status: in-progress
milestone: M9
crates: [ugoe-cli, ugoe-format, bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0176
---

# T0176: CLI: ugoe build に META と物理・モーション・表情の JSON を添える口を足し、Unity の資料の作り直しを寄せる

## 目的

T0174 で `ugoe build` がモデル・モジュールの JSON からバンドルを書けるようになったが、`META` (アバターのメタデータ) と、モデルの JSON に添える物理・モーション・表情は
書けない (T0174 の候補)。また、Unity の検査の資料 (`bindings/unity/tests/Fixtures/`) は `crates/ugoe-format/tests/unity_fixtures.rs` が
`UGOE_UPDATE_FIXTURES=1` で作り直し、同じテストが照合もしている。作り直しの経路を `ugoe build` に寄せ、テストは照合だけにするかを決める (T0174 の候補)。

## 受け入れ条件

- [ ] `ugoe build` に `--meta <json>` と、モデルの JSON の入力に `--physics <json>`・`--motion <json>` (複数)・`--expression <json>` (複数) がある
      (名前と形は既存の `ugoe` のサブコマンドの引数に揃える。読み込みは T0530 の線引きどおり補助のファイルは通常のファイルだけ)。`tests/cli.rs` で、
      書いたバンドルを読み直すと添えたものが入っていることと、壊れた JSON・検証の失敗がエラーになることを確かめる
- [ ] Unity の資料の作り直しを決めた: `ugoe build` だけで今の資料と同じものを作れるなら、`unity_fixtures.rs` の作り直しの経路 (`UGOE_UPDATE_FIXTURES`) を外して
      照合だけにし、作り方を `bindings/unity/tests/Fixtures/README.md` に `just cli build ...` で書く。作れないものが残るなら、理由をメモに書いて今の経路を残す
- [ ] crates/ugoe-cli/README.md の `ugoe build` を直した
- [ ] `just ci` が通る (Unity の資料を変えたら `just unity-check` も)

## 参考

- T0174 のメモ (資料の作り直しについて・起票の候補)
- crates/ugoe-cli/src/json_source.rs、crates/ugoe-format/tests/unity_fixtures.rs、bindings/unity/tests/Fixtures/README.md

## メモ

- T0040 で起票した (元の候補: T0174 の 2 つ)。
