---
id: T1607
title: "editor-core: プロジェクトとモジュールの読み込み・指紋でも、通常のファイルだけを大きさの上限付きで読む"
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T1607
---

# T1607: editor-core: プロジェクトとモジュールの読み込み・指紋でも、通常のファイルだけを大きさの上限付きで読む

## 目的

T0011 で 3 つのアプリのファイルの読み込みを「通常のファイルだけ・大きさに上限」にしたが、editor-core の `Project::load`・`load_base`・
モジュールの読み込み (`project/module.rs`・`project/files.rs`)・`ProjectFingerprint` (`fingerprint.rs`) は `fs::read` で何でも読む (T0011 の候補)。
FIFO や巨大なファイルを置かれたプロジェクトで止まる・メモリを使い切る。

## 受け入れ条件

- [x] editor-core のプロジェクト・モジュール・画像の読み込みと指紋が、通常のファイルでないもの (FIFO・デバイス) を拒み、大きさの上限を超えたら読まずにエラーにする
- [x] 拒んだときのエラーにコードと訳があり (`apps/editor/locales`)、テストがある (FIFO は Unix だけのテストでよい)
- [x] 上限の値と理由を crates/ugoe-editor-core/README.md (か docs/editor.md) に書いた
- [x] `just ci` が通る

## 参考

- T0011 のメモ (起票の候補)、apps/viewer/src-tauri/src/read_file.rs (アプリの形)

## メモ

- T0026 で起票した (元の候補: T0011 の editor-core の読み込み)。読み込みの補助を 3 つのアプリと共有するかは T0025 (blocked) の置き場所の判断と関係する。

### やったこと (T1607)

- `src/project/read.rs` に `read_regular_file(path, max)` を置いた (3 つのアプリの `read_file.rs` と同じ形: 開く前に `fs::metadata` で
  通常のファイルか (リンクは辿る) と大きさを見て、開いた後にも見直し、上限 + 1 バイトまでに限って読む)。共有の置き場所は作らず editor-core の中。
- 上限は読む側の設定 `ProjectReadLimits` (既定: JSON 1 つ 256 MiB・画像 1 つ 512 MiB・本体の `.ugoe` 512 MiB。`with_max_*`)。
  一般の入り口 `Project::load_with_limits`・`load_base_with_limits`・`ProjectFingerprint::reread_with_limits`・`reread_base_with_limits` を足し、
  既存の関数 (`load`・`load_checking_base`・`load_with_base`・`load_module_with`・`load_tracked`・`load_base*`・指紋の `read*`/`reread*`・
  `read_module_link`) は既定の上限で読む。呼び出し側 (エディタ・CLI・MCP) は変えていない (既定のまま)。
- 対象: `project.json`・`model.json`・`module.json`・`physics.json`・モーション・表情・`textures/*.png`・本体 (プロジェクトと `.ugoe`)・指紋。
  `project.json` が FIFO のときも「プロジェクトでない」ではなく `NotRegularFile` にした (`exists` で見る)。画像が FIFO のときも `MissingTexture` でなく `NotRegularFile`。
  指紋は前は通常のファイルでない固定名のファイルを黙って飛ばしていたが、読み込みと揃えて拒む (ディレクトリから集めるものは前から通常のファイルだけ)。
- 保存 (`write_if_changed`) が今のファイルと比べるために `fs::read` していたので、通常のファイルで同じ大きさのときだけ読むようにした (FIFO を置かれると保存で止まっていた)。
- エラー: `ProjectError::NotRegularFile { path }` (`project.not_regular_file`、`path`)・`ProjectError::FileTooLarge { path, len, max }`
  (`project.file_too_large`、`path`・`size`・`max`)。見本 (`message_catalog.rs`)・`apps/editor/locales` (en・ja)・`messageCodes.ts` (`just gen`)・docs/editor.md の表。
- テスト `src/read_limits_tests.rs`: 上限 (JSON・画像・本体の `.ugoe`。ちょうど上限なら読める。確かめて替えたパスでもエラーは元のパス)・ディレクトリ・コードと引数、
  Unix だけで FIFO (`model.json`・`project.json`・`module.json`・本体の `model.json`・本体の `.ugoe`)・`/dev/zero` へのリンク (画像・本体)・FIFO の上への保存。
- docs/editor.md に「読むファイルの種類と大きさ」(値と理由) を書き、README に載せた。

### やらなかったこと・確かめていないこと

- 先に失敗するテストは書いていない (新しい API を使うので前のコードでは作れない。FIFO のテストは前のコードなら止まる)。
- 見てから開くまでの間に FIFO へ差し替えられると止まりうる (アプリの写しと同じく受け入れた。`O_NONBLOCK` には unsafe か libc が要る)。
- Windows・macOS では動かしていない (FIFO・装置のテストは `cfg(unix)`)。
- T0025 との関係: 利用者の判断 (main 3d93c82e) で T0025 は共有のクレート `ugoe-fs` を作ることになった。この `read.rs` は 3 つのアプリの
  `read_file.rs` と同じ形の 4 つ目の写しなので、T0025 (か後続) で `ugoe-fs` に寄せる (`ProjectError` への変換だけを editor-core に残す)。

### 起票の候補

- T0025 の後で、editor-core の `project/read.rs` を `ugoe-fs` の読み込みに置き換える (T0025 の範囲に入れてもよい)。
- エディタ・MCP で上限を設定として持つか (今はどちらも既定の値。必要になったら `ProjectReadLimits` を渡す)。
