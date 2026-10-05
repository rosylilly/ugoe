---
id: T0468
title: 見本のモデルにパラメータのグループとポーズを持たせるか決める
status: in-progress
milestone: M9
crates: [ugoe-cli, assets/samples, bindings/web, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0468
---

# T0468: 見本のモデルにパラメータのグループとポーズを持たせるか決める

## 目的

`assets/samples` の見本のモデル (`ugoe sample` が作る) は、パラメータのグループとポーズを持たない。T0463 の `@ugoe/runtime` の
`*_json()` の出力のキーを本物の wasm で比べるテスト (`queries`) は、フィクスチャにグループとポーズを足して確かめている (T0463 の候補)。
見本に持たせれば、アプリ・npm の例・適合テストで同じものを確かめられる。見本の大きさと見た目 (avatar-*.jpg) が変わるので、持たせるかを決める。

## 受け入れ条件

- [ ] 持たせるかの判断と理由をメモに書いた (持たせないなら、ここで閉じる)
- [ ] 持たせるなら: `ugoe sample avatar`・`ugoe sample dress-up` がパラメータのグループとポーズ (例: 腕や手の差分の切り替え) を持つモデルを作り、`assets/samples` を作り直した。assets/samples/README.md を更新した
- [ ] 持たせるなら: `queries` のテストが見本を使う (フィクスチャへの足し込みを減らせるなら減らした)
- [ ] `just ci` が通る

## 参考

- T0463 のメモ (起票の候補)
- assets/samples/README.md、crates/ugoe-cli/src/sample/

## メモ

- T0032 で起票した (元の候補: T0463 (見本のモデル))。
