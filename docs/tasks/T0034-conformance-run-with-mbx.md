---
id: T0034
title: just を通らずに cargo を呼ぶスクリプト (tests/conformance/run.mjs など) も mbx を通す
status: in-progress
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

- [ ] `cargo` を子プロセスで呼ぶリポジトリのスクリプト (`tests/`・`scripts/`・`bindings/web/scripts` など。`grep` で探す) を一覧にし、メモに書いた
- [ ] 一覧のスクリプトが `cargoEnvWithMbx` (か同じ判定) で mbx を通す。mbx が無い機械では今までどおり動く
- [ ] 判定を確かめるテストが `just script-check` にある (既存の `scripts/mbx.test.mjs` に足してよい)
- [ ] `just ci` が通る

## 参考

- T0028 のメモ (起票の候補)
- docs/workflow/build-cache.md、scripts/lib/mbx.mjs

## メモ

- T0032 で起票した (元の候補: T0028)。
