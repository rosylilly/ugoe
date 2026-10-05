---
id: T0567
title: "レイヤーの分類の答えそのものをプロジェクトに残すか (判断が要る)"
status: todo
milestone: M9
crates: [ugoe-editor-core, apps/editor, ugoe-mcp, docs]
depends_on: [T0566]
assignee:
---

# T0567: レイヤーの分類の答えそのものをプロジェクトに残すか (判断が要る)

## 目的

T1615 で、利用者の直し (`editor.classification` の `corrections`) はプロジェクトに残るようになったが、分類の答え
(名前の規則と AI の答えのラベル・左右・確からしさ・差分の組) はエディタのメモリ (`ProposalSession`) にしかなく、
プロジェクトを開き直すと分類し直し (AI を使ったなら呼び直し) が要る (T1615 の候補)。

**判断が要る:** 残すか、残すなら置き場と形。案の例:

- A. 残さない (今のまま)。開き直したら分類し直す。直しは残るので利用者の答えは失われない。
- B. `project.json` の `editor.classification` に答えも書く (元の PSD の識別 (パス・大きさ・ハッシュなど) を添え、PSD が変わっていたら捨てる)。
  Undo の単位に入れるか (`Snapshot`) も決める。
- C. プロジェクトの中の別のファイル (キャッシュ) に書き、`project.json` は変えない。

docs/editor.md・docs/spec/file-formats.md の `editor.classification` の節を変えるので、案と推奨をメモに書いて利用者の判断を得る。
MCP サーバも同じものを読むかを含めて決める (MCP が直しを読む T0566 の後に着手する)。

## 受け入れ条件

- [ ] 案と推奨 (ファイル形式・Undo・MCP との共有・AI の答えの扱い) をメモに書き、利用者の判断を得た
- [ ] 決めた形で実装し (A なら文書だけ)、開き直しても分類が戻るテストがある (A 以外)
- [ ] docs/editor.md・docs/spec/file-formats.md・docs/ai-integration.md を直した
- [ ] `just ci` が通る

## 参考

- T1615 のメモ (案 A〜C と利用者の判断)
- docs/editor.md (「分類の直し」)、docs/spec/file-formats.md (`editor.classification`)

## メモ

- T0036 で起票した (元の候補: T1615 (分類の答え))。
