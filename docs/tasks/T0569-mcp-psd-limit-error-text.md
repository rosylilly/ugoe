---
id: T0569
title: "ugoe-mcp: PSD の上限のエラーの文の末尾の (code: …) を外すか (判断が要る)"
status: done
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: [T0568]
assignee: claude-worker-T0569
---

# T0569: ugoe-mcp: PSD の上限のエラーの文の末尾の (code: …) を外すか (判断が要る)

## 目的

T0554 で、MCP の自動リギングの PSD の読み込みの失敗 (上限の超過など) は、文の末尾に `(code: psd.channel_limit_exceeded)` の形でコードを付けた
(`crates/ugoe-mcp/src/autorig.rs` の `PsdSource::load_with`)。T0563 でツールのエラーはすべて `structuredContent` の `code`・`args` で
コードを返すようになったので、文の末尾のコードは重複している (T0563 の候補)。ほかのツールのエラーの文にはコードを付けていない。

**判断が要る:** 文を変えると、文で見分けているエージェントやスクリプトに影響する。外す (ほかのツールのエラーと揃える) か、残すか。
推奨は外す (コードは `structuredContent` で読める) の見込み。同じ関数を触る T0568 の後に着手する。

## 受け入れ条件

- [x] 外すか残すかを利用者に確かめた (メモに判断を書いた)
- [x] 外すなら、文からコードを外し、テスト (`autorig.rs` の `ends_with("(code: …)")`) を `structuredContent` のコードを見る形に直した。README を直した
- [x] `just ci` が通る

## 参考

- T0563 のメモ (起票の候補)、T0554 のメモ
- crates/ugoe-mcp/README.md (「ツールのエラー」)

## メモ

- T0036 で起票した (元の候補: T0563)。
- 判断 (2026-10-05、利用者の判断): 外す。文は今の形に即して直す (コードは structuredContent で返る)。
- やったこと: `PsdSource::read_with` の展開の上限などの失敗の文から `(code: …)` を外した (文は `パス: 理由` だけ。コードと引数は
  これまでどおり `structuredContent`)。テスト `exceeding_an_import_limit_names_the_limit` は、文が理由で終わり `(code:` を含まないことと、
  `detail().code` が `psd.channel_limit_exceeded`・`psd.total_limit_exceeded` であることを見る形にした。
  `docs/ai-integration.md` の「PSD を読めないとき」と「エラーのコード」、`crates/ugoe-mcp/README.md` の「ツールのエラー」を、
  文にはコードを付けない今の形に書き直した (あわせて「下の『エラーのコード』」を「上の」に直した。節は上にある)。
- 英語の公開仕様 (`spec/`) にこの文の記述は無かった。`just ci` は通った。
- 起票の候補: なし。
