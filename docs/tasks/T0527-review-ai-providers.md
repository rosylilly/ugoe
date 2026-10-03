---
id: T0527
title: コードレビュー: ugoe-ai (プロバイダと通信)
status: todo
milestone: M9
crates: [ugoe-ai, docs]
depends_on: []
assignee:
---

# T0527: コードレビュー: ugoe-ai (プロバイダと通信)

## 目的

ugoe-ai (プロバイダと通信) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-ai` のプロバイダ抽象・Claude と OpenAI 互換のプロバイダ・transport・turn (自動リギングの本体は T0528)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - API キーなどの秘密がログ・エラー・デバッグ出力に出ないか
  - タイムアウト・再試行・キャンセル、応答の形が想定と違うときの扱い
  - AI が `Command` の列を提案するだけで、モデルを直接書き換えていないか
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
