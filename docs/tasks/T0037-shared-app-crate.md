---
id: T0037
title: "3 つのアプリで写しになっている FileAccess・ダイアログの組み立てを共有する置き場を作るか (判断が要る)"
status: todo
milestone: M9
crates: [ugoe-fs, apps/editor, apps/viewer, apps/studio, Cargo.toml, scripts, docs]
depends_on: [T1620, T1625]
assignee:
---

# T0037: 3 つのアプリで写しになっている FileAccess・ダイアログの組み立てを共有する置き場を作るか (判断が要る)

## 目的

T1616 でエディタもビューア・スタジオと同じく、ファイルのダイアログをバックエンドで開き、選んだパスだけを読み書きできるようにした。
その結果、3 つのアプリ (`apps/*/src-tauri`) に `FileAccess` (読める・1 回だけ書ける)・`with_extension`・`path_string`・
ダイアログの組み立てが写しで並んでいる (T1616 の候補)。

**判断が要る:** 共有の置き場を作るか、作るならどこか。アプリは GPL-3.0-only で、`crates/` は MIT ([ADR 0018](../adr/0018-licensing-and-1.0-scope.md))。
案の例:

- A. 写しのまま (各アプリの差が残る余地を許す)。食い違いを見つける検査だけ足す。
- B. GPL の Rust の共有クレートを `apps/` の下 (例: `apps/shared/`) に作り、3 つのアプリが依存する (ライセンスの検査・依存方向の検査・CI のグループへの登録が要る)。
- C. MIT の `crates/` (例: `ugoe-fs`) に入れる (Tauri に依存する部分は入れられないので、パスの規則だけ)。

`FileAccess` を触る T1620 と T1625 の後に着手する。

## 受け入れ条件

- [ ] 写しの範囲 (関数ごとの差) と案と推奨をメモに書き、利用者の判断を得た
- [ ] 決めた形にした (B・C なら共有して写しを消し、`scripts/check-layers.mjs`・`scripts/ci-groups.mjs`・`just licenses` を通す。A なら理由を docs に)
- [ ] 3 つのアプリの既存のテスト (`access.rs`・`dialog.rs` など) が通る
- [ ] `just ci` が通る

## 参考

- T1616 のメモ (起票の候補・`write_png_file` の共有の検討)
- ADR 0018・ADR 0028、docs/architecture.md

## メモ

- T0036 で起票した (元の候補: T1616 (共有の置き場))。
- 判断 (2026-10-05、利用者の判断): 案 A。`FileAccess` と `with_extension` を MIT の `ugoe-fs` に移し (std だけで書く)、Tauri に依存するダイアログの組み立ては各アプリに残す。
