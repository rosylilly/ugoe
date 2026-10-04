---
id: T0528
title: コードレビュー: ugoe-ai (自動リギング)
status: done
milestone: M9
crates: [ugoe-ai, docs]
depends_on: []
assignee: claude-worker-T0528
---

# T0528: コードレビュー: ugoe-ai (自動リギング)

## 目的

ugoe-ai (自動リギング) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-ai` のレイヤーの分類・自動リグ・モジュールのリグ・法線マップ (プロバイダと通信は T0527)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 規則による分類と AI の分類の食い違いの扱い、提案の検証
  - rig.rs・module_rig.rs の大きな関数の分け方と、ugoe-rigging・ugoe-editor-core との重複
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0546〜T0550 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 読んだ範囲

`src/classify.rs`・`src/classify/{rules,image}.rs`・`src/rig.rs`・`src/turn.rs` (自動リグが使う振り向き)・`src/module_rig.rs`・
`src/module_rig/{mesh,pose}.rs`・`src/normal.rs` と、テスト `tests/{classify,auto_rig,depth_turn,module_rig,module_rig_sample,normal,normal_render,m6_acceptance}.rs`・
`tests/common/mod.rs`。プロバイダと通信 (`claude.rs`・`openai.rs`・`transport.rs`・`depth.rs` の HTTP・`fake.rs`) は T0527。

### 確かめたこと

- **規則と AI の分類の食い違い:** AI が答えたレイヤーは答えで置き換え、答えのないレイヤーだけ名前の規則 (作り直しなら前の分類) で補う
  (仕様どおり)。答えの検証 (スキーマ・番号の範囲・2 度の答え・差分の組の重なり・`confidence` の丸めと NaN) はある。ただし名前の規則が
  確からしく当たったレイヤーを AI が別のラベル・逆の左右で上書きしても、確認に回らない (T0546)。
- **提案の検証:** リグ・モジュールのリグ・振り向き・法線マップはすべて複製したドキュメントに適用し `validate` を通してから返す。
  モジュールは `split_module` も確かめる。AI の値は範囲の外を丸めずに問い直す。テンプレートの内部の失敗も問い直してしまう (T0547)。
- **大きな関数と重複:** `Layout::new`・`ModuleLayout::new`・`rig_group` は長い (T0549)。リグとモジュールのリグの問い直しのループ・範囲の検査・
  スキーマ・作り直しの規則は重複していたので `rig.rs` の `pub(crate)` の関数に寄せた。`ugoe-rigging` との重複はない (深度からの回転・
  ふくらみ・`ParentWorld` は呼んでいる)。`ugoe-editor-core` とは、キーフォームの格子の補助 (`Rig::keyform_at` など) が近い (T0549)。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/module_rig/pose.rs` (`named_variants`) | 差分の名前の末尾に全角の空白などがあると (`badge A　`)、名前をバイトの位置で切って panic する。ASCII の空白なら示す名前が `badge A` になる | 直した (前後の空白を除いてから切る。テスト `variant_names_with_trailing_spaces`) |
| 中 | `src/classify/rules.rs` | 髪のハイライト (`前髪ハイライト`・`back hair hl`・`hair highlight`) が `eye_highlight` になり、自動リグで目の開閉に入る | 直した (髪の規則を目・口の規則より先に見る。テスト `hair_highlights_stay_hair`) |
| 中 | `src/classify.rs` (`classify_layers_guided`) | 名前の規則と AI の分類の食い違い (ラベル・左右) を黙って上書きし、確認に回らない | T0546 を起票 |
| 中 | `src/rig.rs` (`ask_until_valid`、もとは `propose_rig_with_ai_guided`・`propose_module_rig_with_ai_guided`) | テンプレートの内部の失敗も `InvalidProposal` なので AI に送って問い直す。モジュールで AI が `sway_scale = 0` を答えて組むものがなくなると問い直さず失敗。`max_attempts = 0` | T0547 を起票 |
| 中 | `src/module_rig/mesh.rs` (`Poser::mesh`) | 本体のメッシュへの追従を探すとき、衣装のメッシュ × 本体のメッシュの数だけ基準の姿を評価し直す | 直した (基準の姿は 1 回だけ評価して使い回す) |
| 低 | `src/classify/image.rs` (`thumbnail`) | 画素の数を u32 で掛け、一辺 60000 まで許すレイヤーで debug ではあふれて panic、release では回り込む | 直した (usize で数え、画素の長さを先に確かめる。テスト) |
| 低 | `src/rig.rs`・`src/module_rig.rs` | 問い直しのループ・範囲の検査・答えのスキーマ・作り直しの規則・捨てた答えの書き方が重複 | 直した (`ask_until_valid`・`range_errors`・`values_schema`・`revision_rules`・`push_rejected` に寄せた。送る文は同じ) |
| 低 | `src/classify/rules.rs` | 部位の語のないハイライト (顔・頬)・首のリボン・`hair top` の取り違え | T0548 を起票 |
| 低 | `src/classify/rules.rs` | 横髪の「触角」がない (「触覚」だけ) | 直した |
| 低 | `src/rig.rs` (`Layout::new`)・`src/module_rig.rs` (`ModuleLayout::new`・`rig_group`) | 長い関数 | T0549 を起票 |
| 低 | `src/rig.rs` (`Rig::keyform_at`・`has_key`)・`src/turn.rs` (`warp_keys`・`Builder::add_key`・`new_cells`) | 格子の補助の重複。キーの比べ方が `==` と 1e-5 で違う。`new_cells` は空の軸で添字の外を読む (検証済みのモデルでは起きない) | T0549 を起票 |
| 低 | `tests/depth_turn.rs` | 同じ名前のメッシュが 2 つあるときの `mesh_parts` のテストがない | 直した (テストを足した) |
| 低 | `tests/classify.rs` (`names_classify_the_bustup_psd`) | 録画を使わないテストに録画のコメント | 直した |
| 低 | `src/classify.rs` (`merge_answer`) | 差分の組 (`variant_sets`) は 1 回のリクエスト (既定 32 枚) の中でしか作れず、前の組がリクエストをまたぐと片方だけ残る | 見送り (既定の枚数では 1 回に収まり、仕様書にある振る舞い) |
| 低 | `src/module_rig/pose.rs` (`drawn_variants`) | 6 つ以上の差分は A〜E の 5 つまでをポーズに入れ、残りは隠したまま (`hinted_variants` は組ごと使わない) | 見送り (文字の数の制約。実害が小さい) |
| 低 | `src/turn.rs` (`in_triangle`) | 3 点が重なった退化した三角形は、どの点も覆うとみなす (`ugoe-runtime` の `hit.rs` と同じ書き方) | 見送り (取り込みのメッシュでは起きない。横断に書いた) |
| 低 | `src/rig.rs` (`mesh_parts_where`) | 同じ名前の数え方が O(n²) | 見送り (提案ごとに 1 回で、メッシュは数百まで) |

### 横断 (T0008 向け)

- **外から来る名前をバイトの位置で切る:** `&name[..name.len() - 1]` のような切り方は、PSD のレイヤー名など外から来る文字列では
  文字の境界を確かめる (`trim` は全角の空白も除くので、`trim` の後の長さで切ると境界がずれる)。
- **画素の数を u32 で掛ける:** `width * height * 4` を u32 のまま計算するところは、大きな入力で debug ではあふれる。usize で数える。
  `ugoe-psd`・`ugoe-mesher`・`ugoe-rigging`・`ugoe-render`・エディタのテクスチャの処理にもありそう。
- **点と三角形の判定・重心座標が 4 か所にある:** `ugoe-runtime/src/hit.rs`・`ugoe-ai/src/turn.rs` (`in_triangle`)・
  `ugoe-ai/src/module_rig/mesh.rs` (`barycentric`)・`ugoe-editor-core/src/mesh_edit.rs` (`barycentric`、f64)。退化した三角形の扱いと
  許容誤差がそろっていない。`ugoe-core` に寄せる候補。
- **エラーの種類が「誰の誤りか」を分けない:** `AiError::InvalidProposal` を AI の答えの誤りとテンプレートの内部の失敗の両方に使っている
  (T0547)。ほかのクレートでも、利用者の入力・外部・内部の誤りでエラーを分けているか見る。
- **同じ基準の姿を何度も評価する:** `rig::mesh_bounds`・`turn::Scene::evaluate`・`normal::evaluate`・`module_rig::mesh::Poser` が
  それぞれ `CompiledModel::compile` と `update` をする。1 回の提案の中で共有できるところがある (性能の目標に効くのはエディタの操作の応答)。

### just ci

1 回目は `ugoe-studio` の `api::tests::turning_the_api_on_saves_the_choice_and_listens` が `Address already in use (os error 98)` で落ちた
(決まったポートを使うテストで、並行するワーカーのテストと重なった。このタスクは `ugoe-studio` を触っていない)。単独で流すと通り、
流し直した `just ci` は通った。
