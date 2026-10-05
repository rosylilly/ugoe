# Unity のバインディングの検査で使うファイル

| ファイル | 作り方 | 使うテスト |
| --- | --- | --- |
| `short-hair-zstd.ugoemod` | `just cli build spec/examples/bundles/short-hair.ugoemod --compress -o bindings/unity/tests/Fixtures/short-hair-zstd.ugoemod` (チャンクを zstd で圧縮したモジュール。読み込みの上限は圧縮したチャンクを展開したバイトだけを数えるため) | `LimitsTests.cs` |
| `mask-base.model.json` | 手で書いた本体のモデル JSON (`Puppet.FromJson` で読む)。スロット `Hair` がマスクに使わせるのは `Face` だけ | `ResultTests.cs` |
| `unknown-mask.module.json` | 手で書いたモジュール JSON。`unknown-mask.ugoemod` の元 (メッシュ `Back` が本体の `Skin` でマスクする) | (`unknown-mask.ugoemod` の元) |
| `unknown-mask.ugoemod` | `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test unity_fixtures` (`unknown-mask.module.json` に 1×1 の白いテクスチャを付けて書く。`just cli build bindings/unity/tests/Fixtures/unknown-mask.module.json -o bindings/unity/tests/Fixtures/unknown-mask.ugoemod` でも同じモジュールを書ける)。`mask-base.model.json` に付けると `unknown_mask` | `ResultTests.cs` |

`unknown-mask.ugoemod` は、`crates/ugoe-format/tests/unity_fixtures.rs` (`just ci` に含まれる) が、JSON と同じモジュールであることと、
`mask-base.model.json` に付けると `unknown_mask` になることを確かめる。JSON を直したら上のコマンドで作り直す。
