---
id: T0115
title: 互換性テストの資料と CI
status: done
milestone: M5
crates: [ugoe-format, assets/compat, CI]
depends_on: [T0109, T0112, T0113]
assignee: claude-worker-T0115
---

# T0115: 互換性テストの資料と CI

## 目的

各リリースのファイルを `assets/compat/` に保存し、新しい機能を含むファイルを古い読み込みルールで劣化させて読めることを CI で検査する。

## 受け入れ条件

- [x] 資料の作り方と追加の手順を `docs/testing.md` に書く
- [x] 未知の任意機能を含むファイルを、フォールバックを適用して読めるテストがある
- [x] CI で実行される
- [x] `just ci` が通る

## 参考

- `docs/adr/0013-forward-compatibility.md`

## メモ

- 資料: `assets/compat/dev/` (今の書き出しで作る。1.0 の基準の機能を含むフィクスチャごとに 1 ファイル、`.ugoe` 10 個と `.ugoemod` 2 個、
  それぞれ数 KB)、`assets/compat/<版>/` (リリースのときに `dev/` を写す。1.0 より前はない)、`assets/compat/future/`
  (知らない任意の機能・必須の機能・知らない enum の値・フィールド・チャンクを含む、新しい版を模したファイル。`.ugoe` と `.ugoemod` を 2 つずつ)。
  手順は `docs/testing.md#互換性テスト`、置き場所の説明は `assets/compat/README.md`。
- テスト: `crates/ugoe-format/tests/compat.rs`。`cargo test --workspace` に入るので、`just ci` と CI の `rust` ジョブ (`just test`) で実行される。
  `.github/workflows/ci.yml` は変えていない。
  - `corpus_is_up_to_date`: `dev/` と `future/` が今のコードで作ったものとバイト単位で一致する。作り直しは
    `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test compat` (ugoe-psd のフィクスチャと同じ環境変数)。
    **形式・フィクスチャ (`dress_up_base` などコーパスに使ったもの)・書き出しを変えるタスクは、このテストが失敗するので資料を作り直してコミットする必要がある。**
  - `every_release_loads_without_warnings`: `future/` 以外のすべての版のディレクトリのファイルを警告なしで読み、`CompiledModel` で評価する
    (モジュールは同じディレクトリの `dress-up-base.ugoe` に `compose` する)。
  - `unknown_optional_features_in_{bundles,modules}_fall_back`: 警告の一覧が正確で、フォールバック (デフォーマは恒等、合成は通常、
    マスクは和集合) の結果の評価が `dev/` の元のファイルと同じ。書き出しは `UnknownValues`。
  - `unknown_required_features_are_rejected`: 本体が今の版では読めない形でも、`UnsupportedFeatures(["test.future_required"])` で拒否する。
- `future/` のコンテナは、テストの中で仕様だけから組み立てる (`ugoe-format` の書き出しのコードを使わない)。
- `test.` で始まる機能 ID は試験用に予約する、と `docs/testing.md` に書いた (T0116 の worker に共有済み)。
- `ugoe-format` の dev-dependency に `ugoe-runtime` を足した (フォールバックを評価の結果で確かめるため。`check-layers` は dev の依存を見ない)。
- 範囲外の変更: `docs/testing.md` (受け入れ条件)、`crates/ugoe-format/README.md` の関連タスク。
- 確かめていないこと: モーション・物理・表情の未知の値の実行時のフォールバック (ugoe-motion / ugoe-physics) は、
  値が `Unknown` として読めることだけを確かめた。実行時の扱いはそれぞれのクレートのテストにある。
- 後続の候補: 1.0 のリリースの手順 (`docs/workflow/` など) に「`assets/compat/dev/` を `assets/compat/1.0.0/` に写す」を加える。
  `AGENTS.md` のリポジトリ構成に `assets/compat/` を加える。
