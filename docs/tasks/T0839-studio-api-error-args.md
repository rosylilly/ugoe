---
id: T0839
title: スタジオの操作 API の -32010 の error.data に args と ADR 0029 のコードを足すか決める (判断が要る)
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0838]
assignee: claude-worker-T0839
---

# T0839: スタジオの操作 API の -32010 の error.data に args と ADR 0029 のコードを足すか決める (判断が要る)

## 目的

T0835 でスタジオのコマンドのエラーを `{ code, args, message }` にしたが、操作 API の `-32010` の `error.data` は外のツールとの公開の約束なので、
平らな引数と camelCase のコード (`unknownScene` など) のまま残した (T0835 の候補)。`args` と ADR 0029 のコード (`scene.unknown` など) を足せば、
外のツールがスタジオと同じ訳を使える。足すだけなら互換を壊さない。

**判断が要る:** 外部 API (docs/studio-api.md) の公開の約束の変更。足すか、足すならフィールドの名前。

## 受け入れ条件

- [x] 利用者の判断を得た (待つ間は blocked にしてメモに案を書く)
- [x] 足すなら、`-32010` の `error.data` に新しいフィールドがあり、今のフィールドは変わらないテストがある。docs/studio-api.md を更新した
- [x] `just ci` が通る

## 参考

- T0835 のメモ (起票の候補)、ADR 0029、apps/studio/README.md (「コマンドのエラー」)

## メモ

- T0026 で起票した (元の候補: T0835)。オーケストレーターの集計には無かったが、M9 の done のタスクのメモの起票の候補にあった。

- **判断 (2026-10-05、利用者の判断):** 足す (互換を保つ)。-32010 の error.data の元のフィールド (平らな引数と camelCase のコード) は残し、ADR 0029 の形のコードと args を足す。docs/studio-api.md も同じ PR で直す。

- やったこと:
  - `-32010` の `error.data` に `messageCode` (ADR 0029 のコード。`scene.unknown`・`scene.model_missing`・`animation.no_model` など)
    と `args` (コマンドのエラーと同じ引数) を足した。元の `code` (camelCase) と平らなフィールドは変えていない。
    `api/rpc.rs` の `RpcError::failed` が、`SceneError`・`AnimationError` を `CommandError` に変えて (error.rs の `From` の実装を
    そのまま使う) 足す。`failed` は `CommandError` を境界に取るので `pub(crate)` にした (呼び出しはクレートの中だけ)。
  - フィールドの名前: コードは `code` が既にあるので `messageCode` (エディタの「Message (コード・引数・英語の文)」の呼び名に
    合わせた)、引数は `args`。どちらも今の `SceneError`・`AnimationError` のフィールドとぶつからない。
  - テスト: rpc.rs (`failed_keeps_the_fields_and_adds_the_message_code_and_args`。元のフィールドと足したフィールドの全体を比べる)、
    api/tests.rs (WebSocket を通した `scene.activate` の失敗で `code`・`id`・`messageCode`・`args` を確かめる)。
  - 文書: docs/studio-api.md (Errors に `messageCode`・`args` と、カタログ `apps/studio/locales/*.json` の `errors.<messageCode>`
    で訳を引けることと例。Versioning に「`error.data` の新しいフィールド」を足しても `apiVersion` は変えないことを明記)、
    apps/studio/README.md (操作 API との関係)、error.rs のモジュールの doc。
- やらなかったこと:
  - `apiVersion` は上げていない (足すだけで、Versioning の約束どおり)。
  - 実際の外部のクライアントでの確認はしていない (Rust のテストで確かめた)。
- 起票の候補: なし。
