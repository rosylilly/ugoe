---
id: T1621
title: "editor-core: keep_look で名前も変えたとき、check_reparented_pairs が前のモデルと突き合わせられるようにする"
status: done
milestone: M9
crates: [ugoe-editor-core]
depends_on: []
assignee: claude-worker-T1621
---

# T1621: editor-core: keep_look で名前も変えたとき、check_reparented_pairs が前のモデルと突き合わせられるようにする

## 目的

T1609 でモジュールの適用のエラーに直し方の案内を出し、`keep_look` の作り直しの警告を捨てないようにした。
`keep_look` で接頭辞 (名前) も同時に変えると、`check_reparented_pairs` が前のモデルと ID で突き合わせられず、付け替えた組を見逃す (T1609 の候補)。
接頭辞を付け替えてから比べるかを検討し、直す。

## 受け入れ条件

- [x] 名前を同時に変えた `keep_look` で、付け替えた組が前のモデルと突き合わされることを確かめる、先に失敗するテストがある
- [x] 接頭辞を付け替えてから比べる (か、別の方法で突き合わせる) ように直し、テストが通る。採った方法をメモに書いた
- [x] `just ci` が通る

## 参考

- T1609 のメモ (起票の候補)

## メモ

- T0032 で起票した (元の候補: T1609 (keep_look))。
- やったこと:
  - 採った方法: 接頭辞を付け替えて比べる。`inference::check_reparented_pairs` に、後のモデルの要素の前のモデルでの ID を返す
    `previous` を渡す (親の比較・前のモデルでの `check_mirror` に使う)。ふつうの編集では同じ ID (`KeyformTarget::clone`)、
    `set_module_settings` の `keep_look` では `ModuleContext::previous_target` (新しい名前の接頭辞 `<名前>/` を前の名前に付け替える。
    本体の要素 (`is_base_*`・本体のグルー) と接頭辞の無い ID はそのまま)。前のモデルを付け替えて複製するより安く、ID の参照を漏れなく
    書き換える必要もない。
  - 先に失敗するテスト `module_settings_keep_look_checks_reparented_pairs_when_renamed`: 本体の `Torso` を台形 (非線形の双線形) にし、
    最上位のワープ `Frame` の中の `Skirt` と、最上位の `SkirtR` (キャンバスで反転した位置) の対を持つモジュールを `keep_look` で Hair へ移す。
    `Frame` の中の `Skirt` は双線形の補間が変わって数ピクセルずれ、ミラー元が変わらないので作り直し (`rederive`) はミラーし直さず、
    付け替えの確かめだけが警告を出す。名前を変えない (`Dress`) と警告が出て、変える (`Robe`) と出なかった (修正前)。
  - `docs/editor.md#モジュールの設定` の記述を直した。
- 調べて分かったこと: `keep_look` では最上位の要素がすべて一緒に新しい取り付け先へ移るので、対の親が同じか異なるかは前後で変わらず、
  ミラー元が動けば作り直しがミラーし直して警告を出す。付け替えの確かめだけが見つけるのは、ミラー先だけが動き、ミラー元が動かない
  モジュールのデフォーマの中にあって、取り付け先の写像が非線形 (ワープ) で見た目がずれるときに限られる (テストはその形)。
- やらなかったこと: 特になし。
- 起票の候補: なし。
