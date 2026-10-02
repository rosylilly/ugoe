---
id: T0809
title: スタジオ: 仮想カメラ (macOS の Camera Extension)
status: todo
milestone: M8
crates: [apps/studio, docs]
depends_on: [T0807]
assignee:
---

# T0809: スタジオ: 仮想カメラ (macOS の Camera Extension)

## 目的

T0807 の macOS 版。CoreMediaIO の Camera Extension (System Extension) で出す。

## 受け入れ条件

- [ ] Camera Extension と、スタジオからの画の受け渡し (T0807 と同じ形)
- [ ] 署名・公証・entitlement の手順を docs/release.md に書く (配布そのものは Release のマイルストーンで行う)
- [ ] macOS の実機での確認は利用者に残す (手順を書く)
- [ ] `just ci` が通る

## 参考

- T0807 のメモ

## メモ
