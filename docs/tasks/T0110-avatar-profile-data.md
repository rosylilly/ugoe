---
id: T0110
title: アバタープロファイルのデータと検証
status: done
milestone: M3
crates: [ugoe-model, ugoe-format, ugoe-cli]
depends_on: [T0109]
assignee: claude-worker-T0110
---

# T0110: アバタープロファイルのデータと検証

## 目的

`Model::avatar` (準拠レベル・メタデータ・トラッキングの対応の上書き・アプリ向けの情報) と拡張パラメータを追加し、`ugoe validate --profile avatar` で準拠を検査する。

## 受け入れ条件

- [x] `docs/spec/avatar-profile.md` のメタデータと拡張パラメータを型として定義する (許諾の既定値は最も制限の強い値)
- [x] `ugoe_model::standard` に拡張パラメータを追加する
- [x] 準拠レベルごとに必要なパラメータとメタデータを検査する関数と、`ugoe validate --profile avatar` がある
- [x] 機能 `avatar.profile` を宣言する
- [x] 仕様書の「実装待ち」を更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0012-avatar-profile.md`
- `docs/spec/avatar-profile.md`

## メモ

### やったこと

- `ugoe_model::avatar` (新しいモジュール) に `AvatarProfile` / `AvatarMeta` / 許諾の enum 4 つ / `TrackingOverride` / `TrackingSource` / `LookAt` / `AvatarHitArea` を定義し、`Model::avatar: Option<AvatarProfile>` を末尾に追加した (`#[serde(default)]`、`None` は書き出さない。`SCHEMA_VERSION` は変えていない)。
- 許諾の既定値は最も制限の強い値。許諾の enum と `TrackingSource` は `Unknown` を持ち、許諾の `Unknown` は `effective()` で最も制限の強い値になる。`unknown::in_model` が `avatar` の未知の値も返す。
- `ugoe_model::standard` に拡張パラメータ (`EXTENDED`、16 個。`EyeLSmile` / `EyeRSmile` は `ALL` と共通)、母音 (`VOWELS`)、値域を引く `standard::range` を追加した。
- 準拠の検査 `avatar::check::check_profile` (`ProfileReport { claimed, satisfied, issues }`、問題はエラーと警告に分かれる) と `ugoe validate --profile avatar` を追加した。`ugoe standard-params` は拡張パラメータと母音も表示する。
- フィクスチャ `avatar_profile` (レベル 3 を満たす) を追加し、CLI の `fixture:avatar-profile` でも使える。
- 機能 `avatar.profile` は `ugoe-format` の機能の一覧に基準の機能として既にあった。基準の機能は宣言しないので、`detect` は変えず、`avatar` を含むバンドルの往復と宣言が空であることのテストを足した。
- 仕様: `avatar-profile.md` (実装待ちの更新、トラッキングの上書きの意味、`look_at` のフィールド、「準拠の検査」の節)、`model.md` (`avatar` のフィールド・節、予定の表から削除、未知の値の enum)、`file-formats.md` (未知の値のフォールバック)、`glossary.md`。

### 下流のタスク向けのメモ

- T0306: 上書きの意味は `avatar-profile.md#トラッキングの対応の上書き` に決めた (上書きのあるパラメータには標準の対応で書かない、同じパラメータへの書き込みは足し合わせる、最後にクランプ)。使える定数: `avatar::TRACKED_PARAMETERS` (標準の対応表で値が決まる ID)、`ARKIT_BLEND_SHAPES` (52 個)、`HEAD_INPUTS`、`TrackingOverride::target()`。`look_at` が `None` なら `LookAt::default()` を使う。
- T0702: `ugoe validate --profile avatar <入力>` は「avatar profile: claims level N, satisfies level M」と `ok` を出す。エラーがあれば終了コードが 0 以外。

### 範囲外の変更

- `Model` にフィールドを足したので、`ugoe-editor-core/src/project.rs` の `Model` の構造体リテラルに `avatar: None` を 1 行足した (コンパイルに必要な最小限)。生成物 `apps/editor/src/lib/generated/types.ts` は `just gen` で更新した。

### やらなかったこと・後続の候補

- `avatar.slots` の ID の存在の検査は、スロット (T0111) の実装後に行う (`TODO(T0111)`)。
- 構造の検証 (`validate::validate`) は `avatar` を検査しない (ランタイムが読み飛ばせるデータなので、読み込みを拒否する理由にしない)。
- エディタでメタデータやトラッキングの上書きを編集する UI と `Command` は未着手 (後続のタスクの候補)。
- `look_at` の可動範囲の既定値 (30 度) と、標準・拡張パラメータの値域が仕様と違うときを警告にした判断は、実際のアプリで使って見直す。
