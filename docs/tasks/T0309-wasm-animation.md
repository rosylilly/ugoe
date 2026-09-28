---
id: T0309
title: wasm: 物理・モーション・表情・エフェクトの公開
status: todo
milestone: M3
crates: [ugoe-wasm, ugoe]
depends_on: [T0303, T0305]
assignee:
---

# T0309: wasm: 物理・モーション・表情・エフェクトの公開

## 目的

`WebPuppet` から、`Puppet::update` と同じ順で物理・モーション・表情・目パチ・呼吸・リップシンク・ポーズを動かせるようにする。
エディタの物理の設定画面 (T0629) とタイムライン (T0630)、ビューア (T07xx) のプレビューに使う。

## 受け入れ条件

- [ ] モーション・表情を JSON で渡して再生・停止できる。物理と各エフェクトを有効・無効にできる。リップシンクの入力値を渡せる
- [ ] 物理の設定だけを差し替えられる (エディタで設定を変えながらプレビューするため)
- [ ] 手で動かしたパラメータ (スライダー) と、モーション・物理の出力の関係を決めて書く
- [ ] TS の型 (`ugoe-wasm` の生成物) と、ネイティブでの単体テスト
- [ ] `just ci`・`just wasm-check`・`just wasm` が通る

## 参考

- `docs/spec/motion.md`、`docs/spec/physics.md`
- `crates/ugoe-wasm/src/web.rs`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
