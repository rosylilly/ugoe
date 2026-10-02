---
id: T0666
title: エディタ: バックエンドのエラーと警告をコードと引数で返し、画面で訳す
status: in-progress
milestone: M8
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0900]
assignee: claude-worker-T0666
---

# T0666: エディタ: バックエンドのエラーと警告をコードと引数で返し、画面で訳す

## 目的

Tauri のコマンドと editor-core が返すエラーや警告は英語の文で、画面で訳せない。コードと引数で返す形にそろえる。

## 受け入れ条件

- [ ] editor-core と Tauri のコマンドのエラー・警告・問題 (ModuleProblem など) に、安定したコードと引数を持たせる (文は英語のまま残し、ログや MCP で使う)
- [ ] ipc.ts と生成した型を更新し (`just gen`)、画面はコードから文言を引く形にできるようにする (訳の付け替えは T0663〜T0665)
- [ ] MCP・CLI の文言は英語のまま変えない
- [ ] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- ADR 0029

## メモ
