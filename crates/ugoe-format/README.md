# ugoe-format

**層:** L2 入出力 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

`.ugoe` バンドル・`.ugoemod` (着せ替えのモジュール。同じコンテナ) と JSON 形式の読み書き。

## 守ること

- 読み込み時には必ずスキーマのバージョンを検査する
- 壊れた入力でパニックしない
- ファイルシステムに触れない (バイト列と `Read`/`Write` を扱う)
- 知らない必須の機能は拒否し、未知の値 (`Unknown`) は書き出さない (`features` モジュール)

## 仕様

- [docs/spec/file-formats.md](../../docs/spec/file-formats.md)

## 関連タスク

- T0105 (`docs/tasks/T0105-*.md`)
- T0109 (前方互換: 機能の宣言と未知の値)
- T0113 (着せ替えのモジュールのファイル形式)
- T0115 (互換性テストの資料 `assets/compat/` と `tests/compat.rs`。手順は `docs/testing.md#互換性テスト`)
