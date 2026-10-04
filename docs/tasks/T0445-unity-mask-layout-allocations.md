---
id: T0445
title: Unity: マスクのページの配置と描画で毎フレーム確保しない
status: done
milestone: M9
crates: [bindings/unity, docs]
depends_on: [T0426]
assignee: claude-worker-T0445
---

# T0445: Unity: マスクの配置の確保を減らす

## 目的

T0426 で、マスクを読む要素が動くフレームごとに `UgoeMaskRenderer.Render` が `MaskAtlasLayout.Build` を呼び、ページごとの
`List`・内容ごとの `CanvasRegion` の配列・並べ替えの `List` を確保していることを見つけた (マスクのあるモデルでは毎フレーム GC の対象が出る)。
中間のテクスチャの四角形と描画の並びは T0426 で使い回すようにした。マスクの配置も、内容の数が変わらない間はバッファを使い回す。
`MaskAtlasLayout` は公開の型なので、使い回しの形 (既存の `Build` を残すか、置き換えるか) を決めてから直す。

## 受け入れ条件

- [x] 内容の数と深さが変わらない間、`UgoeMaskRenderer.Render` が配置のために確保しない
- [x] `tests/OffscreenTests.cs` の配置の検査が通り、使い回しても同じ配置になることを確かめるテストがある
- [x] `just ci` と `just unity-check` が通る

## 参考

- `bindings/unity/com.ugoe.unity/Runtime/Core/Offscreen.cs`、`Runtime/Unity/UgoeMaskRenderer.cs`

## メモ

- **形の決定:** `Build` は残し (同じ結果・同じ契約)、インスタンスの `MaskAtlasLayout.Update(bounds, depths, scale, maxSize)` を足した。
  内容の数とページの数が前と同じなら `Regions`・`PageOf`・`PageSizes` と作業用の配列を使い回して同じ配列に書き直す。
  `Build` は新しい配置を作って `Update` を呼ぶだけ。追加だけなので既存の利用者は壊れない (パッケージは 0.0.0 で未公開)。
  置き換え (`Build` を消す) は、使い回さない用途 (テスト・1 回だけの配置) で不便になるので選ばなかった。
- 並べ替えは `List.Sort` とラムダ (閉包の確保) をやめて、作業用の配列の挿入ソートにした。比較は「高い順、同じなら元の順」の全順序なので、結果は変わらない。
- 収まらないまま 64 回試し終えたページの大きさは、以前と同じ `(0, 0)` (使い回すので明示的に戻す)。
- `UgoeMaskRenderer` は配置を 1 つ持ち、2 回目からは `Update` で並べ直す (内容の数と深さは `MaskPlan` で決まり変わらない)。
- テスト (`tests/OffscreenTests.cs`): `UpdatedMaskAtlasMatchesAFreshBuild` (高さの順の入れ替わり・倍率の下げ・内容とページの数の変化を通して、
  並べ直した配置が同じ引数の `Build` と一致)、`UpdatingMaskAtlasDoesNotAllocate` (`GC.GetAllocatedBytesForCurrentThread` で 100 回の
  `Update` の確保が 0 で、配列が同じもの)。既存の配置の検査も通る。README の「マスク」の節に使い回しを 1 文足した。
- **確かめていないこと:** Unity の上では動かしていない (環境に Unity が無い)。`UgoeMaskRenderer` の変更はスタブの型検査 (netstandard2.1・net48) だけ。
  Unity の Mono / IL2CPP での確保は、Core の配置が確保しないことを dotnet で確かめた範囲で、`Render` 全体 (CommandBuffer・MaterialPropertyBlock など
  Unity の API の内部) の確保は Profiler で見ていない (T0443 で見るとよい)。
- `just unity-check` (dotnet test 46 件・スタブの型検査・release.mjs のテスト) と `just ci` は通った。
- 起票の候補: なし。
