---
id: T0667
title: コードレビュー: ugoe-editor-core (文書・コマンド・プロジェクト)
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0667
---

# T0667: コードレビュー: ugoe-editor-core (文書・コマンド・プロジェクト)

## 目的

ugoe-editor-core (文書・コマンド・プロジェクト) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-editor-core` の document・command・project・module_project・module_compare とそのテスト (編集の各モジュールは T0668)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - Undo と Redo の対称性 (適用して戻すと元に戻るか)、`Command` enum の網羅
  - `.ugoeproj` の保存と読み込みの往復、外からの変更 (T0641) との競合
  - 1000 行を超えるファイル (document.rs・module_compare.rs・project.rs) の分け方
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0677〜T0680 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/project.rs` `remove_stale` | 大文字小文字だけを変えた名前 (`RenameMotion` の `idle` → `Idle`、テクスチャを消して大文字小文字違いの名前で足すなど) で保存すると、大文字小文字を区別しないファイルシステム (Windows・macOS) では、古い名前のファイルとして書いたばかりのファイルを消す (モーション・表情・テクスチャが失われ、テクスチャなら次に開けない) | 直した (古い名前のファイルを消した後に書き直す。テスト `renaming_only_the_case_keeps_the_file` を追加。CI と手元の Linux は大文字小文字を区別するので、先に失敗するテストは書けなかった) |
| 高 | `src/module_project.rs` `Splitter::module` | モジュールのプロジェクトで足した法線マップの名前が、ファイルから読んだ法線マップ (作業用の名前は `<モジュール>/<名前>`) やテクスチャと接頭辞を外すと重なっても受け付け、分けるときに画像の片方を黙って上書きする (保存すると片方の画像が失われる) | 直した (テクスチャと法線マップの名前を ID と同じく `renames` で調べ、`ModuleShape` で拒否。先に失敗するテスト `normal_maps_that_collide_without_the_prefix_are_rejected`) |
| 中 | `src/document.rs` `apply_to` の `UnwrapDeformer` | 外したデフォーマをミラー元 (`keyforms.mirror_source`) にする格子の記録を外さないので、検証 (`UnknownReference`) で拒否される (`RemoveDeformer` は外す) | 直した (`forget_missing_mirror_sources` を呼ぶ。先に失敗するテスト `unwrap_deformer_drops_mirror_sources_like_remove`。`docs/editor.md#削除の連鎖と拒否` にも書いた。T0668 も同じ 1 行を c05c1bd で直したので、マージで重なる。`RemoveGlue` の同じ誤りは T0668 が 2ac9e2b で直した) |
| 中 | `src/project.rs` `files`・`to_json` | 新しい版のファイルから読んだ未知の値 (`Unknown`) を含むモーション・表情・物理を保存すると、serde のエラーで、パスが空の `ProjectError::Json` (`: the enum variant CurveTarget::Unknown cannot be serialized`) になる。`model.json` は `FormatError::UnknownValues` で場所を示す | 直した (`ugoe_model::unknown::in_*` で調べて `ProjectError::Format { path, UnknownValues }`。JSON のエラーにも書き込み先のパスを付ける。先に失敗するテスト `save_rejects_unknown_values_with_the_file_name`) |
| 中 | `src/module_project.rs:74` `From<ModuleError> for EditError` | `ReadOnlyBase` 以外の `ModuleError` (`ColliderNeedsBasePhysics`・`UnknownSlot` など) を `EditError::ModuleShape { reason: to_string() }` に畳むので、コード (ADR 0029) が `edit.module_shape` になり案内が翻訳できない | T0678 を起票 (公開 API とコードの一覧の変更) |
| 中 | `src/module_compare.rs:87` | `.ugoemod` が読めないときに `ProjectError::Export` (「bundle export failed」) を返す。読み込みの失敗が書き出しの失敗と表示される | T0678 を起票 |
| 中 | `src/command.rs:152` `SetMeshProperties::texture` | テクスチャを番号で指定する (コマンドの規則「インデックスは使わない」に反する)。`RemoveTexture` が番号を詰めるので、古い状態で作った提案を後で適用すると別のテクスチャを指す (検証は通る) | T0679 を起票 (コマンドの JSON の変更) |
| 中 | `src/document.rs` `Document` | Undo の履歴に上限がなく、コマンドごとにモデル・物理・アニメーションを丸ごと複製して積む。適用のたびに適用前のモデルも `validate` し直す | T0680 を起票 (ADR 0005 の暫定の実装の範囲で上限と検証の結果の記憶) |
| 低 | `document.rs` (約 1660 行)・`module_compare.rs` (約 1570 行)・`project.rs` (約 1160 行)・`module_project.rs` (約 1050 行) | 1000 行を超える。どれも独立した部分 (削除の連鎖・テクスチャのコマンド・アトラスを戻す部分・`Splitter`・モジュールの読み書き) を持つ | T0677 を起票 (分け方の案を書いた) |
| 低 | `src/document.rs:476` `set_module_settings` | `keep_look` のときの `inference::rederive` の警告 (`RederiveWarning`) を捨て、`apply_with_report` は空の報告を返す | 見送り (起票の候補。ミラーのやり直しが失敗する `keep_look` の組み立てが大きいので、T0680 か T0678 のついでに `ApplyReport` で返す) |
| 低 | `src/document.rs:542` `rebase_module_on` | モジュールのプロジェクトでないときのエラーが `ModuleError::Shape` で、`EditError::NotModule`・`ProjectError::NotAModule` と揃っていない | T0678 に含めた |
| 低 | `src/project.rs:118` | `NotAModule` と `InvalidModule` の doc コメントが入れ違って 1 つに重なっていた。`module_meta_with` の doc コメントが 2 文で重複 | 直した |
| 低 | `src/document.rs:40` | `HasChildren` の doc コメントのバッククォートが閉じていない (clippy pedantic の `doc_markdown`) | 直した |
| 低 | `docs/editor.md#外での書き換え` | 指紋を取るファイルの一覧に `module.json` がない (実装 `fingerprint.rs` は含む) | 直した |
| 低 | `src/module_project.rs:895` | グルーの格子のミラー元を空の表で付け替えるので、モジュールのグルーどうしのミラー元が「本体の要素をミラー元にする」という誤った文で拒否される | 見送り (`ugoe-rigging` はグルーの格子をミラーできないので、手で書いたファイルでしか起きない) |
| 低 | `src/project.rs` `load` | モーション・表情は読み込むとファイル名の順になり、保存前の並び (追加の順) と変わる (書き出す `.ugoe` の並びも変わる) | 見送り (`Project::motions` の doc に書いてある仕様) |
| 低 | `src/project.rs` `write_if_changed` | 書き込みの途中で落ちると `*.ugoe-tmp` が残り、以後消さない | 見送り (読み込み・指紋は無視する。害は小さい) |
| 低 | `src/module_compare.rs` `Finder::decode` | 比べる `.ugoemod` のアトラスの PNG を大きさを確かめずに RGBA に展開する | 見送り (自分で書き出したファイルを比べる。`.ugoemod` の大きさの上限は `ugoe-format` の側の話) |
| 低 | `src/project_tests.rs`・`src/module_project_tests.rs` (と範囲外の `fingerprint_tests.rs`・`bundle_project_tests.rs`) | 一時ディレクトリの補助 (`TempDir`) をテストのファイルごとに書いている | 見送り (横断に書いた) |

確かめたこと (所見なし):

- **Undo と Redo の対称性:** スナップショット方式なので、適用・Undo・Redo は状態を丸ごと入れ替える。`SetModuleSettings` も `ModuleContext` を
  スナップショットに持つので戻る。本体の読み直し (`rebase_module`) だけは履歴を消す (設計どおり、`docs/editor.md#本体の変化`)。テストの `applied` が
  コマンドごとに Undo・Redo で前後に戻ることを確かめている。
- **`Command` の網羅:** `tests.rs` の `op_name` が網羅的な `match` で、`OP_COUNT` と `json_samples` (`codegen` の例) がすべてのバリアントの JSON の往復を
  確かめる。`apply_to` も網羅的な `match` (`SetModuleSettings` は `Batch` の中では拒否)。`RetriangulateMesh`・`AddKeyformKey`・`RemoveKeyformKey`・
  `SetKeyformInterpolation` が `touched` に積まないのは `docs/editor.md#キーフォームの編集` のとおり。
- **`.ugoeproj` の往復:** 保存は書く前にすべての中身を作り、名前を検査し、変わったファイルだけを一時ファイル経由で書き、`project.json` を最後に書く。
  読み込みは名前を検査してからパスを作る (`..` を拒否)。モジュールのプロジェクトは `module.json` と画像だけを書き、本体は書かない。
- **外からの変更 (T0641) との競合:** `Project::save` は指紋を見ず、確かめるのはバックエンドの `ProjectSession` (範囲外)。`save` は中身の同じファイルを
  書き直さないので、保存の直後に取り直す指紋が外の変更と取り違えない。「Overwrite with my version」では、外で足されたモーション・表情・テクスチャの
  ファイルも `remove_stale` で消える (上書きの意味どおり)。

### 横断 (T0008 向け)

- **大文字小文字を区別しないファイルシステム:** 「書いた後に、今回書かなかったファイルを消す」処理は、大文字小文字だけが違う名前を同じファイルとして
  消しうる。ディレクトリに書き出すほかの処理 (CLI・MCP・サンプルの生成など) も同じ形なら確かめる。CI は Linux だけなので、この種の誤りはテストで
  見つからない (Windows・macOS で Rust のテストを流すジョブがない)。
- **未知の値の書き出し:** `#[serde(skip_serializing)]` の `Unknown` は、`serde_json` で書くと場所の分からないエラーになる。`ugoe-format` の
  `physics_to_json` は `model_to_json`・`module_to_json` と違って `ugoe_model::unknown::in_physics` で調べない。モーション・表情を書くほかの経路も同様。
- **エラーのコードの平坦化:** `From<X> for Y` で、コードを持つエラーを `reason: other.to_string()` の文字列に畳むと ADR 0029 のコードが失われる
  (`ModuleError` → `EditError`)。ほかのクレートの `From` の変換も確かめる。
- **テストの一時ディレクトリ:** 各テストのファイルが `std::env::temp_dir()` とプロセス ID で一時ディレクトリの補助を書いている
  (ugoe-editor-core だけで 6 か所)。名前が重なると並列のテストが不安定になるので、共通の補助 (または `tempfile` の dev-dependency) にまとめるとよい。
