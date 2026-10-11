---
id: T1006
title: "Godot を 4.7.2 に上げる (godot-check・CI のヘッドレスの Godot・GDExtension の互換)"
status: done
milestone: Release
crates: [bindings/godot, scripts, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1006
---

# T1006: Godot を 4.7.2 に上げる

## 目的

`just tool-updates` (T1004) が Godot 4.7.2-stable を知らせた。利用者の決定 (2026-10-12): 上げる。

## 受け入れ条件

- [x] 固定している Godot の版と SHA512 (T1004 の `PINNED_TOOLS` の写しをすべて) を 4.7.2-stable にした。sha512 は公式の配布から取り直した
- [x] godot-rust の対応する API の版 (`api-4-x` の feature など) と `.gdextension` の `compatibility_minimum` を確かめ、要るなら直した (下げられる最小の版の方針を README に書く)
- [x] `just godot-check` (ヘッドレスの smoke)・`just conformance --only godot`・`node scripts/check-licenses.mjs --godot` が通る
- [x] bindings/godot/README.md と docs/release.md の版の記述を直した
- [x] `just ci` が通る

## 参考

- T1004 のメモ、bindings/godot (build.mjs・Cargo.toml・example)、ADR 0026、docs/release.md#固定した道具の版

## メモ

- 利用者の決定 (2026-10-12) を受けてオーケストレーターが起票した。
- やったこと:
  - `release-checks.yml` の `GODOT_VERSION` を 4.7.2-stable に、`GODOT_SHA512` を公式の Release (godotengine/godot の
    4.7.2-stable、2026-08-18 公開) の `SHA512-SUMS.txt` の `Godot_v4.7.2-stable_linux.x86_64.zip` の値にした。zip を落として
    `sha512sum` が一致するのも確かめた。`PINNED_TOOLS` の Godot には写しが無かったので、`bindings/godot/README.md` の
    「版を固定した Godot (…)」(2 か所) を `mirrors` に足した (`just tool-updates --list` と単体テストが食い違いに気づく)。
  - godot-rust は 0.5.5 (crates.io の最新) のまま。feature は `api-4-4`、`compatibility_minimum = 4.4` のまま変えない
    (GDExtension の API は新しい版の Godot でも使える。4.7.2 の上で `API v4.4 … runtime v4.7.2` と初期化されるのを見た)。
    下限の方針を README の「対応する Godot の版」に書き、docs/release.md の「版を上げる」の Godot の行からそこへ案内した。
  - README・build.mjs の 4.7.1 の記述を直した: CI の版、Godot のソースへのリンク (png_driver_common.cpp・texture_loader_ktx.cpp は
    4.7.1-stable と 4.7.2-stable の間で変わっていないのを GitHub の compare で確かめて 4.7.2-stable に)、初回の `--import` の
    異常終了 (4.7.2 でも `.godot/` を消して 3 回とも exit 134 で起きたので「4.7.1・4.7.2」に。やり直しは残す)。
    bench.gd の値の節の「Godot 4.7.1-stable」は測った時の記録なので残した。
  - 手元で公式の 4.7.2-stable を `GODOT` に指して確かめた: `just godot-check` (smoke と適合テスト 69/69)、
    `just conformance --only godot` (69/69)、lavapipe の Forward+ (`--godot-renderer forward_plus`) も 69/69、
    `node scripts/check-licenses.mjs --godot` (52 の依存)。`just ci` が通った。
- やらなかったこと: CI の `godot` のジョブ (release-checks.yml) は手で起動していない (push しないため。マージの後に流す)。
  下限の 4.4 の Godot での動作は確かめていない (前から CI に無い)。
