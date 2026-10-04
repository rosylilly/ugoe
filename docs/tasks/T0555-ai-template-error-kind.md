---
id: T0555
title: "AI: テンプレートで組めない失敗に専用の種類とエディタの文言を付ける"
status: in-progress
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

- [ ] テンプレートで組めない失敗が専用の種類で返り、どのテンプレートで何が足りないかを持つ。テストがある
- [ ] エディタの `error.rs` がコード (ADR 0029) で返し、翻訳がある
- [ ] `crates/ugoe-ai/README.md` と `docs/ai-integration.md` を更新した
- [ ] `just ci` が通る

## 参考

- T0547 のメモ (起票の候補)、T0543
- apps/editor/src-tauri/src/error.rs

## メモ

- T0019 で起票した (元の候補: T0547)。
