---
id: T0037
title: "3 つのアプリで写しになっている FileAccess・ダイアログの組み立てを共有する置き場を作るか (判断が要る)"
status: done
milestone: M9
crates: [ugoe-fs, apps/editor, apps/viewer, apps/studio, Cargo.toml, scripts, docs]
depends_on: [T1620, T1625]
assignee: claude-worker-T0037
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

- [x] 写しの範囲 (関数ごとの差) と案と推奨をメモに書き、利用者の判断を得た
- [x] 決めた形にした (B・C なら共有して写しを消し、`scripts/check-layers.mjs`・`scripts/ci-groups.mjs`・`just licenses` を通す。A なら理由を docs に)
- [x] 3 つのアプリの既存のテスト (`access.rs`・`dialog.rs` など) が通る
- [x] `just ci` が通る

## 参考

- T1616 のメモ (起票の候補・`write_png_file` の共有の検討)
- ADR 0018・ADR 0028、docs/architecture.md

## メモ

- T0036 で起票した (元の候補: T1616 (共有の置き場))。
- 判断 (2026-10-05、利用者の判断): 案 A。`FileAccess` と `with_extension` を MIT の `ugoe-fs` に移し (std だけで書く)、Tauri に依存するダイアログの組み立ては各アプリに残す。
- 写しの範囲 (関数ごとの差。着手時):
  - `FileAccess` の読めるパス: エディタ・スタジオは `HashSet` をこのセッションの間だけ持つ (同じ実装)。ビューアは古い順の `Vec` で
    1000 件までをアプリのデータの `readable-files.json` に覚える (別物)。読んでよいかの規則 (プロジェクトの中・プロジェクトが指す /
    プロファイルにある・最近のプロファイル) とエラーのコードは各アプリで違う。
  - `FileAccess` の書けるパス (1 回だけ): 3 つとも同じ (`HashSet` から除けたら書ける)。
  - `with_extension`: エディタは文字列の末尾が `.<拡張子>` か (画面の `withExtension` と同じ)、ビューア・スタジオは `Path::extension` で見ていた。
    違いは名前が拡張子だけ (`.json`) のときだけ (エディタは付けない、ほかは `.json.json` にする)。
  - `path_string`・ダイアログの組み立て: 3 つとも Tauri に依存する組み立てが別々 (フィルタの渡し方・親のウィンドウ・e2e の差し替え)。
- やったこと (案 A の判断どおり):
  - `ugoe-fs` に `access` モジュールを足した (std だけ。依存は増やしていない): `FileAccess` (読める `grant_readable`・`is_chosen` と、
    1 回だけ書ける `grant_writable`・`take_writable`)、`WritablePaths` (1 回だけ書けるパスだけ)、`with_extension`。テストも移した。
  - エディタ・スタジオの `access::FileAccess` は `ugoe_fs::FileAccess` を包み、アプリの規則とエラーのコードだけを足す形にした。
    ビューアは読めるパスの持ち方 (覚えておく一覧) を残し、書けるパスに `ugoe_fs::WritablePaths` を使う。3 つとも `ugoe_fs::with_extension` を使う。
  - `with_extension` の規則はエディタのもの (末尾の文字列。画面と食い違わない) に揃えた。ビューア・スタジオで名前が `.json` だけの
    ときに `.json.json` にしなくなった (それ以外の振る舞いは同じ)。
  - 文書: `crates/ugoe-fs/README.md`・`docs/architecture.md`・`docs/editor.md`・`apps/viewer/README.md`・`apps/studio/README.md`。
    `scripts/check-layers.mjs`・`scripts/ci-groups.mjs` は `ugoe-fs` がすでに登録済みで、変更は要らなかった。
- やらなかったこと: `path_string` (1 行) とダイアログの組み立ては各アプリに残した (判断の範囲外。`path_string` は IPC の文字列の
  取り決めで、アプリの側の約束のため)。ビューアの読めるパスの一覧 (覚えておく・上限) は共有していない (ビューアだけの規則)。
- 起票の候補: 無し。
