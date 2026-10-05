---
id: T0034
title: just を通らずに cargo を呼ぶスクリプト (tests/conformance/run.mjs など) も mbx を通す
status: done
milestone: M9
crates: [tests/conformance, scripts, docs]
depends_on: []
assignee: claude-worker-T0034
---

# T0034: just を通らずに cargo を呼ぶスクリプト (tests/conformance/run.mjs など) も mbx を通す

## 目的

T0028 で `bindings/godot/build.mjs` は `scripts/lib/mbx.mjs` の `cargoEnvWithMbx` で mbx (ビルドのキャッシュ) の shim を PATH に足すようにした。
`tests/conformance/run.mjs` など、ほかにも just のレシピを通らずに `cargo` を呼ぶスクリプトがあり、mbx の PATH の無いシェル (エージェントの Bash・IDE) から
流すと置き場を使わずにビルドする (T0028 の候補)。

## 受け入れ条件

- [x] `cargo` を子プロセスで呼ぶリポジトリのスクリプト (`tests/`・`scripts/`・`bindings/web/scripts` など。`grep` で探す) を一覧にし、メモに書いた
- [x] 一覧のスクリプトが `cargoEnvWithMbx` (か同じ判定) で mbx を通す。mbx が無い機械では今までどおり動く
- [x] 判定を確かめるテストが `just script-check` にある (既存の `scripts/mbx.test.mjs` に足してよい)
- [x] `just ci` が通る

## 参考

- T0028 のメモ (起票の候補)
- docs/workflow/build-cache.md、scripts/lib/mbx.mjs

## メモ

- T0032 で起票した (元の候補: T0028)。

### やったこと

- `scripts/lib/mbx.mjs` に `exportMbxPath()` を足した。`cargoEnvWithMbx` と同じ判定で、mbx を通すなら process.env の PATH の先頭に shim を足す
  (justfile の `export PATH` と同じ)。`env` を渡さずに起動する子プロセスの cargo も、cargo を呼ぶ pnpm・tauri・cargo-xwin も mbx を通る。
  shim が無い・`MBX_DISABLE` がある・Windows では何も変えない (mbx の無い機械と CI では今までどおり)。
- **cargo を子プロセスで呼ぶスクリプトの一覧** (`"cargo", ...` を spawnSync・execFileSync などに渡すか、`pnpm ... tauri build` を動かすもの)。
  すべて始めに (main のあるものは main の始めに) `exportMbxPath()` を呼ぶようにした:
  - ビルド・検査をするもの: `tests/conformance/run.mjs` (ugoe-cli などの build)、`tests/e2e/run.mjs` (tauri build)、`scripts/app-build.mjs` (tauri build)、
    `scripts/ci-groups.mjs` (`os` の clippy・test)、`scripts/cross-check.mjs` (clippy)、`scripts/ffi.mjs` (ugoe-ffi の build)、`scripts/wasm.mjs`、
    `bindings/web/scripts/build.mjs` (wasm の build)、`bindings/unity/build.mjs` (ugoe-ffi の build)、`apps/editor/scripts/sidecar.mjs` (ugoe-mcp)、
    `apps/studio/scripts/build-vcam.mjs` (cargo xwin)。
  - `cargo metadata` だけのもの: `scripts/check-layers.mjs`・`scripts/check-licenses.mjs`・`scripts/crates.mjs`・`scripts/gpu-test-args.mjs`。
    ビルドはしないが、規則を「cargo を呼ぶなら通す」の 1 つにして検査を単純にするため同じく呼ぶ (just から呼ばれるときは何も変えない)。
  - `bindings/godot/build.mjs` は T0028 で `cargoEnvWithMbx` を使っている (変えていない)。
  - `tests/ui-audit/run.mjs` は `just wasm` などを呼ぶだけ (just が PATH を足す) なので対象外。
- `scripts/mbx.test.mjs` (`just script-check`) に `exportMbxPath` の単体テストと、git の管理下のスクリプトのうち cargo か `tauri build` を
  子プロセスで呼ぶものがすべて `exportMbxPath` か `cargoEnvWithMbx` を呼ぶかの検査を足した (`tests/conformance/run.mjs` の変更を戻すと失敗することを確かめた)。
- `docs/workflow/build-cache.md` の「just を通さずに cargo を直接呼ぶとき」の節と、justfile の script-check のコメントを直した。
- 確かめたこと: mbx の PATH の無いシェルで `node tests/conformance/run.mjs --only queries` を流し、cargo の出力に `mbx[cache]: ... hits` が出て
  (worktree の target が mbx の managed target になり) queries が通った。
- `crates:` の外 (`bindings/web`・`bindings/unity`・`apps/editor/scripts`・`apps/studio/scripts`・`tests/e2e`・justfile のコメント) は、
  1 行の import と呼び出しだけの小さな変更なので行った (受け入れ条件が `bindings/web/scripts` などを挙げているため)。

### やらなかったこと

- Windows・macOS では確かめていない (`exportMbxPath` は Windows では何もしない。macOS の shim の場所は `mbxShimDir` の単体テストだけ)。
- tauri build・cargo-xwin・Unity・e2e を mbx を通して実際に流してはいない (justfile から呼ぶときはすでに shim が PATH にあるので、同じ状態になるだけ)。

### 起票の候補

- なし。
