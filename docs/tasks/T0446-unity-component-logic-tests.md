---
id: T0446
title: Unity: UgoePuppet の状態の扱いを Unity 無しでテストできるようにする
status: done
milestone: M9
crates: [bindings/unity, docs]
depends_on: [T0426]
assignee: claude-worker-T0446
---

# T0446: Unity: コンポーネントの状態の扱いのテスト

## 目的

T0426 で見つけた不具合 (読み込みの途中の失敗で毎フレーム例外・インスペクタでの `pixelsPerUnit` の変更が動かないメッシュに効かない・
着せ替えの差し替えで覚えておく順が C ABI と違う) は、どれも `UgoePuppet` (`Runtime/Unity/`) の状態の扱いにあり、テストが無かった
(Unity の部分はスタブでの型検査だけ)。着せ替えの順の規則は `ModuleOrder` (`Runtime/Core`) に出してテストしたが、
読み直すかの判断 (モデル・モジュール・`pixelsPerUnit` の変化)、変更フラグから送り直す範囲 (`Sync`・`SyncBatch`)、マスクを描き直す条件も、
Unity に依存しない形で `Runtime/Core` に寄せて dotnet のテストで確かめる。

## 受け入れ条件

- [x] 読み直すかの判断と、送り直す範囲・マスクを描き直す条件を `Runtime/Core` の型にし、`UgoePuppet` はそれを使う
- [x] それぞれに dotnet のテストがある (少なくとも T0426 の 3 つの不具合を再現する入力)
- [x] `just ci` と `just unity-check` が通る

## 参考

- `bindings/unity/com.ugoe.unity/Runtime/Unity/UgoePuppet.cs`、`docs/tasks/T0426-review-unity-binding.md` のメモ

## メモ

- やったこと:
  - `Runtime/Core/ComponentState.cs` を足し、`UgoePuppet` の判断をそこへ出した (どれも `internal`。公開 API は増やしていない)。
    - `LoadedSource<TModel, TModule>`: 読み込んだモデル・モジュールの並び・`pixelsPerUnit` を覚え、OnValidate の後に読み直すかを決める
      (`loadedModel`・`loadedModules`・`loadedPixelsPerUnit`・`SameModules` を置き換え)。
    - `PixelScale.Sanitize`: `pixelsPerUnit` を正の有限の値にする。NaN と無限大は前の値 (setter) か既定の 100 (OnValidate) にする
      (前は `Mathf.Max` で、無限大が通り、NaN は 1e-3 になっていた)。
    - `PuppetLoad.Run`: 本体を作る → 読めるモジュールを付ける → 準備 (パラメータ・テクスチャ・Build) の手順と、途中の失敗で何を残すか。
      準備の途中の失敗では作りかけの `Puppet` を破棄して null を返す。モジュールを付ける段の失敗は `UgoeException` 以外も捕まえて
      本体だけを表示するようにした (前は `UgoeException` だけで、ほかの例外では作りかけの Puppet が残った)。
    - `BatchUpload`: 変更フラグから、まとめたメッシュ 1 つに送り直す範囲 (要素ごとの頂点・色、メッシュの頂点・色・三角形) を決める。バッチごとに使い回す。
    - `SyncRules`: バッチを作り直す条件、マスクの内容が変わりうるか、マスクを描き直す条件 (Mathf.Approximately と同じ比べ方)。
  - `tests/ComponentStateTests.cs` (23 件)。T0426 の 3 つの不具合を再現する入力: `InspectorPixelsPerUnitChangeReloads`・
    `FailureWhilePreparingLeavesNoPuppet`・`SlotReplacementKeepsThePosition` (と既存の `PuppetTests.RememberedModuleOrderMatchesTheNativeOrder`)。
  - `bindings/unity/README.md` の構成と「制限」に書き足した。
  - `just unity-check` (dotnet test 69 件・UnityCompile の netstandard2.1/net48・.meta 50 件) と `just ci` が通った。
- やらなかったこと・確かめていないこと:
  - Unity の上では動かしていない (T0443)。判断を Unity の物 (メッシュ・マテリアル・子の GameObject) に写す部分は、スタブでの型検査だけ。
  - 3 つの不具合は T0426 で直してあったので、新しいテストは今のコードで通る。前のコードで失敗することは、判断を写した形
    (pixelsPerUnit を比べない・準備の失敗で Puppet を返す・末尾に加える) で考えて確かめたが、前のコードそのものでは走らせていない。
- 起票の候補:
  - `UgoePuppet` の残りの Unity に依存する状態 (`dirty`・`intermediateDirty`・`lightsDirty` の立て方と LateUpdate の順) も、
    必要なら同じように Core に出す (今回は受け入れ条件の範囲だけにした)。
