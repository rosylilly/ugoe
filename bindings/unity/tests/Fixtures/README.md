# Unity のバインディングの検査で使うファイル

| ファイル | 作り方 | 使うテスト |
| --- | --- | --- |
| `short-hair-zstd.ugoemod` | `just cli build spec/examples/bundles/short-hair.ugoemod --compress -o bindings/unity/tests/Fixtures/short-hair-zstd.ugoemod` (チャンクを zstd で圧縮したモジュール。読み込みの上限は圧縮したチャンクを展開したバイトだけを数えるため) | `LimitsTests.cs` |
| `mask-base.model.json` | 手で書いた本体のモデル JSON (`Puppet.FromJson` で読む)。スロット `Hair` がマスクに使わせるのは `Face` だけ | `ResultTests.cs` |
| `unknown-mask.module.json` | 手で書いたモジュール JSON。`unknown-mask.ugoemod` の元 (メッシュ `Back` が本体の `Skin` でマスクする) | (`unknown-mask.ugoemod` の元) |
| `unknown-mask.ugoemod` | `just cli build bindings/unity/tests/Fixtures/unknown-mask.module.json -o bindings/unity/tests/Fixtures/unknown-mask.ugoemod` (`textures/` を置かないので、テクスチャは 1×1 の白で埋まる。その警告が出る)。`mask-base.model.json` に付けると `unknown_mask` | `ResultTests.cs` |

`ugoe build` で作るファイルは、`crates/ugoe-format/tests/unity_fixtures.rs` (`just ci` に含まれる) が、元の JSON と同じモジュールであることと、
`unknown-mask.ugoemod` を `mask-base.model.json` に付けると `unknown_mask` になることを確かめる (照合だけで、作り直しはしない)。JSON を直したら上のコマンドで作り直す。
