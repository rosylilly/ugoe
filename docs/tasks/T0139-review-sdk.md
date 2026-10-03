---
id: T0139
title: コードレビュー: ugoe (ランタイム SDK)
status: todo
milestone: M9
crates: [ugoe, docs]
depends_on: []
assignee:
---

# T0139: コードレビュー: ugoe (ランタイム SDK)

## 目的

ugoe (ランタイム SDK) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe` のすべて (Puppet・着せ替え・ホストの操作)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 公開 API の一貫性と doc コメント (crates.io に出す面)
  - 各層 (runtime・physics・motion・render) の呼び順と `dt` の扱い、エラーコード (T0412) の網羅
  - ffi・wasm・各バインディングと重複している処理
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
