---
id: T1606
title: "エディタ: プロジェクトのモーションと表情も検証し、拒んだ理由をコードで出す"
status: done
milestone: M9
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1606
---

# T1606: エディタ: プロジェクトのモーションと表情も検証し、拒んだ理由をコードで出す

## 目的

T0323 でモーションと表情の数値の検証 (`validate_motion`・`validate_expression`) を足し、`ugoe-format` の読み込みは
`FormatError` の新しい 2 つの変種で拒むようになった。エディタには 2 つの漏れがある。

- `.ugoeproj` のモーション・表情は `ugoe-editor-core` の `project/files.rs` が `serde_json` で直接読むので検証されない
  (書き出したバンドルを読み直すと拒まれる)。
- `message_codes.rs` は `FormatError` の知らない変種を `format.corrupt` にまとめるので、新しい 2 つは「壊れている」と出る。
  `format.invalid_motion`・`format.invalid_expression` のコードと翻訳を足す。

## 受け入れ条件

- [x] プロジェクトの読み込みで NaN を含むモーション・表情が拒まれ (場所つき)、テストがある
- [x] `format.invalid_motion`・`format.invalid_expression` のコードと翻訳を足し、`docs/editor.md#エラーと警告のコード` を更新した (メッセージの一覧の検査 T0681 が通る)
- [x] `just ci` が通る

## 参考

- T0323 のメモ (起票の候補)
- crates/ugoe-editor-core/src/project/files.rs・message_codes.rs

## メモ

- T0019 で起票した (元の候補: T0323 のエディタのコードと `.ugoeproj` の検査)。

### やったこと

- `Project::load` が `motions/*.motion.json`・`expressions/*.expression.json` を `validate_motion`・`validate_expression` で検査し、
  `ProjectError::Format { path, source: FormatError::InvalidMotion / InvalidExpression }` で拒む (`project/files.rs` の `read_json_dir` に
  検査の関数を渡す)。JSON は NaN を書けないので、テストは `f32` で無限大になる `1e39` と負の `duration` で確かめた
  (`project_tests.rs` の `load_rejects_motions_and_expressions_with_invalid_numbers`)
- `message_codes.rs`: `format.invalid_motion`・`format.invalid_expression` (引数 `name`・`count`・`details`) を足し、検証エラーの列を持つ種類は
  `FormatError::validation_errors()` で引数を作るようにした。`details` は `describe_errors` と同じく先頭の `DESCRIBED_ERRORS_MAX` 件までにし、
  残りは `; and N more` にまとめた (T0015 の候補)
- `ProjectError::InvalidModule` の文を `describe_errors("module", …)` に揃えた (T0015 の候補)
- 日英の翻訳、カタログの見本 (`message_catalog.rs`)、`just gen` の `messageCodes.ts`、`docs/editor.md` のコードの一覧、
  `docs/spec/model.md#モーションと表情の検証` の「呼ぶ場所」と `docs/spec/file-formats.md` のプロジェクトの節を更新した
- `just ci` が通った

### やらなかったこと

- 英語の公開仕様 (`spec/`) は触っていない (「呼ぶ場所」はエディタの実装の話で、公開仕様に対応する節がない)
- モジュールのプロジェクトはモーション・表情を読まないので対象外
- 保存 (`Project::save`) では検査しない (編集のコマンド側の検査に任せる)

### 起票の候補

- `ugoe-format` の `check_motion`・`check_expression` を公開し、editor-core の同じ関数 (`project.rs`) を置き換える
