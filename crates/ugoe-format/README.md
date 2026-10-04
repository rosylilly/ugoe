# ugoe-format

**層:** L2 入出力 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

`.ugoe` バンドル・`.ugoemod` (着せ替えのモジュール。同じコンテナ) と JSON 形式の読み書き。

## 守ること

- 読み込み時には必ずスキーマのバージョンを検査する
- 壊れた入力でパニックしない
- 外から来る入力で資源を使い切らない。展開の上限は形式でなく読む側の設定 (`ReadLimits`。既定は 1 チャンク 256 MiB・合計 1 GiB)。超えたら展開の途中でやめてエラーにする
- ファイルシステムに触れない (バイト列と `Read`/`Write` を扱う)
- 知らない必須の機能は拒否する (`features` モジュール)。未知の値 (`Unknown`) は警告にして読み、書き出さない (`ugoe_model::unknown`)

## 仕様

- [docs/spec/file-formats.md](../../docs/spec/file-formats.md)

## 関連タスク

- T0105 (`docs/tasks/T0105-*.md`)
- T0109 (前方互換: 機能の宣言と未知の値)
- T0113 (着せ替えのモジュールのファイル形式)
- T0115 (互換性テストの資料 `assets/compat/` と `tests/compat.rs`。手順は `docs/testing.md#互換性テスト`)
- T0150 (読み込みの上限 `ReadLimits`。`docs/spec/file-formats.md#読み込みの上限`)
