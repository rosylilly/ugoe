---
id: T0446
title: Unity: UgoePuppet の状態の扱いを Unity 無しでテストできるようにする
status: in-progress
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

- [ ] 読み直すかの判断と、送り直す範囲・マスクを描き直す条件を `Runtime/Core` の型にし、`UgoePuppet` はそれを使う
- [ ] それぞれに dotnet のテストがある (少なくとも T0426 の 3 つの不具合を再現する入力)
- [ ] `just ci` と `just unity-check` が通る

## 参考

- `bindings/unity/com.ugoe.unity/Runtime/Unity/UgoePuppet.cs`、`docs/tasks/T0426-review-unity-binding.md` のメモ

## メモ
