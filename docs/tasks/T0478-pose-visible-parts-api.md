---
id: T0478
title: "wasm・C ABI・@ugoe/runtime に表示中のポーズの要素を読む口を足す"
status: todo
milestone: M9
crates: [ugoe-wasm, ugoe-ffi, bindings/web, bindings/unity, docs]
depends_on: []
assignee:
---

# T0478: wasm・C ABI・@ugoe/runtime に表示中のポーズの要素を読む口を足す

## 目的

モーションがパーツの不透明度のカーブでポーズの要素を要求すると、描く姿はモーションに従うが、ホストは今どの要素が表示されているかを
読めない (`show_pose` で書く口と `pose_json` の定義だけ)。Rust の SDK には `Puppet::pose()` の `PosePlayer::visible_parts` がある。
ビューア・エディタのプレビューのポーズの欄をモーションに追従させる (T0736) ために、ランタイムの各入口に読む口を足す (T0735 の候補)。

## 受け入れ条件

- [ ] wasm に表示中の要素を読む口 (例: `pose_visible_json`。グループごとの表示中のパーツ。モジュールのグループを含む) を足し、テストで `show_pose` とモーションによる切り替えの両方を確かめる
- [ ] C ABI に同じ口を足し (ヘッダを作り直す。足すだけなので ABI の版の扱いを crates/ugoe-ffi/README.md の規則で決める)、テストがある。Unity の宣言も足す (`just refs` の過不足の検査)
- [ ] `@ugoe/runtime` の型 (`WebPuppet` の宣言)・README を直した。Godot に足すかはメモに書く (足さないなら起票の候補)
- [ ] docs/spec/motion.md (または対応するランタイムの文書) と英語の spec の該当の節に、読む口があることを書く必要があるか確かめ、要るなら直した
- [ ] `just wasm-check`・`just ffi-check`・`just unity-check`・`just ci` が通る

## 参考

- T0735 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-motion/src/pose.rs (`visible_parts`)、crates/ugoe-wasm/src/animation.rs (`show_pose`・`pose_json`)、crates/ugoe-ffi

## メモ

- T0038 で起票した (元の候補: T0735 (表示中の要素を読む口))。ビューアとエディタの欄の追従は T0736。
