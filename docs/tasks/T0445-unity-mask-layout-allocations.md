---
id: T0445
title: Unity: マスクのページの配置と描画で毎フレーム確保しない
status: todo
milestone: M9
crates: [bindings/unity, docs]
depends_on: [T0426]
assignee:
---

# T0445: Unity: マスクの配置の確保を減らす

## 目的

T0426 で、マスクを読む要素が動くフレームごとに `UgoeMaskRenderer.Render` が `MaskAtlasLayout.Build` を呼び、ページごとの
`List`・内容ごとの `CanvasRegion` の配列・並べ替えの `List` を確保していることを見つけた (マスクのあるモデルでは毎フレーム GC の対象が出る)。
中間のテクスチャの四角形と描画の並びは T0426 で使い回すようにした。マスクの配置も、内容の数が変わらない間はバッファを使い回す。
`MaskAtlasLayout` は公開の型なので、使い回しの形 (既存の `Build` を残すか、置き換えるか) を決めてから直す。

## 受け入れ条件

- [ ] 内容の数と深さが変わらない間、`UgoeMaskRenderer.Render` が配置のために確保しない
- [ ] `tests/OffscreenTests.cs` の配置の検査が通り、使い回しても同じ配置になることを確かめるテストがある
- [ ] `just ci` と `just unity-check` が通る

## 参考

- `bindings/unity/com.ugoe.unity/Runtime/Core/Offscreen.cs`、`Runtime/Unity/UgoeMaskRenderer.cs`

## メモ
