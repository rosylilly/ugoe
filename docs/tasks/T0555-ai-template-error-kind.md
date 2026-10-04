---
id: T0555
title: "AI: テンプレートで組めない失敗に専用の種類とエディタの文言を付ける"
status: done
milestone: M9
crates: [ugoe-ai, apps/editor, docs]
depends_on: [T0543]
assignee: claude-worker-T0555
---

# T0555: AI: テンプレートで組めない失敗に専用の種類とエディタの文言を付ける

## 目的

T0547 で AI の再試行の失敗の種類を分けたが、テンプレート (自動リギングの型) でモデルを組めない失敗は汎用の種類のままで、
エディタは「このモデルはテンプレートで組めません」と言えない。専用の種類 (`AiError::Template` など) を足し、エディタの `error.rs` と翻訳を同時に直す。

**判断が要る:** `ugoe-ai` の公開 API (`AiError`) の変更。T0543 (AI の公開 API の小さな整理) と同じ型を触るので、その後に着手する。

## 受け入れ条件

- [x] テンプレートで組めない失敗が専用の種類で返り、どのテンプレートで何が足りないかを持つ。テストがある
- [x] エディタの `error.rs` がコード (ADR 0029) で返し、翻訳がある
- [x] `crates/ugoe-ai/README.md` と `docs/ai-integration.md` を更新した
- [x] `just ci` が通る

## 参考

- T0547 のメモ (起票の候補)、T0543
- apps/editor/src-tauri/src/error.rs

## メモ

- T0019 で起票した (元の候補: T0547)。

### やったこと

- `AiError::Template { template: RigTemplate, reason: String }` を足した (文言 "the {character|module} template cannot rig this model: {reason}")。
  `ugoe_ai::rig::RigTemplate` (`Character` = `propose_rig`・`Module` = `propose_module_rig`。`as_str`・`Display`) で、どのテンプレートかを示す。
  `rig::template(kind, reason)` が作り、共有の `Rig` は組んでいるテンプレート (`Rig::template`) を持つ。`ask_until_valid` は今までどおり
  `InvalidProposal` だけを問い直すので、`Template` は 1 回で返り、理由の文は AI に送らない。
- `Template` にしたもの: テンプレートの内部の失敗 (今まで "the template cannot rig this model: …" の `InvalidRequest`) に加えて、モデルの中身が
  足りずに組めないもの (キャラクター: `face` のラベルのメッシュがない。モジュール: 取り付け先の直下に衣装のメッシュがない・組むものがない
  ("nothing to rig")・本体に従うキーフォームが多すぎる)。
  `InvalidRequest` に残したもの: 呼び出しの誤り (モジュールのプロジェクトでない・`max_attempts` が 0・画像が PNG でない・注文が長すぎる) と、
  モデルを評価できない (`mesh_bounds` の compile の失敗。テンプレートではなくモデルの壊れ)。
- テスト: `rig.rs` の `a_template_failure_is_not_asked_again` (種類・理由・文言と、問い直さないこと)、`tests/auto_rig.rs` の顔のないモデル
  (`Character` で `face` を含む理由、AI に送らない)、`tests/module_rig.rs`・`tests/module_rig_sample.rs` の組むものがない場合 (`Module`)。
- エディタ: `error.rs` で `ai.template` (引数 `template`・`detail`)、見本 (`message_samples`) とテストを足した。日英の翻訳
  (「このモデルは{{template}}のテンプレートで組めません ({{detail}})。」)、`errorArgs` に `character`・`module`、`errors.ts` の
  `IDENTIFIER_ARGS` に `template` を足して訳す (`errors.test.ts`)。`just gen` で `messageCodes.ts` を作り直した。`docs/editor.md` のコード表。
- `docs/ai-integration.md` に「テンプレートで組めない失敗」の節を足し、`InvalidRequest` と書いていた箇所を直した。`crates/ugoe-ai/README.md`。
  英語の公開仕様 (`spec/`) に対応する節はない。

### やらなかったこと

- MCP (`ugoe-mcp/src/autorig.rs` の `ai_error`) は範囲の外なので変えていない。`Template` は `e.to_string()` に落ちて
  "the module template cannot rig this model: …" を返す (今までの前置きを外した文より、どのテンプレートかが分かる)。
- 実際のエディタの画面での表示は確かめていない (翻訳と `messageText` は単体テストで確かめた)。

### 起票の候補

- なし。
