---
id: T0103
title: ポーズとエフェクト設定のデータ定義
status: done
milestone: M1
crates: [ugoe-model]
depends_on: []
assignee: claude-worker-T0103
---

# T0103: ポーズとエフェクト設定のデータ定義

## 目的

ポーズ (排他的なパーツ切り替え) と、目パチ・呼吸・リップシンクの対象パラメータの設定を `ugoe-model` にデータとして追加する (インターフェースを先に入れるタスク)。

## 受け入れ条件

- [x] `ugoe_model::pose` と `ugoe_model::effects` (仮称) モジュールがあり、serde で往復できるテストがある
- [x] docs/spec/model.md と docs/spec/motion.md を更新する
- [x] アルゴリズムは実装しない (T0303 で行う)
- [x] `just ci` が通る

## 参考

- `docs/spec/motion.md#エフェクト`

## メモ

- 配置: `Model.pose` / `Model.effects` として `#[serde(default)]` で埋め込んだ。既存の JSON / MessagePack はそのまま読め、バンドルは `MODL` チャンクに含まれるのでファイル形式と `SCHEMA_VERSION` は変えていない。`validate` がパーツ・パラメータの参照を検査できるのも理由。
- `Effects::default()` は対象パラメータなし (何もしない)。標準パラメータを持たない既存モデルが検証で落ちないようにするため。推奨設定は `Effects::standard()` (目パチ `EyeLOpen`/`EyeROpen`、リップシンク `MouthOpenY`、呼吸 5 本)。
- エフェクトグループ名は `effects::EYE_BLINK_GROUP` (`EyeBlink`) と `LIP_SYNC_GROUP` (`LipSync`)。呼吸とポーズはグループを持たない。
- 検証を追加: ポーズの `part` / `links` の参照、ポーズ全体でのパーツ重複 (`DuplicatePosePart`)、エフェクトの対象パラメータの参照、時間・周期の値 (`InvalidValue`)。
- フィクスチャ `fixtures::pose_and_effects()` を追加した。
- T0303 への申し送り: 目パチの間隔の分布、呼吸の加算とモーションとの関係、ポーズの初期表示パーツの決め方 (現状は各パーツのキーフォームの不透明度から決める想定) を決める。ポーズのグループや要素に ID は持たせていない (エディタで個別に指す必要が出たら追加する)。
