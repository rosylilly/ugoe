---
id: T1615
title: "エディタ: 分類の結果 (ラベル・左右) を直し、食い違いを「これでよい」と認めて印を消す (判断が要る)"
status: done
milestone: M9
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1615
---

# T1615: エディタ: 分類の結果 (ラベル・左右) を直し、食い違いを「これでよい」と認めて印を消す (判断が要る)

## 目的

T0546 で AI の分類と名前の規則の食い違いに印を付けて一覧で見せるようにしたが、エディタで分類の結果 (ラベル・左右) を直したり、
食い違いを認めて印を消したりできない (T0546 の候補)。

**判断が要る:** 直した結果と「認めた」の印をどこに保存するか (`.ugoeproj` のメタ情報か、モデルか)。保存するなら `Command` を通して Undo できるようにする。

## 受け入れ条件

- [x] 保存先を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] 分類の確認の一覧からラベル・左右を直せ、食い違いを認めると印が消える。どちらも `Command` を通り Undo できる。editor-core のテストがある
- [x] 画面の文に訳があり、docs/editor.md (と docs/manual) を更新した
- [x] `just ci` が通る

## 参考

- T0546 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0546)。
- **blocked (保存先の判断が要る):** 既存の設計から保存先が決まらないので止めた。実装はしていない。
  - **今の保存のされ方:** 分類の結果 (ラベル・左右・確からしさ・差分の組) と T0546 の食い違いの印 (`LayerLabel::name_disagreement`) は、
    エディタでは Tauri 側の `ProposalSession.classification` (`apps/editor/src-tauri/src/proposals.rs` の `Classified { psd, labels, .. }`) の
    **メモリだけ** にあり、`.ugoeproj` にもモデルにも書かない (docs/editor.md「分類とその PSD はプロジェクトを開いている間だけ持ちます (ファイルには残さない)」)。
    別の PSD を選ぶと捨てる。MCP サーバも自分のメモリに持ち (`set_layer_labels` は「モデルは変えない」)、エディタとは共有しない。
    `ugoe_editor_core::Document` の `Snapshot` (Undo の単位) は model・textures・animations・physics・module だけで、
    `project.json` の `editor.*` (`editor.lighting`・`editor.rigged_meshes` など) は Snapshot の外にあり、`Command` も Undo も通らない。
  - **決まらない理由:** 受け入れ条件の「`Command` を通り Undo できる」は Document の状態に入れることを前提にするが、
    分類は今「ファイルに残さない」と決まっており、ADR 0006・0022・docs/ai-integration.md にも保存先の定めがない。どれを選んでも
    文書の方針 (残さない) かファイル形式・スキーマのどれかを変える。
  - **案 A (メモリのまま):** `ProposalSession` の分類を直接直し、認めた印もそこに持つ。ファイル形式・スキーマへの影響なし。
    Undo は `Command` を通らない (セッションの中の小さな履歴を持つか、Undo なし)。閉じると直しも消える (分類も消えるので一貫はする)。
    受け入れ条件の「Command を通り Undo」を外すことになる。MCP とは共有しない。
  - **案 B (`.ugoeproj` の `project.json` の `editor.classification`):** 元の PSD (パス) と、レイヤーのパスごとの **直し** (ラベル・左右) と
    **認めた印** (認めたときの名前の規則の答えも持ち、名前の規則の答えが変われば印を戻す) を書く。分類し直しても直しと印を上に重ねる。
    モデルと `SCHEMA_VERSION` は変えない。`docs/spec/file-formats.md` の project.json の表に 1 行足す (省略できるキーなので `PROJECT_FORMAT_VERSION` は上げない)。
    Undo: editor-core の `Snapshot` にこの情報を足し、新しい `Command` (例: `SetLayerClassification { layer_path, label?, side?, acknowledge? }`) で変える
    (editor.* を Snapshot に入れる初めての例になる。`editor.rigged_meshes` は前例だが Undo しない)。MCP も同じファイルから読めば直しを共有できる。
    editor.md の「ファイルには残さない」を改める。editor-core を触るので並行の T1621・T0171 と衝突しやすい。
  - **案 C (モデル `ugoe-model`):** メッシュ (部品) に意味のラベル・左右を持たせる。`docs/spec/model.md`・`file-formats.md`・`SCHEMA_VERSION`・英語の公開仕様 `spec/` を変える
    (ランタイム層。AGENTS.md の「core / model の公開 API はインターフェースだけ先に入れる」に当たる)。分類は PSD のレイヤー単位なので
    取り込んだ後のメッシュとの対応も要る。Undo は普通の `Command` で自然に通る。ランタイム (アバターの約束・ライティングのまとまり) でも使えるが最も重い。
  - **推奨: 案 B。** 受け入れ条件 (Command と Undo) を満たしつつモデルとランタイム層に触れず、エディタの作業の情報を `project.json` の `editor.*` に置く
    前例 (`editor.rigged_meshes`) に沿い、閉じて開き直しても直しと「認めた」が消えない。モデルに持つ価値 (案 C) が出たら、そのとき別の ADR で移す。
    小さく済ませたいなら案 A (受け入れ条件から Undo を外す)。
- **利用者の判断 (2026-10-05): 案 B。** `project.json` の `editor.classification` に置き、新しい `Command` で変えて Undo できるようにする。
  モデルと `SCHEMA_VERSION` は変えない。省略できるキーなので `PROJECT_FORMAT_VERSION` は上げない。docs/editor.md の「ファイルには残さない」を改める。
- **やったこと:**
  - editor-core: `src/classification.rs` に `LayerCorrection { label, side }`・`LayerSide`・`LayerCorrections` (レイヤーのパス → 直し) と、
    `ProjectMeta::layer_corrections`・`set_layer_corrections` (`editor.classification` = `{"corrections": [{path, label, side}]}`。読めない件は捨てる)。
    `Document` の `Snapshot` に `corrections` を足し (Undo の単位)、新しい `Command::SetLayerCorrection { path, correction }` (`null` で消す) で変える。
    モデルは変えず検証もしない。同じ値なら何もしない (Undo を積まない)。`Batch` の中では拒む。形の検査 (パス 1〜64 段・名前 1024 文字・
    ラベル 1〜64 バイトの `[a-z0-9_]`・65536 件まで) に外れれば新しいエラー `edit.invalid_layer_correction` (翻訳・見本・一覧も足した)。
    `Project::document` が読み、`Project::from_document` が書く。ラベルの語彙は ugoe-ai にあり editor-core から見えないので形だけを確かめる。
  - エディタのバックエンド: `corrected_labels` で分類に直しを重ねる (直したレイヤーは確からしさ 1・食い違いの印なし。語彙にないラベルは重ねない)。
    `get_proposals` の一覧・リグの提案と作り直し・分類の作り直しの前の答え・法線マップのまとまりがどれも重ねた分類を使う。
    `LayerLabelView.corrected` と `Classification.label_choices` を足した。法線マップのまとまりの IPC はドキュメントとプロジェクトのロックを
    `lock_both` の順で取るようにした。
  - 画面: AI タブの分類の確認の一覧 (確かめるもの + 直したもの) の各行に、ラベルの Select・左右の SegmentedControl・「Looks right」(今の答えで認める)・
    「Reset」(直しを消す)。どれも `set_layer_correction` を 1 つ送る (`ProposalsView.correctLayer`・`resetLayer`)。日英の訳。
  - テスト: `src/classification_tests.rs` (Undo/Redo・同じ値・形の検査・Batch・保存と読み込み・壊れた `project.json`)、
    `proposals_tests.rs` の `corrections_overlay_the_classification_and_undo`、`proposalsView.test.ts` の送るコマンド。
  - 文書: docs/editor.md (新しい節「分類の直し」・コマンド一覧・IPC の表・エラーのコード・「ファイルには残さない」を改めた)、
    docs/spec/file-formats.md (`editor.classification`)、docs/ai-integration.md、docs/manual/ai.md (「分類を直す」)、editor-core の README。
- **決めたこと (案 B の細部):** 「認める」は今のラベル・左右をそのまま直しとして書く (直しと同じ仕組み。分類し直しても利用者の答えが残る)。
  直しは PSD のパスを持たずレイヤーのパスだけで対応させる (プロジェクトや PSD を動かしても効き、同じ構成で書き出し直した PSD にも効く)。
  直しもドキュメントの状態を変えるので、置いてある提案は古く (stale) なる。
- **やらなかったこと:** MCP サーバ (`get_psd_layers`・`set_layer_labels`・`propose_rig`) が `editor.classification` の直しを読むこと
  (MCP は分類を自分のメモリに持つ。範囲外の ugoe-mcp を触るので見送った)。`just ui-audit` は流していない (一覧は分類した後にしか出ない)。
  実際のエディタの画面での見た目は確かめていない (svelte-check・Vitest・Rust のテストまで)。
- **起票の候補:**
  - MCP サーバの分類 (`get_psd_layers`・`set_layer_labels`・`propose_rig`・`propose_module_rig`) に `project.json` の `editor.classification` の直しを重ねる。
  - 分類の答えそのものもプロジェクトに残す (今は開き直すと分類し直しが要る)。
- `just ci` は通った (rebase main の後)。
