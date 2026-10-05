---
id: T1608
title: エディタのコマンドで、物理の未知の出力先・対象を受け付けるか拒むかを揃える (判断が要る)
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T1608
---

# T1608: エディタのコマンドで、物理の未知の出力先・対象を受け付けるか拒むかを揃える (判断が要る)

## 目的

T0140 で物理の未知の `VertexOwner` などを読めるようにしたが、エディタのコマンド (`SetPhysicsOutputs`・`CreatePhysicsRig` など) は
`PhysicsBlend::Unknown` だけを拒み、ほかの未知の値 (出力先・対象) は受け付ける。揃っていない (T0140 の候補)。

**判断が要る:** 未知の値をコマンドで受け付ける (未来の版のファイルを開いて編集しても保つ) か、拒む (編集で作らせない) か。

## 受け入れ条件

- [x] 方針を決め、docs/editor.md に書いた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] 物理のコマンドが、未知の値を方針どおりに扱うテストがある (どの未知の種類も同じ扱い)
- [x] `just ci` が通る

## 参考

- T0140 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0140 のエディタのコマンド)。
- 方針: **拒否する** (利用者の判断は待たずに決めた)。根拠:
  - 未知の値は書き出せず、エディタの保存は `FormatError::UnknownValues` で拒否する。コマンドで作れても保存できない文書になるだけ。
  - ほかのコマンドはすでに未知の値を拒否している (モーションの未知の対象・セグメント、表情の未知の op、アバターのプロファイルの未知の値、
    格子の補間・推定の未知の値、物理の出力の `blend`)。物理の残りだけが受け付けていた。
  - Command は JSON から読むので、IPC・MCP から届いた知らない文字列も `#[serde(other)]` で `Unknown` になり、黙って保存できない値を作れていた。
  - 「未来の版のファイルを開いて編集しても保つ」ことは、既存の未知の値をそれを置き換えないコマンドが触らないことで満たされる (T0140 の `forget_removed` の扱いのまま)。
- やったこと
  - `physics_edit.rs` の `check_input`・`check_output`・`check_collider` で、入力・出力の `source`、出力先 `target`、`VertexOwner`、`blend`、衝突体の `shape` の `Unknown` を `EditError::InvalidPhysics` で拒否する
    (`add_physics_rig`・`set_physics_inputs`・`set_physics_outputs`・`add_physics_collider`・`set_physics_collider`)。`set_physics_collider` は従来どおり先に `NotFound` を見る。
  - テスト: 未知の種類ごとに各コマンドが拒否し物理が変わらないこと、JSON の知らない値が拒否されること (変更を外すと失敗するのを確かめた)。
  - docs/editor.md#物理の編集 に「未知の値」の項を足した。
- やらなかったこと
  - `apps/editor` のフロントエンドは変えていない (`Unknown` は IPC に書き出せないので画面からは送られない)。
  - リグ全体を置き換えるコマンドで既存の未知の出力を自動で保つ (未知の値だけを残して差し替える) ことはしない。置き換えるなら未知の値を含まない列を渡す。
- 起票の候補
  - なし。
- `just ci` は rebase 後に通った。
