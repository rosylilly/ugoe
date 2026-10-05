---
id: T0567
title: "レイヤーの分類の答えそのものをプロジェクトに残すか (判断が要る)"
status: done
milestone: M9
crates: [ugoe-editor-core, apps/editor, ugoe-mcp, docs]
depends_on: [T0566]
assignee: claude-worker-T0567
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

- [x] 案と推奨 (ファイル形式・Undo・MCP との共有・AI の答えの扱い) をメモに書き、利用者の判断を得た
- [x] 決めた形で実装し (A なら文書だけ)、開き直しても分類が戻るテストがある (A 以外)
- [x] docs/editor.md・docs/spec/file-formats.md・docs/ai-integration.md を直した
- [x] `just ci` が通る

## 参考

- T1615 のメモ (案 A〜C と利用者の判断)
- docs/editor.md (「分類の直し」)、docs/spec/file-formats.md (`editor.classification`)

## メモ

- T0036 で起票した (元の候補: T1615 (分類の答え))。
- 判断 (2026-10-05、利用者の判断): 案 C。分類の答えはプロジェクトの中の別のキャッシュのファイルに書き、`project.json` は変えない。元の PSD の識別 (ハッシュなど) が合わなければ捨てる。Undo の対象にしない。MCP サーバも同じファイルを読む。
- **やったこと (案 C):**
  - editor-core: `src/classification_cache.rs` (`.ugoe-cache/classification.json`。`CachedClassification<L>`・`PsdIdentity` (大きさと FNV-1a 64)・
    `write_classification_cache` (`write_if_changed` = `ugoe_fs::write_atomic`、中身が同じなら書かない)・`read_classification_cache`
    (通常のファイルだけを 64 MiB まで。読めない JSON・版違い・65536 件を超えるものは `None`)。PSD のパスはプロジェクトの中なら相対、外なら絶対)。
    レイヤーの型は ugoe-ai にあるので型引数で受け取る。`project.json` は変えず、`Document`・Undo にも入れない。
    範囲内の小さな変更として `project/files.rs` の `to_json`・`write_if_changed` と `paths.rs` の `normalize_path` を `pub(crate)` にした。
  - エディタ: `Classified` に PSD の識別を持たせ、保存先のあるプロジェクトで分類したとき (ロックの外) と保存したとき (`PendingSave::write`) にキャッシュを書く。
    `ProjectSession::from_project` が保存先のあるプロジェクトを開くとき、PSD の大きさ・ハッシュ・レイヤーが合えば分類と PSD を戻す。書けない・合わないときは何もしない (標準エラーに書くだけ)。
  - MCP: PSD を読むとき同じ PSD のキャッシュがあれば名前の規則の案の代わりに使う (`set_by: "editor"`・`saved_by_editor`・提案の理由)。
    `psd` を渡されず覚えてもいなければキャッシュの PSD を読む (`--root` で確かめる。get_psd_layers・set_layer_labels・リグのガイドと提案)。書くのはエディタだけ。
  - **範囲の外の小さな変更:** ugoe-ai に `classify::cached_labels` (レイヤーの数・パス・確からしさを確かめ、食い違いの印を付け直す) を足した
    (エディタと MCP で共有するため。T0566 の `corrected_labels` と同じ置き場)。テストは `tests/classify.rs` の `cached_labels_must_fit_the_psd`。
  - テスト: editor-core の `classification_cache_tests.rs`、エディタの `the_classification_is_restored_when_the_project_is_reopened`
    (分類で書く・保存で書く・開き直して AI の答えと注文が戻る・PSD を変える/消すと戻らない・project.json は変わらない)、
    MCP の `tests/stdio.rs` の `the_classification_saved_by_the_editor_is_used`。
  - 文書: docs/editor.md (新しい節「分類の答えのキャッシュ」)、docs/spec/file-formats.md (新しい節「分類のキャッシュ」・ディレクトリ構成)、
    docs/ai-integration.md (「エディタが残した分類 (T0567)」)、docs/manual/ai.md、ugoe-mcp と ugoe-editor-core の README。
- **やらなかったこと:** MCP からキャッシュを書くこと (エージェントの答えはサーバのメモリだけ。判断のとおり MCP は読むだけ)。
  画面 (IPC の型) は変えていない (戻した分類は今の `get_proposals` の `classification` にそのまま出る)。実際のエディタの画面での確認はしていない
  (Rust のテストまで)。キャッシュのある大きな PSD のプロジェクトは、開くときに PSD を読み直してハッシュを求めるので、その分だけ開くのが遅くなる (文書に書いた)。
- **起票の候補:**
  - MCP のエージェントの分類 (`set_layer_labels`) もキャッシュに書き、エディタで開いたときに戻す (書き手が 2 つになるので、`by_ai` に加えて誰の答えかを持たせる)。
  - キャッシュのあるプロジェクトを開くときの PSD の読み直しを、開いた後のバックグラウンドに回す (大きな PSD で開くのが遅くならないように)。
- `just ci` は通った (rebase main の後)。
