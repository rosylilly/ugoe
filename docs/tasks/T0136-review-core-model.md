---
id: T0136
title: コードレビュー: ugoe-core と ugoe-model
status: done
milestone: M9
crates: [ugoe-core, ugoe-model, docs]
depends_on: []
assignee: claude-worker-T0136
---

# T0136: コードレビュー: ugoe-core と ugoe-model

## 目的

ugoe-core と ugoe-model のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-core` と `crates/ugoe-model` のすべて (フィクスチャ・検証・シリアライズを含む)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - `ugoe-model` がデータだけを持っているか (補間・変形のロジックが紛れていないか)
  - 型と `docs/spec/model.md`・英語の公開仕様 (`spec/`) の一致、`SCHEMA_VERSION` と前方互換 (ADR 0013) の扱い
  - 検証 (validation) の漏れ: 参照の切れ・循環・範囲外の値が検証を通らないか
  - 公開 API の変更が要る所見は、直さずにインターフェース先行のタスクとして起票する
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0140〜T0144 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
- 起票: T0140・T0141・T0142 (T0143・T0144 は使わなかった)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `ugoe-model/src/keyform.rs` `KeyformGrid::expected_len`、`validate.rs` のワープの制御点数 | 軸のキー数の積があふれると、debug ではパニックし、release では小さい値に巻き戻る。2 キーの軸 64 本と空の `keyforms` の格子が検証を通り、ランタイムの添字で落ちる (細工したファイルでクラッシュ)。ワープの `(rows+1)*(cols+1)` も 32 ビットの `usize` (wasm32) であふれる | 直した (積を飽和させて `usize::MAX` にし、どの長さとも一致しないようにした。失敗するテスト `keyform_count_overflow_is_reported` を先に追加) |
| 中 | `ugoe-model/src/validate.rs` `check_grid` | キーが 1 つの軸の NaN を見逃す (隣と比べるだけなので)。仕様 (`docs/spec/model.md#検証`) は「NaN を含まないこと」 | 直した (失敗するテスト `single_nan_key_is_rejected` を先に追加) |
| 中 | `ugoe-model/src/physics.rs` `VertexOwner` | 外部タグの enum で `Unknown` が無い。新しい版で頂点への出力の対象が増えると、物理の設定全体が読めない (ADR 0013 の前方互換の穴) | T0140 を起票 (公開 API の変更) |
| 中 | `ugoe-model/src/validate.rs`・`validate/physics.rs` | 数値の有限性 (頂点・不透明度・回転・`uvs`・無限大のキー・グルー・呼吸) と、物理の `fps`・出力の `particle` の範囲・粒子 0 個・正規化の範囲を検証しない。ランタイムは黙って無視するか NaN を描画まで流す | T0141 を起票 (読めていたファイルを拒むかの判断が要る) |
| 中 | `ugoe-model/src/fixtures.rs` のテスト | `smooth_arc`・`physics_relay`・`physics_additive`・`avatar_profile` が `validate` を、`physics_relay_settings`・`physics_additive_settings` が `validate_physics` を通るかを確かめていない | 直した (テストに加えた) |
| 低 | `ugoe-model/src/validate.rs` の `exists` | `mirror_source` の参照先の確認が ID の表の線形探索 | 直した (`contains_key`) |
| 低 | `ugoe-core/Cargo.toml` | 使っていない `thiserror` への依存と、説明の「エラー」(エラー型は無い) | 直した |
| 低 | `ugoe-model/src/keyform.rs` の `KeyformGrid` の doc | 「行優先 (column-major 的)」が矛盾している (軸 0 が最も速いのは列優先) | 直した |
| 低 | `ugoe-model/src/unknown.rs` `check_grid` | 「場所をすべて返す」とあるが、`origins` の未知の由来は格子ごとに最初の 1 つだけ返す | doc を直した (セルの数だけ警告が並ばないための意図した挙動) |
| 低 | `HitArea`・`Glue`・`PhysicsRig`・`PhysicsCollider` の `id`、`AvatarProfile::slots` などの参照 | ID と参照が `Id<T>` でなく `String` のものがある | T0142 を起票 (公開 API の変更) |
| 低 | `ugoe-model/src/validate/module.rs` | `validate_module` はモジュールのポーズの `links` (モジュールのパーツ) を検査しない | 見送り (合成が名前を付け替えてから `validate` するので、そこで報告される。`docs/spec/modules.md#検証` どおり) |
| 低 | `ugoe-model/tests/spec_files.rs` | 生成物の一覧から外れたファイル (消した例など) が `spec/` に残っても検出しない | 見送り (`just gen` の運用で足り、例を消すことはまれ) |

確かめたこと (問題なし):

- `ugoe-model` はデータだけを持つ。ロジックらしいものは `KeyformGrid::derivation` / `origin` / `blend_shape_index`・`Module::referenced_parameters`・各 enum の `effective()`・`TrackingOverride::target` だけで、どれもデータの読み方 (フォールバックや参照の列挙) であり、補間・変形・再生は無い。検証 (`validate`・`avatar::check`・`unknown`) はこのクレートに置く決まり。
- 型と `docs/spec/model.md` の一致: フィールドと既定値・`skip_serializing_if`・未知の値の一覧 (`VertexOwner` を除く) は仕様どおり。英語の公開仕様のフィールドの表は `just spec` (`just ci` に含む) が検査する。
- `SCHEMA_VERSION` は 0 のまま。追加したフィールドはすべて `#[serde(default)]` で、未知のフィールドを拒まない (`deny_unknown_fields` は無い)。未知のタグは `Unknown` として読み、書き出しはエラーになる (テストあり)。
- 検証: 参照の切れ (全種類)・親子とマスクの循環・ポーズの重複・スロット・物理の参照と種類はすべて検査とテストがある。検証の中の添字 (`model.meshes[i]` など) はどれも同じ列から作った表の値で、範囲外にならない。
- `unsafe`・`unwrap()` (ライブラリのコード)・`todo!()` は無い。`ugoe-core` は依存がほぼ無い状態を保っている。

### 横断 (T0008 向け)

- 長さの積のあふれ: `ugoe-runtime/src/keyform.rs` の `stride *= binding.keys.len()` と `ugoe-rigging/src/grid.rs` の `stride` も検査なしで掛ける。ランタイムはコンパイル前に検証するので今回の修正で塞がるが、`ugoe-rigging` は格子の大きさを確かめる前に `stride` を計算するので、検証していない格子では debug でパニックしうる。外から来た数を掛ける箇所は `checked_mul` / `saturating_mul` にそろえるとよい。
- 検証とランタイムのフォールバックの分担がそろっていない: 物理の `fps`・`particle` のようにランタイムだけが黙って直す値と、`mask_strength` のように検証が有限性を見てランタイムがクランプする値が混ざっている (T0141)。
- 「最初の 1 つだけ報告する」の扱い (深度・重み・未知の由来・インデックスの範囲外) が要素ごとに決まっていて、どれがそうかは doc を読まないと分からない。
- ID の型 (`Id<T>` と `String`) が要素によって違う (T0142)。

### `just ci`

- 1 回目は `apps/editor/src/lib/generated/types.ts is stale` で失敗した (`KeyformGrid` の doc コメントの直しが生成した型と `crates/ugoe-editor-core/schema/command.schema.json` の説明に入るため)。
  `just gen` で作り直してコミットした (範囲の外のファイルだが、生成物の追従だけ)。
- 2 回目の `just ci` と `just wasm-check` は通った。

