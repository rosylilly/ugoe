---
id: T0174
title: "CLI: ugoe build がモデル・モジュールの JSON からバンドル・モジュールを書けるようにする"
status: done
milestone: M9
crates: [ugoe-cli, docs, spec]
depends_on: []
assignee: claude-worker-T0174
---

# T0174: CLI: ugoe build がモデル・モジュールの JSON からバンドル・モジュールを書けるようにする

## 目的

`ugoe build` の入力はプロジェクトのディレクトリ・バンドル・`.ugoemod` だけで、手で書いたモデル・モジュールの JSON (`*.model.json`・
`*.module.json`) からバンドルやモジュールを作る道具が無い。T0472 では Unity のテストの資料 (`unknown-mask.ugoemod`) を作るために
Rust のテスト (`crates/ugoe-format/tests/unity_fixtures.rs`) を書いた。spec/examples/bundles の `short-hair.ugoemod` も JSON から作る道具が無い (T0472 の候補)。

## 受け入れ条件

- [x] `ugoe build <file>.model.json -o <out>.ugoe` と `ugoe build <file>.module.json -o <out>.ugoemod` が書ける (読み込みの検証を通すこと。
      壊れた JSON・検証の失敗は今の `build` と同じ形のエラー)
- [x] テクスチャ・法線マップの画像の置き場 (JSON の隣の名前で探す・画像なし) を決め、docs (crates/ugoe-cli/README.md と docs/spec/file-formats.md の
      該当の節があればそこ) に書いた
- [x] テストで、JSON から書いたバンドル・モジュールを読み直すとモデル・モジュールが同じになることと、`--atlas` との組み合わせ (使えないなら分かるエラー) を確かめる
- [x] T0472 の `unity_fixtures.rs` の作り直しや spec/examples の資料をこのコマンドで作れることを確かめ、作り方を書いた文書 (bindings/unity/tests/Fixtures/README.md など) を
      直すかをメモに書いた (範囲の外の文書は 1 行の直しだけ)
- [x] `just ci` が通る

## 参考

- T0472 のメモ (起票の候補)
- crates/ugoe-cli/src/main.rs の `run_build`・`rebuild_module`

## メモ

- T0038 で起票した (元の候補: T0472)。
- やったこと (T0174):
  - `ugoe build` が、先頭のマジックが `UGOE` でないファイルを JSON として読むようにした (`crates/ugoe-cli/src/json_source.rs`)。
    最上位に `slot` があればモジュール、なければモデル (拡張子は見ない。コンテナを中身で判別するのと揃えた)。読んだ後に
    `validate` / `validate_module` を通し、失敗はバンドルと同じ形 (`error: ...` を 1 行ずつ、最後に `N validation error(s)`) にした
    (`read_container` のエラーの書き出しを `validation_failure` に切り出して共有)。壊れた JSON は `parsing ...`。
  - モデルの JSON は `Project` にして既存の書き出し (アトラスを含む) に流し、モジュールの JSON は `ModuleFile` にして `rebuild_module` に流す。
    `--atlas`・`--compress` はバンドル・モジュールの入力と同じに使える (法線マップのあるテクスチャは今までどおり警告して残す)。
  - **画像の置き場 (決めたこと):** プロジェクトと同じく JSON の隣の `textures/<name>.png` (テクスチャと法線マップ)。通常のファイルだけを
    プロジェクトの画像の上限 (`ProjectReadLimits::default().max_image`) まで読み、PNG でなければエラー。見つからない画像 (名前がファイル名に
    使えないものも) は 1x1 で埋めて `warning: ... using a 1x1 placeholder` を書く (テクスチャは不透明な白 = `ugoe render` が JSON に使うもの、
    法線マップは `(128, 128, 255)` = 正面向き)。`crates/ugoe-cli/README.md` の `ugoe build` と `docs/spec/file-formats.md` のプロジェクトの節に書いた
    (プロジェクトは公開仕様に含めないので `spec/` は直していない)。
  - 入力の読み込みは T0530 の線引きに合わせ、`build` の入力 (JSON・バンドル・モジュール) は `fs::read` のまま FIFO も読める (`/dev/stdin` で確かめた)。
    CLI の README の「使い方」に、利用者が渡す入力は FIFO も読み、補助のファイル (`--texture` など) と PSD は通常のファイルだけ、と書いた。
  - テスト (`tests/cli.rs`): `build_writes_a_bundle_from_model_json` (置いた画像と埋めた画像・法線マップの並び・読み直すと同じモデル・`--atlas`)、
    `build_writes_a_module_from_module_json` (Unity の `unknown-mask` と公開仕様の `short-hair` の JSON から、読み直すと同じモジュール・`--atlas --compress`)、
    `build_rejects_broken_or_invalid_json` (壊れた JSON・検証の失敗・PNG でない画像)。
- 資料の作り直しについて:
  - `unknown-mask.ugoemod` は `just cli build bindings/unity/tests/Fixtures/unknown-mask.module.json -o ...` で同じモジュール (同じ 724 バイト。
    PNG のエンコードの違いで中身のバイトは違う) を書ける。`bindings/unity/tests/Fixtures/README.md` の行に 1 行で併記した。
    `crates/ugoe-format/tests/unity_fixtures.rs` (範囲外) は、置いたファイルが JSON と同じかを `just ci` で確かめる役があるので残した
    (作り直しの経路 `UGOE_UPDATE_FIXTURES` を消すかは ugoe-format の側の判断)。
  - `spec/examples/bundles/short-hair.ugoemod` などの公開仕様の例は `tools/spec-check/src/make-bundles.ts` が目録 `bundles.json` から作る
    (`META`・画像の色・`style` も目録で決まる)。`ugoe build` は同じモジュールは書けるが `META` は付けられないので、例の作り方は変えていない
    (T0472 の候補の「作る道具が無い」は make-bundles.ts で足りている)。
- やらなかったこと: モデルの JSON に物理・モーション・表情・`META` を添える手段 (`--physics` など) は入れていない。
- 起票の候補:
  - `ugoe build` に `--meta <json>` (と、モデルの JSON に `--physics`・`--motion`・`--expression`) を足すと、`META` や物理を持つ資料も CLI だけで作れる。
  - `crates/ugoe-format/tests/unity_fixtures.rs` の作り直しの経路を `ugoe build` に寄せる (テストは照合だけにする) か決める。
