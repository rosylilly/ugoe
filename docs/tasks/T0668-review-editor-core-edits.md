---
id: T0668
title: コードレビュー: ugoe-editor-core (編集の各モジュール)
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0668
---

# T0668: コードレビュー: ugoe-editor-core (編集の各モジュール)

## 目的

ugoe-editor-core (編集の各モジュール) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-editor-core` の deformer_edit・keyform・motion_edit・physics_edit・inference・import・atlas・grid・codegen・message_codes とそのテスト

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 各編集が `Command` として Undo できるか、検証の漏れ
  - message_codes とフロントエンドの翻訳のキーの対応 (ADR 0029)、codegen の出力と `generated/` の一致
  - ugoe-rigging・ugoe-runtime との重複
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0681〜T0684 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/motion_edit.rs` `insert_point` | 点のないカーブ (`segments: []`。読み込んだプロジェクトは検査しないので入りうる) に `SetMotionKey` を打つと `points[0]` の添字でパニックする | 直した (そのキーを `start` にする。`keying_a_loaded_curve_without_segments_starts_it`) |
| 高 | `src/document.rs` `UnwrapDeformer` | 外したデフォーマをミラー元 (`mirror_source`) にする格子が残り、検証 (`UnknownReference`) で拒否される。`RemoveDeformer` は `forget_missing_mirror_sources` を呼んでいた | 直した (`unwrap_forgets_the_deformer_as_a_mirror_source`) |
| 高 | `src/document.rs` `RemoveGlue` | 上と同じく、消したグルーをミラー元にするグルーの格子が残り拒否される | 直した (`removing_a_glue_forgets_it_as_a_mirror_source`)。document.rs は T0667 の範囲なので worker-T0667 に連絡済み |
| 中 | `src/deformer_edit.rs` `create` | `children` にデフォーマとその子孫 (例: `Hand` と `Palm`) を一緒に指定すると、子孫を新しいデフォーマへ直接付け替え、間のデフォーマ (`Hand`) の変形に追従しなくなる。パーツから選ぶ子 (`part_children`) はこれを避けていた | 直した (指定したデフォーマの下にある子は付け替えない。`create_warp_keeps_children_given_with_their_parent_deformer`、`docs/editor.md#デフォーマの編集` に「子の中の子」を追記) |
| 中 | `message_codes.rs`・`apps/editor/locales/` | コードと翻訳のカタログ (`errors.<code>`) の対応・引数の名前の一致を検査するものがない (docs の一覧だけ)。いまは手作業の照合ですべて一致 | T0681 を起票 |
| 低 | `src/document.rs` `SetMeshKeyform` | `SetKeyformCell` (メッシュ) と同じ処理を別に書いていて、頂点数の検査 (`CellError::ShapeMismatch`) がない (検証では拒否されるがエラーが分かりにくい) | 見送り (document.rs・command.rs は T0667 の範囲。`keyform::set_cell` に寄せるのがよい) |
| 低 | `src/physics_edit.rs` `check_particles` 等 | 値の検査の一部 (有限・非負) が `validate_physics` と重複する | 見送り (`mobility` の範囲など `validate_physics` に無い検査もあり、先に拒否するとエラーが分かりやすい) |
| 低 | `src/inference.rs` `mark` | すべて `authored` に戻しても `origins` を空に戻さない (`set_origins` は戻す)。意味は同じ | 見送り (振る舞いは変わらない) |
| 低 | `src/keyform.rs` `derive_combinations`・`src/inference.rs` `mirror` | モデル全体を複製して推定する | 見送り (編集の 1 回ごとの処理で、毎フレームではない) |
| 低 | `src/message_codes.rs` `UnsupportedInterpolation` | 引数 `interpolation` が `Debug` の表記 (`Smooth`) で、JSON の名前 (`smooth`) と違う | 見送り (引数の値の変更は画面の文に響くので T0681 と合わせて扱う) |
| 低 | `src/codegen.rs` `slot_samples` ほか | ライブラリのコードで `expect` と添字 (フィクスチャの形を前提) | 見送り (生成と型の検査の見本だけに使い、フィクスチャが変わればテストで分かる) |
| 低 | `src/atlas.rs` `shelf_pack` | `x + w` を `u32` で足す (`max_size` が 2^31 を超えるとあふれうる) | 見送り (書き出しの設定は `MAX_ATLAS_SIZE` で上限を検査済み) |

確かめたこと:

- Undo: どの編集も `Document::apply_with_report` が複製に適用して成功したときだけ履歴に積むので、範囲のコマンドはすべて 1 つの Undo の単位。範囲の各テストは Undo/Redo を確かめている (`applied`)。
- message_codes: 範囲のすべてのコード (146) が `docs/editor.md`・`en.json`・`ja.json` にあり、文の引数の名前は `Coded::args` と一致した (手作業の照合。自動化は T0681)。
- codegen: `generated/` と `schema/command.schema.json` が最新かは `apps/editor/src-tauri/src/bindings.rs` のテストと `checked_in_command_schema_is_up_to_date` が検査し、`just ci` で通った。
- ugoe-rigging・ugoe-runtime との重複: 推定・ミラーは `ugoe-rigging`、写像は `ugoe_runtime::WorldTransform` を使っており、重複は逆写像のニュートン法 (エディタだけが持つ) と格子の線形補間 (`grid::lerp_keyform`、キーの追加でだけ使う) のみ。見送り。

### 横断

- 要素を消すコマンドは、消した要素を指す参照 (ミラー元・対・物理・スロットなど) をそれぞれの場所で外している。`RemoveDeformer` と `UnwrapDeformer`、`RemoveMesh` と `RemoveGlue` のように、同じ削除を別の経路で行うコマンドで外し忘れが起きた。削除の後始末を `apply_with_report` の後段 (`physics_edit::forget_removed` と同じく、前後のモデルを比べて消えた ID から外す) にまとめると漏れにくい。
- 読み込んだファイルは検査しない (問題を増やさない編集はできる) 方針なので、編集の関数は「不正だが読み込めるデータ」(空の列・未知の値) でパニックしないことをテストで確かめるとよい。

- 起票は T0681 だけ。枠の T0682〜T0684 は使っていない。

### just ci

- 1 回目の `just ci` は、Rust の fmt・clippy・全テスト・依存方向・ライセンスなどを通ったが、`web-check` の `@ugoe/editor` の Vitest で
  `src/lib/preview/tracking.test.ts` の `beforeAll` が `Hook timed out in 10000ms` で落ちた (変換に 182 秒かかっており、並行する約 8 つのワーカーの負荷による。このタスクはフロントエンドを触っていない)。
  `pnpm --filter '@ugoe/editor' test` を流し直すと 44 ファイル・349 件がすべて通った。続く `web-check` の残り (viewer・studio・face-tracking・player・i18n・runtime・ui の check と test)、`i18n`、`app-build-check` も個別に流して通った。

### マージの注意

- T0667 (コミット 8f459bc) も `UnwrapDeformer` に同じ 1 行 (`inference::forget_missing_mirror_sources(model);`) を足し、テスト `tests::unwrap_deformer_drops_mirror_sources_like_remove` を加えた。行は同じなので衝突しないはずだが、後からマージする側で `deformer_edit_tests::unwrap_forgets_the_deformer_as_a_mirror_source` と重複するテストのどちらかを消す。`RemoveGlue` の修正はこのタスクだけ。
