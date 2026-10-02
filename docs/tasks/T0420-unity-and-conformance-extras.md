---
id: T0420
title: Unity と適合テスト: Linear の正確な一致・アトラス化した見本・Godot の頂点の比較
status: todo
milestone: M7
crates: [bindings/unity, bindings/godot, tests/conformance, ugoe-cli, assets, apps/editor, apps/viewer, docs]
depends_on: [T0419, T0418]
assignee:
---

# T0420: Unity と適合テストの残り

## 目的

T0419・T0418 の後続の候補をまとめる。

## 受け入れ条件

- [ ] Unity の Linear の色空間で、中間の RenderTexture を使って参照と正確に合わせる (今は Gamma を要求)
- [ ] アトラス化した見本を同梱する (バッチングが効く見本。法線マップ付きのテクスチャのアトラス化も検討)。マスクの内容のアトラス
- UPM の tarball を GitHub Release に添付するのは、公開のマイルストーン (Release) の T0421 に移した
- [ ] Godot から頂点を読み戻して比べる。Godot に JSON のモデルを読ませる
- [ ] ビューアとエディタのスクリーンショットに wasm の read_pixels を使う
- [ ] `just ci`・`just conformance`・`just unity-check`・`just godot-check` が通る

## 参考

- `docs/tasks/T0419-unity-followups.md`・`docs/tasks/T0418-conformance-more-backends.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
