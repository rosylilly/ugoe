---
id: T0681
title: エラーと警告のコードと翻訳のカタログの対応を自動で検査する
status: done
milestone: M9
crates: [apps/editor, packages/i18n, docs]
depends_on: [T0668]
assignee: claude-worker-T0681
---

# T0681: エラーと警告のコードと翻訳のカタログの対応を自動で検査する

## 目的

`ugoe-editor-core` が画面に出すエラー・警告のコード (`message_codes.rs`・`import.rs` の `psd.*`・`inference.rs` の
`inference.mirror_failed`・`project.rs`) は、`docs/editor.md#エラーと警告のコード` に載っていることだけがテスト
(`message_tests.rs` の `every_code_is_documented`) で確かめられている。エディタの翻訳のカタログ
(`apps/editor/locales/en.json`・`ja.json` の `errors.<code>`) に文があるか、文の引数 (`{{name}}`) が Rust の
`Coded::args` の名前と一致するかは、どこでも検査していない ([ADR 0029](../adr/0029-shared-ui-and-i18n.md))。

T0668 のレビューで手作業で照合した時点では、すべてのコードに英語と日本語の文があり、引数も一致していた。
コードを足したときにカタログへの追加を忘れると、画面には英語の文 (`message`) が黙って出る。

## 受け入れ条件

- [x] コードの一覧の出し方を決める (例: `ugoe-editor-core` が既知のコードと引数の名前の一覧を返す関数を持ち、`just gen` で
      `apps/editor/src/lib/generated/` に書き出す。MIT のクレートから GPL のカタログを読まない向きにする)
- [x] エディタ側のテスト (Vitest か `just i18n`) で、すべてのコードが `en.json` と `ja.json` の `errors` にあることを確かめる
- [x] 文の引数の名前が、そのコードの引数の名前の部分集合であることを確かめる
- [x] `just ci` が通る

## 参考

- docs/adr/0029-shared-ui-and-i18n.md
- docs/editor.md#エラーと警告のコード
- packages/i18n/scripts/check-catalogs.mjs
- T0668 のメモ

## メモ

### やったこと

- コードの一覧は **見本の値** から作る: `ugoe_editor_core::message_catalog` (新しい公開モジュール) の `message_samples()` が
  editor-core のすべてのコードの見本 (本物のエラーの `to_message()`) を返し、`message_codes()` がコード → 引数の名前
  (原因 `cause` も含め、同じコードの見本の引数を合わせたもの) にする。エディタのバックエンドの `EditorError`・`AiError` の見本は
  `src-tauri/src/error.rs` の `message_samples()`。`just gen` (`bindings.rs`) が両方を合わせて
  `apps/editor/src/lib/generated/messageCodes.ts` に書き出し、`generated_files_are_up_to_date` が最新かを検査する。
  カタログは MIT のクレートから読まない。
- 見本の漏れの検査 (Rust): `message_tests.rs` と `error.rs` の `every_code_has_a_sample` が、ソースに書いたコードの集合と見本の
  コードの集合が等しいことを確かめる。その場でコードと `message_args!` を書く警告 (PSD の取り込みの `Builder::warn`・
  `module.draw_order_outside_slot`) は見本を手で書くので、`inline_warning_samples_match_the_source` がソースの引数の名前と照らし合わせる。
  - 検査で、`docs` の一覧の検査が拾っていなかった `module.draw_order_outside_slot` (project.rs でその場で作る警告) が見つかった
    (カタログと docs にはあった)。`project.json` はファイル名と同じ綴りで `codes_in` が除くので、テストで明示的に足した。
  - `error.rs` の既存の `every_code_is_documented` は `'"'` の数の偶奇に依存して文字列を拾っていたので、コードを拾う関数
    `codes_in_source` にまとめた (プレフィックスだけの文字列 `"editor."` は除く)。
- カタログの検査 (Vitest): `apps/editor/src/lib/messageCodes.test.ts` が、すべてのコードの文が `en.json` と `ja.json` の
  `errors.<code>` (複数形なら `_one`・`_other` など) にあり、文の引数 (`{{name}}`) がそのコードの引数の名前の部分集合であることを
  確かめる。キーを消す・引数の名前を変えると失敗することを手で確かめた。いまは 205 コードすべて通る。
- T0668 の所見 (低) の `edit.unsupported_interpolation` の引数 `interpolation` を、Rust の表記 (`Smooth`) から JSON の名前
  (`smooth`・`linear`・`unknown`) にした (英語の文 `message` は変えていない)。`docs/editor.md` の一覧に書いた。
- 文書: `docs/editor.md` の「エラーと警告のコード」と「型の生成」、`crates/ugoe-editor-core/README.md`。

### 範囲の外の変更

- `crates/ugoe-editor-core` (`crates:` に無い): 受け入れ条件の例のとおり一覧を返す関数を足した (`message_catalog.rs`・`lib.rs` の
  `pub mod`)。`import.rs` の `psd_warning_code` と `inference.rs` の `RederiveWarning::mirror_failed` を `pub(crate)` にし、
  `message_codes.rs` の補間の引数の値を変え、`message_tests.rs` にテストを足した。公開 API の追加は `message_catalog` だけ。

### やらなかったこと

- 見本が網羅するのは「コード」で、同じコードを出す新しいバリアントが別の引数を持つ場合は検査できない (見本を足さない限り)。
  `Coded::args` の `match` は網羅的なので、そこで気づける。
- `packages/i18n/scripts/check-catalogs.mjs` (`just i18n`) には組み込んでいない (コードの一覧はエディタだけのものなので、
  エディタの Vitest に置いた)。
- ビューア・スタジオは `errors.<code>` を引かないので対象外。

### 起票の候補

- 自由な文の引数 (`reason`・`detail`・`details`) を訳すために、元のエラー (検証・合成・推定など) にもコードを付ける
  (docs/editor.md に「後続」とある)。
