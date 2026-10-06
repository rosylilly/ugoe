---
id: T0176
title: "CLI: ugoe build に META と物理・モーション・表情の JSON を添える口を足し、Unity の資料の作り直しを寄せる"
status: done
milestone: M9
crates: [ugoe-cli, ugoe-format, bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0176
---

# T0176: CLI: ugoe build に META と物理・モーション・表情の JSON を添える口を足し、Unity の資料の作り直しを寄せる

## 目的

T0174 で `ugoe build` がモデル・モジュールの JSON からバンドルを書けるようになったが、`META` (アバターのメタデータ) と、モデルの JSON に添える物理・モーション・表情は
書けない (T0174 の候補)。また、Unity の検査の資料 (`bindings/unity/tests/Fixtures/`) は `crates/ugoe-format/tests/unity_fixtures.rs` が
`UGOE_UPDATE_FIXTURES=1` で作り直し、同じテストが照合もしている。作り直しの経路を `ugoe build` に寄せ、テストは照合だけにするかを決める (T0174 の候補)。

## 受け入れ条件

- [x] `ugoe build` に `--meta <json>` と、モデルの JSON の入力に `--physics <json>`・`--motion <json>` (複数)・`--expression <json>` (複数) がある
      (名前と形は既存の `ugoe` のサブコマンドの引数に揃える。読み込みは T0530 の線引きどおり補助のファイルは通常のファイルだけ)。`tests/cli.rs` で、
      書いたバンドルを読み直すと添えたものが入っていることと、壊れた JSON・検証の失敗がエラーになることを確かめる
- [x] Unity の資料の作り直しを決めた: `ugoe build` だけで今の資料と同じものを作れるなら、`unity_fixtures.rs` の作り直しの経路 (`UGOE_UPDATE_FIXTURES`) を外して
      照合だけにし、作り方を `bindings/unity/tests/Fixtures/README.md` に `just cli build ...` で書く。作れないものが残るなら、理由をメモに書いて今の経路を残す
- [x] crates/ugoe-cli/README.md の `ugoe build` を直した
- [x] `just ci` が通る (Unity の資料を変えたら `just unity-check` も)

## 参考

- T0174 のメモ (資料の作り直しについて・起票の候補)
- crates/ugoe-cli/src/json_source.rs、crates/ugoe-format/tests/unity_fixtures.rs、bindings/unity/tests/Fixtures/README.md

## メモ

- T0040 で起票した (元の候補: T0174 の 2 つ)。
- やったこと (T0176):
  - `ugoe build` に `--meta <PATH>`・`--physics <PATH>`・`--motion <PATH>` (複数)・`--expression <PATH>` (複数) を足した
    (`render` / `validate` の `--physics`・`--motion`・`--expression` と同じ名前。`build` はパスだけを受ける)。
    - `--meta` はどの入力にも使え、自由な JSON を `META` に書く (入力のバンドル・モジュールの `META` の代わりにする)。
    - `--physics`・`--motion`・`--expression` はモデルの JSON の入力だけ。ほかの入力 (プロジェクト・バンドル・モジュール・モジュールの JSON) では
      `need a model JSON input` で失敗する (それらは物理などを自分で持つか持てないので、足すのか差し替えるのかを決めずに済むように)。
    - 物理は `validate_physics` でモデルとの参照を、モーション・表情は `motion_from_json` / `expression_from_json` の数値の検証を通し、
      失敗はバンドルと同じ形 (`error: ...` と件数。モーション・表情は名前を前に付ける) にした。壊れた JSON は `parsing ...`。
    - 補助のファイルなので T0530 の線引きどおり通常のファイルだけを、チャンクの上限 (`MAX_DECOMPRESSED_CHUNK_LEN`) まで読む
      (`json_source::read_json`)。
    - テスト (`tests/cli.rs`): `build_adds_meta_and_animation_files_to_model_json` (公開仕様の例から、読み直すと物理・並べた順のモーションと表情・`META` が入る。
      バンドルとモジュールの入力の `--meta` の差し替え)、`build_rejects_broken_or_invalid_added_files` (4 つの壊れた JSON・物理の参照・モーション・表情の検証・
      ディレクトリを渡した `--meta`・モジュールの入力への `--motion`)。
  - **Unity の資料 (決めたこと):** `ugoe build` に寄せた。`unknown-mask.ugoemod` は `just cli build bindings/unity/tests/Fixtures/unknown-mask.module.json -o ...` で
    今のものと同じモジュール (同じ 724 バイト。1×1 の白の PNG のエンコードだけが違う) を書けるので、作れないものは残っていない。
    `crates/ugoe-format/tests/unity_fixtures.rs` から `UGOE_UPDATE_FIXTURES` の経路と埋め込みの PNG を外して照合だけにし (テクスチャの数の照合を足した)、
    `bindings/unity/tests/Fixtures/README.md` の作り方を `just cli build ...` だけにした。理由: 作る道具が CLI の 1 つになり、テストが資料を書き換える経路
    (環境変数を立てると照合を飛ばして通る) が無くなる。置いた `.ugoemod` は書き直していない (中身のモジュールは同じなので `just unity-check` は流していない)。
  - `crates/ugoe-cli/README.md` の `ugoe build` (例・オプションの表・添えるものの説明・補助のファイルの読み方) を直した。
  - `just ci` は通った。
- やらなかったこと: 公開仕様の例 (`spec/examples/bundles/`) の作り方は `tools/spec-check/src/make-bundles.ts` のまま (画像の色や `style` も目録で決まるので `ugoe build` には寄せない)。
- 起票の候補: なし。
