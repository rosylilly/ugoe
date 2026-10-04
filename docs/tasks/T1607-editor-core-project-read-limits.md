---
id: T1607
title: "editor-core: プロジェクトとモジュールの読み込み・指紋でも、通常のファイルだけを大きさの上限付きで読む"
status: todo
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee:
---

# T1607: editor-core: プロジェクトとモジュールの読み込み・指紋でも、通常のファイルだけを大きさの上限付きで読む

## 目的

T0011 で 3 つのアプリのファイルの読み込みを「通常のファイルだけ・大きさに上限」にしたが、editor-core の `Project::load`・`load_base`・
モジュールの読み込み (`project/module.rs`・`project/files.rs`)・`ProjectFingerprint` (`fingerprint.rs`) は `fs::read` で何でも読む (T0011 の候補)。
FIFO や巨大なファイルを置かれたプロジェクトで止まる・メモリを使い切る。

## 受け入れ条件

- [ ] editor-core のプロジェクト・モジュール・画像の読み込みと指紋が、通常のファイルでないもの (FIFO・デバイス) を拒み、大きさの上限を超えたら読まずにエラーにする
- [ ] 拒んだときのエラーにコードと訳があり (`apps/editor/locales`)、テストがある (FIFO は Unix だけのテストでよい)
- [ ] 上限の値と理由を crates/ugoe-editor-core/README.md (か docs/editor.md) に書いた
- [ ] `just ci` が通る

## 参考

- T0011 のメモ (起票の候補)、apps/viewer/src-tauri/src/read_file.rs (アプリの形)

## メモ

- T0026 で起票した (元の候補: T0011 の editor-core の読み込み)。読み込みの補助を 3 つのアプリと共有するかは T0025 (blocked) の置き場所の判断と関係する。
