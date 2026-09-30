---
id: T0126
title: 公開仕様: 物理・モーション・効果の例のファイルと、独立の実装での検証
status: todo
milestone: M5
crates: [spec, tools/spec-check, docs]
depends_on: [T0120]
assignee:
---

# T0126: 公開仕様: 物理・モーション・効果の例のファイルと、独立の実装での検証

## 目的

T0120 の後続の候補。独立の読み込み器 (`tools/spec-check`) で確かめたのは読み込み・検証・変形の評価・着せ替えの取り付けまでで、
物理 (spec/physics.md)・モーションと効果 (spec/motion.md) は例のファイルも独立の実装もない。

## 受け入れ条件

- [ ] 作業者は T0120 と同じく `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する (手順は `tools/spec-check/README.md`)
- [ ] `spec/examples/` に物理・モーション・効果 (目パチ・呼吸・リップシンク) の例と期待値 (時刻ごとのパラメータの値など) を足す。例の形式を `spec/examples/README.md` に書く
- [ ] 参照実装がそれらの例を満たすことをテストで確かめる (`crates/` 側のテストの追加は team-lead に相談する。作業者はテストを実行して結果だけを見る)
- [ ] `tools/spec-check` に物理・モーション・効果の独立の実装を足し、例の期待値と一致させる
- [ ] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md`・`docs/adr/0021-public-specification.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
