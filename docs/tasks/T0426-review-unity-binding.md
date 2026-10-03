---
id: T0426
title: コードレビュー: bindings/unity (UPM パッケージ)
status: in-progress
milestone: M9
crates: [bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0426
---

# T0426: コードレビュー: bindings/unity (UPM パッケージ)

## 目的

bindings/unity (UPM パッケージ) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `bindings/unity` のすべて (com.ugoe.unity の C#・テスト・build.mjs・release.mjs)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - C ABI の宣言 (DllImport) と `ugoe-ffi` のヘッダの一致、ハンドルの解放 (SafeHandle・Dispose・ファイナライザ)
  - マーシャリング (文字列の符号化・配列の長さ・固定)、メインスレッドの前提、毎フレームの GC の確保
  - プラットフォームごとのネイティブライブラリの置き場所と、エディタとビルドでの読み込み
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0443〜T0447 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
