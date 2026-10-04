---
id: T1602
title: "editor-core: モジュールの設定の変更と本体の付け替えでも参照の後始末を確かめる"
status: in-progress
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T1602
---

# T1602: editor-core: モジュールの設定の変更と本体の付け替えでも参照の後始末を確かめる

## 目的

T0014 で、要素の削除と ID の書き換えの参照の後始末を、`Document::apply_with_report` の前後の比較で 1 か所にまとめた。
`set_module_settings`・`rebase_module` は別の経路でモデルを組み直す (モジュールの本体を読み直して合成し直す) ので、この後始末を通らない。
今は合成の側で参照が揃うはずだが、要素が消えうる経路なので確かめる。

## 受け入れ条件

- [ ] `set_module_settings`・`rebase_module` で要素が消える場合 (スロットを変える・本体からパーツが消えた) を洗い出し、消えた要素への参照が残らないテストを足した
- [ ] 残る場合は、同じ後始末を通すように直した (先に失敗するテスト)
- [ ] `docs/editor.md#削除の連鎖と拒否` に、この 2 つの経路の扱いを書いた
- [ ] `just ci` が通る

## 参考

- T0014 のメモ (起票の候補)
- crates/ugoe-editor-core/src/document.rs (`set_module_settings`・`rebase_module`)

## メモ

- T0019 で起票した (元の候補: T0014)。
