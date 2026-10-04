---
id: T1602
title: "editor-core: モジュールの設定の変更と本体の付け替えでも参照の後始末を確かめる"
status: done
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

- [x] `set_module_settings`・`rebase_module` で要素が消える場合 (スロットを変える・本体からパーツが消えた) を洗い出し、消えた要素への参照が残らないテストを足した
- [ ] 残る場合は、同じ後始末を通すように直した (先に失敗するテスト)
- [x] `docs/editor.md#削除の連鎖と拒否` に、この 2 つの経路の扱いを書いた
- [x] `just ci` が通る

## 参考

- T0014 のメモ (起票の候補)
- crates/ugoe-editor-core/src/document.rs (`set_module_settings`・`rebase_module`)

## メモ

- T0019 で起票した (元の候補: T0014)。

### やったこと

- 2 つの経路を読んだ。どちらも作業用のモデルを差分で書き換えず、`ModuleContext::split` でモジュールに分けてから
  `ModuleContext::workspace` (`ugoe_runtime::compose`・`compose_physics`) で本体と合成し直す。合成は本体の複製とモジュールの中の参照
  (接頭辞を付け替えたもの) だけからモデルと物理を作り、合成したモデルを検証するので、消えた要素への参照は残らない。
  モジュールの外のデータ (モーション・表情) はモジュールのプロジェクトでは空 (分けるときに拒否される)。
- 要素が消える場合の洗い出し:
  - `set_module_settings` の名前の変更: モジュールの要素 (物理のリグ・衝突体を含む) の ID がすべて `<旧名>/…` から `<新名>/…` に替わる。
  - `set_module_settings` のスロットの変更 (`keep_look` の有無とも): 前のスロットの `replaces` を隠すパーツ `#replaced/…` が消える。
  - `rebase_module`: 本体のスロットが置き換えるパーツを変えた (`#replaced/…` が消える)・本体から要素が消えた。
  - `rebase_module_on` のスロットの付け替え: `#replaced/…` が替わる。
  - モジュールが参照する本体の要素 (マスクの元・公開されたパラメータ・衝突体・ポーズの要素・スロット) が消えた場合は、合成がエラーになり何も変えない。
- テスト (`module_project_tests`): `module_settings_leave_no_references_to_removed_elements`・`rebase_leaves_no_references_to_removed_elements`。
  合成し直した後のモデルと物理に、前後の比較の後始末 (`cleanup::forget_removed`) を通しても何も変わらないこと、`validate`・`validate_physics` を通ること、
  消えた ID を実際に確かめる。フィクスチャの LongHair にメッシュの対 (`mirror`)・ポーズのグループ・物理を持たせ、参照の種類を増やした。
  マスクの元 (`Face`) や衝突体 (`Shoulders`) が本体から消えたときは `rebase_module` がエラーで、状態を変えないことも確かめた。
- `docs/editor.md#削除の連鎖と拒否` に、この 2 つの経路が後始末を通らない理由と扱いを書いた。

### やらなかったこと

- 2 つ目の受け入れ条件 (残る場合に同じ後始末を通す) は、参照が残る場合が見つからなかったので該当なし (コードは変えていない)。

### 起票の候補

- なし。

### just ci

- `git rebase main` の後に `just ci` が通った。
