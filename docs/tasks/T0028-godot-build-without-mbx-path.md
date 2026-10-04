---
id: T0028
title: bindings/godot の build.mjs を mbx の PATH なしで流しても target/ に書けるようにする
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0028
---

# T0028: bindings/godot の build.mjs を mbx の PATH なしで流しても target/ に書けるようにする

## 目的

`bindings/godot/build.mjs` を mbx の PATH を通さずに流すと、mbx の置き場からのハードリンク (読み取り専用) を上書きしようとして
target/ に書けずに失敗する (T0456 の候補)。ワークスペースの外 (ADR 0026) なので just のレシピの環境を通らないことがある。

## 受け入れ条件

- [x] `build.mjs` が mbx の有無を見て cargo を呼ぶ (mbx を通すか、通さないときは別の target ディレクトリを使う) か、手順を docs/workflow/build-cache.md と bindings/godot/README.md に書いた (どちらにしたかと理由をメモに書いた)
- [x] mbx の PATH なしのシェルで `node bindings/godot/build.mjs` が通ることを確かめた
- [x] `just ci` が通る

## 参考

- T0456 のメモ (起票の候補)、docs/workflow/build-cache.md、ADR 0026

## メモ

- T0026 で起票した (元の候補: T0456 の mbx)。

### やったこと

- **build.mjs で mbx の有無を見る方を採った** (手順を書くだけにしなかった)。理由: 失敗は「PATH に shim が無いシェルで直接呼んだ」だけで起き、
  エージェントや人が `node bindings/godot/build.mjs` と打つことは README や過去のタスクの検査の手順にある。手順を書くだけでは同じ失敗が繰り返される。
- `scripts/lib/mbx.mjs` に `mbxShimDir` (justfile の `mbx_shim_dir` と同じ場所) と `cargoEnvWithMbx` を足した。shim が入っていて
  `MBX_DISABLE` が無ければ、justfile と同じく PATH の先頭に shim を足した環境を返す。`scripts/mbx.test.mjs` にテストを足した。
- `bindings/godot/build.mjs`: cargo を上の環境で呼ぶ。`MBX_DISABLE` で mbx を通さないのに mbx が入っているか、ルートの `target` が
  シンボリックリンク (managed target) なら、`bindings/godot/target/` (gitignore 済み) に分けてビルドする。`CARGO_TARGET_DIR` があればそれを使う。
  `--conformance` で呼ぶ `tests/conformance/run.mjs` にも同じ環境と `CARGO_TARGET_DIR` を渡す (参照を描く ugoe のクレートのビルドも同じ失敗をしうるため)。
- `bindings/godot/README.md` と `docs/workflow/build-cache.md` に書いた。
- `scripts/` (`crates:` の外) を触ったのは小さな変更として行った (判定の関数を `scripts/lib/mbx.mjs` に置き、`scripts/worktree.mjs` と同じ所で保つため)。

### 確かめたこと (Linux・ext4・WSL2、mbx 1.22.0)

- 直す前: worktree で mbx を通してビルドした後、PATH に shim の無いシェル (`cargo` = /snap/bin/cargo) で `node bindings/godot/build.mjs` が
  `output file .../liblibloading-....rmeta is not writeable` で失敗することを再現した。
- 直した後: 同じシェルで `node bindings/godot/build.mjs` が mbx を通って通る (61 hits, 0 misses)。`MBX_DISABLE=1` では
  `bindings/godot/target/` に分けてビルドして通る (初回 約 3 分)。`node bindings/godot/build.mjs --check --conformance` も同じシェルで通る
  (ライセンス ok、smoke ok、適合テスト 63 / 63、ktx2 ok)。
- `just ci`: 通る。

### やらなかったこと

- mbx が入っていない機械の動き (従来どおりルートの `target/` を共有) は単体テストだけで確かめた。macOS の shim の場所
  (`~/Library/Application Support/mbx/bin`) は justfile の `data_local_directory()` に合わせたが、実機では確かめていない。
- `tests/conformance/run.mjs` を単独で (build.mjs を通さずに) PATH の shim なしで呼んだときの同じ失敗は直していない。

### 起票の候補

- `tests/conformance/run.mjs` など、ほかの just を通らずに cargo を呼ぶスクリプトも `cargoEnvWithMbx` で mbx を通す。
