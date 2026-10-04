---
id: T0453
title: "Unity: UgoePuppet の描き直しの印と LateUpdate の順を Core に出してテストする"
status: todo
milestone: M9
crates: [bindings/unity]
depends_on: []
assignee:
---

# T0453: Unity: UgoePuppet の描き直しの印と LateUpdate の順を Core に出してテストする

## 目的

T0446 で `UgoePuppet` の判断の一部を Unity に依存しない `Runtime/Core` に出し、dotnet のテストで確かめられるようにした。
残りの Unity に依存する状態 (`dirty`・`intermediateDirty`・`lightsDirty` の立て方と、`LateUpdate` の中の順) は Unity の上でしか動かず、
Unity を持たない環境 (CI) では確かめられない (T0443 の実機の確認を待つことになる)。同じ方法で Core に出す。

## 受け入れ条件

- [ ] 印の立て方と `LateUpdate` の順を、Unity の型に依存しない Core のクラスに移した (`UgoePuppet` はそれを呼ぶだけ)
- [ ] dotnet のテストで、パラメータ・光源・中間の段の変更がそれぞれ正しい印を立て、順どおりに処理されることを確かめる
- [ ] `just unity-check` と `just ci` が通る (Unity での確認は T0443)

## 参考

- T0446 のメモ (起票の候補)、T0443
- bindings/unity/README.md

## メモ

- T0019 で起票した (元の候補: T0446)。
