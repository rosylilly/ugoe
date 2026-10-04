---
id: T0443
title: Unity: 実際の Unity エディタとプレイヤーでパッケージを確かめる
status: todo
milestone: M9
crates: [bindings/unity, docs]
depends_on: [T0426]
assignee:
---

# T0443: Unity: 実際の Unity で確かめる

## 目的

T0426 (bindings/unity のレビュー) で、Unity に依存する部分 (`Runtime/Unity/`・`Editor/`・シェーダ・見本) は Unity の上で一度も
動かしていないことを確かめた (確かめたのは `Runtime/Core` の dotnet のテストと、スタブでの型検査だけ)。レビューで直した
`UgoePuppet` の振る舞い (読み込みの失敗からの立ち直り・インスペクタでの `pixelsPerUnit` の変更・着せ替えの順・失われたマスクのページの描き直し)
も、Unity では確かめていない。Unity を持つ人が、`bindings/unity/README.md` の「制限と確かめていないこと」の項目を確かめる。

## 受け入れ条件

- [ ] Unity 2022.3 と Unity 6 の新しいプロジェクトで、パッケージのコンパイルが通る (API 互換レベルは .NET Standard 2.1 と .NET Framework の両方)
- [ ] README の「制限と確かめていないこと」の 1.〜6. と、色空間 (Gamma と Linear の画素の一致) を確かめ、結果を README に書く
- [ ] 壊れた `.ugoemod` を `UgoePuppet` のモジュールに入れても、エラーを 1 回出して本体が表示される
- [ ] 再生中にインスペクタで Pixels Per Unit を変えると、すべてのメッシュが新しい大きさになる
- [ ] 着せ替えで同じスロットのモジュールを差し替えてから、コンポーネントを無効・有効にしても見た目 (合成の順) が変わらない
- [ ] 見つかった不具合を直すか起票する
- [ ] `just ci` が通る

## 参考

- `bindings/unity/README.md`、`docs/tasks/T0426-review-unity-binding.md` のメモ

## メモ
