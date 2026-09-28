# ugoe-format

**層:** L2 入出力 (依存してよいクレートは `scripts/check-layers.py` を参照)

`.ugoe` バンドルと JSON 形式の読み書き。

## 守ること

- 読み込み時には必ずスキーマのバージョンを検査する
- 壊れた入力でパニックしない
- ファイルシステムに触れない (バイト列と `Read`/`Write` を扱う)

## 仕様

- [docs/spec/file-formats.md](../../docs/spec/file-formats.md)

## 関連タスク

- T0105 (`docs/tasks/T0105-*.md`)
