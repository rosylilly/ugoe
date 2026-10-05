---
id: T1618
title: "editor-core: Coded::code を元のクレートのコードから組み立て、手で並べたコードの一覧をなくす"
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0168]
assignee: claude-worker-T1618
---

# T1618: editor-core: Coded::code を元のクレートのコードから組み立て、手で並べたコードの一覧をなくす

## 目的

T0167 で合成のエラーのコードを `ugoe-runtime` の `ComposeError::code()` から作るようにしたが、editor-core の `Coded::code` は `&'static str` なので、
元のクレートのコードに領域の名前 (`validation.`・`infer.`・`world.`・`normal_map.`・`compose.`) を付けたものを、手で並べた一覧で持っている (T0167 の候補)。
`Coded::code` を `Cow<'static, str>` などにして組み立てれば、元のクレートにコードが増えても一覧を直さずに済む。同じ着せ替えのエラーを触る T0168 の後に着手する。

## 受け入れ条件

- [x] `Coded::code` が元のクレートのコードと領域の名前から組み立てたコードを返し、`validation`・`infer`・`world`・`normal_map`・`compose` の手で並べた一覧が無い
- [x] 出すコードの文字列は今と同じ (エディタの訳の表と `codes.json` を変えない)。元のクレートのすべてのコードについて今と同じ文字列になることを確かめるテストがある
- [x] `just ci` が通る

## 参考

- T0167 のメモ (起票の候補)
- ADR 0029

## メモ

- T0032 で起票した (元の候補: T0167)。
- claude-worker-T1618 のメモ:
  - `Coded::code` を `Cow<'static, str>` にした (`src/message.rs`)。固定のコードの実装は `Cow::Borrowed(match …)`、ほかの `Coded` に任せる腕
    (`EditError::Module` など) は `return error.code()`。検証・推定・写像・法線マップ・合成は `message_codes.rs` の `sourced(area, code)` で
    `{領域}.{元のコード}` を組み立て、手で並べた 5 つの一覧を消した。
  - テスト (`message_tests.rs`): `sourced_codes` が元のクレートのソース (`ugoe-model/src/validate/code.rs`・`ugoe-rigging/src/{error,world,normal}.rs`・
    `ugoe-runtime/src/compose.rs`) の `code()` から全コード (75 個) を読み取る。`source_error_codes_follow_the_crate_codes` は、見本の
    `Coded::code` が `{領域}.{元のコード}` と一致し、見本の集合が元のクレートの全コードと等しいことを確かめる。`every_code_is_documented`・
    `every_code_has_a_sample` はソースの文字列に加えて組み立てるコードも対象にした (元のクレートにコードが増えたら、見本・docs/editor.md の
    一覧が無いことでテストが落ちる)。値が今と同じことは、docs/editor.md の一覧にすべてあることと、`just gen` で生成物
    (`apps/editor/src/lib/generated/messageCodes.ts` など) が変わらないことで確かめた。訳のキー・messageCodes.ts・docs/editor.md の表は変えていない。
  - 範囲外の小さな変更: `apps/editor/src-tauri/src/error.rs` の `EditorError` の `Coded::code` を新しい型に合わせた (match を inherent の
    `fixed_code` に移し、`Coded::code` は `Cow::Borrowed(self.fixed_code())`。T1613・T0029 との衝突を減らすため、腕の行は動かしていない)。
    ugoe-mcp・ugoe-cli・ugoe-ai は変更なしでビルドできた。
  - docs: `docs/editor.md` の「元のエラーのコード」に組み立てる旨とテストを書いた。README に T1618 の行。
  - 起票の候補: なし。
