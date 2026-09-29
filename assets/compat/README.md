# 互換性テストの資料

`.ugoe` と `.ugoemod` の互換性テスト (`crates/ugoe-format/tests/compat.rs`) が読むファイルです。
ディレクトリの意味・名前の付け方・作り直しと追加の手順は [docs/testing.md#互換性テスト](../../docs/testing.md#互換性テスト) にあります。

- `dev/`: 今のコードの書き出しで作ったもの。`UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test compat` で作り直す
- `<版>/` (例 `1.0.0/`): リリースの時点の `dev/` の写し。作り直さない、書き換えない (1.0 より前はない)
- `future/`: 新しい版のファイルを模したもの (知らない機能・値・フィールド・チャンクを含む)。`dev/` と同じ手順で作り直す

どのファイルも `ugoe_model::fixtures` から作る ugoe の自作で、ライセンスは MIT です。
