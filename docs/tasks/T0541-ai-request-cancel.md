---
id: T0541
title: AI のリクエストのキャンセルと全体の時間の上限
status: in-progress
milestone: M9
crates: [ugoe-ai, editor, docs]
depends_on: [T0527]
assignee: claude-worker-T0541
---

# T0541: AI のリクエストのキャンセルと全体の時間の上限

## 目的

T0527 のレビューの所見。`Provider::complete_json`・`DepthEstimator::estimate` は同期で、途中で止める手段がない。
1 回のリクエストの上限は 600 秒 (`ClaudeConfig::timeout`・`OpenAiConfig::timeout`) で、再試行 (既定 2 回、`retry-after` は 60 秒まで待つ) と、
OpenAI 互換の問い直し (`json_retries`)、リギングの問い直しが重なると、最悪で数十分かかる。エディタは `spawn_blocking` で待つだけで、利用者は止められない。

利用者が提案の作成を取り消せるようにし、1 回の提案にかける時間の合計にも上限を設ける。

## 受け入れ条件

- [ ] 取り消しの手段を決める (例: `Provider` に渡す取り消しの印 (`Arc<AtomicBool>` など) を、再試行の待ち (`sleep`) と問い直しの間で確かめる。通信中の取り消しは `Transport` の時間の上限で抑える)。公開 API の変更なので、ADR が要るかを判断する
- [ ] 取り消したときは専用のエラー (`AiError::Cancelled` など) を返し、エディタのエラーの文言 (ADR 0029 のコード) を足す
- [ ] 1 回の提案の時間の合計の上限 (再試行と問い直しを含む) を設定に持つ
- [ ] エディタの AI の操作に「取り消す」を付ける (見た目と配置は M10 の範囲に合わせる)
- [ ] 取り消しと上限のテスト (`ReplayTransport` と差し替えた `sleep` で、実際には待たない)
- [ ] `docs/ai-integration.md` に書く
- [ ] `just ci` が通る

## 参考

- docs/ai-integration.md (Claude API のプロバイダ・OpenAI 互換の API)
- crates/ugoe-ai/src/transport.rs (`send_with_retry`)
- apps/editor/src-tauri/src/ai.rs (`blocking`)

## メモ
