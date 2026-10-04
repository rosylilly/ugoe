---
id: T0560
title: AI の法線マップとテクスチャの追加・差し替えで、PNG の一辺の上限を入口で確かめる (判断が要る)
status: in-progress
milestone: M9
crates: [ugoe-ai, ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0560
---

# T0560: AI の法線マップとテクスチャの追加・差し替えで、PNG の一辺の上限を入口で確かめる (判断が要る)

## 目的

T0162 の残り (T0162 の候補)。

- `ugoe-ai::normal` は `decode_rgba_png` を使い、上限を超えた PNG を「読めない」と同じに扱う。`ugoe_format::image::decode_png` に替え、上限の超過を区別して伝える。
- editor-core のテクスチャの追加・差し替え (`add_texture`・`set_texture_image`、`crates/ugoe-editor-core/src/document/texture_edit.rs`) は、一辺が上限を超える
  PNG を受け入れ、画素が要る処理で初めてエラーになる。

**判断が要る:** テクスチャの追加・差し替えで上限を超える PNG を入口で拒むか (拒むと、今まで開けたプロジェクトに足せなくなる画像がある)。

## 受け入れ条件

- [ ] `ugoe-ai::normal` が `decode_png` を使い、上限の超過が別のエラーになるテストがある
- [ ] テクスチャの入口で拒むかを決め (利用者の判断を待つなら blocked にしてメモに案を書く)、拒むならコマンドのエラーとコードと訳があり、テストがある
- [ ] `just ci` が通る

## 参考

- T0162 のメモ (起票の候補)、crates/ugoe-format/src/image.rs

## メモ

- T0026 で起票した (元の候補: T0162 の 2 つ)。
