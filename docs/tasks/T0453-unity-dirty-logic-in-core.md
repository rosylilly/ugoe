---
id: T0453
title: "Unity: UgoePuppet の描き直しの印と LateUpdate の順を Core に出してテストする"
status: done
milestone: M9
crates: [bindings/unity]
depends_on: []
assignee: claude-worker-T0453
---

# T0453: Unity: UgoePuppet の描き直しの印と LateUpdate の順を Core に出してテストする

## 目的

T0446 で `UgoePuppet` の判断の一部を Unity に依存しない `Runtime/Core` に出し、dotnet のテストで確かめられるようにした。
残りの Unity に依存する状態 (`dirty`・`intermediateDirty`・`lightsDirty` の立て方と、`LateUpdate` の中の順) は Unity の上でしか動かず、
Unity を持たない環境 (CI) では確かめられない (T0443 の実機の確認を待つことになる)。同じ方法で Core に出す。

## 受け入れ条件

- [x] 印の立て方と `LateUpdate` の順を、Unity の型に依存しない Core のクラスに移した (`UgoePuppet` はそれを呼ぶだけ)
- [x] dotnet のテストで、パラメータ・光源・中間の段の変更がそれぞれ正しい印を立て、順どおりに処理されることを確かめる
- [x] `just unity-check` と `just ci` が通る (Unity での確認は T0443)

## 参考

- T0446 のメモ (起票の候補)、T0443
- bindings/unity/README.md

## メモ

- T0019 で起票した (元の候補: T0446)。
- やったこと:
  - `Runtime/Core/ComponentState.cs` に `FrameState` (印: `NeedsLoad`・`Validated`・`Dirty`・`LightsDirty`・`IntermediateDirty`・`IntermediateUsed`) と
    `IFrameHost` (LateUpdate が呼ぶ Unity の側の処理) を足した。印は出来事のメソッド (`Changed`・`LightsChanged`・`InspectorChanged`・
    `PixelScaleChanged`・`Built`・`Evaluated`・`LightsApplied`・`SetIntermediateUsed`・`IntermediateCreated`・`TakeIntermediate`) で立て・下ろし、
    `FrameState.LateUpdate(host)` が順 (インスペクタの変更の反映 → 読み直し → マスクが失われたら印 → 評価 → 光源 → 中間のテクスチャ) を持つ。
    どれも `internal` で、公開 API は変えていない。
  - `UgoePuppet` は 6 つの bool を捨てて `FrameState` を持ち、`IFrameHost` を明示的に実装する (LateUpdate は `frame.LateUpdate(this)` だけ)。
    振る舞いは前と同じにした (`Mathf.Max(dt, 0)` の NaN → 0 も `FrameState.ClampDeltaTime` で同じ。`AfterEvaluate` で印を下ろすのは送る前のまま)。
  - `tests/FrameStateTests.cs` (18 件): 呼ばれた順を記録する偽のホストで、パラメータ・光源・インスペクタ・pixelsPerUnit・有効/無効・
    マスクの喪失・中間のテクスチャの切り替えと喪失のそれぞれが正しい印を立て、LateUpdate の順どおりに処理されることを確かめる。
  - `bindings/unity/README.md` の構成と「制限」に書き足した。
  - `just unity-check` (dotnet test 87 件・UnityCompile の netstandard2.1/net48・release.mjs のテスト) と `just ci` が通った。
- やらなかったこと・確かめていないこと:
  - Unity の上では動かしていない (T0443)。`IFrameHost` の実装 (Unity の物を触る部分) はスタブでの型検査だけ。
- 起票の候補: なし
