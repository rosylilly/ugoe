# Unity のバインディングの検査で使うファイル

| ファイル | 作り方 | 使うテスト |
| --- | --- | --- |
| `short-hair-zstd.ugoemod` | `just cli build spec/examples/bundles/short-hair.ugoemod --compress -o bindings/unity/tests/Fixtures/short-hair-zstd.ugoemod` (チャンクを zstd で圧縮したモジュール。読み込みの上限は圧縮したチャンクを展開したバイトだけを数えるため) | `LimitsTests.cs` |
