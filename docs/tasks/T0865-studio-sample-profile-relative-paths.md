---
id: T0865
title: "スタジオ: 見本のプロファイルを、見本のモデルと同じフォルダに相対パスで書いてコミットする"
status: todo
milestone: M8
crates: [apps/studio, assets, scripts, docs]
depends_on: []
assignee:
---

# T0865: スタジオ: 見本のプロファイルを相対パスでコミットする

## 目的

プロファイルはモデルを相対パスでも持てるようになった (T0852。プロファイルのフォルダからの相対。`..` は読まない)。ところが見本のプロファイル
(`pnpm --filter @ugoe/studio sample-profile`・`just check-pack` の `ugoe-sample-profile.json`) は、出力先がモデルのフォルダの外なので
今も絶対パスで書き、機械ごと (WSL・Windows) に作り直す必要がある (docs/manual-check.md の「プロファイルはモデルを絶対パスで持つので…」)。
見本のプロファイルを `assets/samples/` の中に相対パスで置いてコミットし、どの機械でもそのまま開けるようにする。

## 受け入れ条件

- [ ] 見本のプロファイルを `assets/samples/` (見本のモデルと同じフォルダ) に、モデル・モジュール・背景を相対パスで書いてコミットした
      (ファイル名は docs/manual/studio.md#見本のプロファイル に合わせる)。スタジオの読み込み (T0852 の規則) で開けることをテストで確かめた
- [ ] `apps/studio/scripts/sample-profile.mjs` は、コミットしたファイルを作る (か検査する) 形にし、コミットしたものと作ったものが食い違えば
      テスト (sample-profile.test.mjs) が落ちる。絶対パスで書き出す引数を残すかは作業者が決め、残さないなら呼んでいる所 (docs) を直した
- [ ] `just check-pack` (scripts/check-pack.mjs) は、見本のプロファイルを `samples\` の中にそのまま写す (絶対パスに書き直さない)。
      `ugoe-check` のフォルダを動かしても作り直さずに開ける
- [ ] docs/manual-check.md (作り方・Linux と macOS の手順・「フォルダーを動かしたら作り直す」の行)・docs/manual/studio.md#見本のプロファイル・
      apps/studio/README.md を今の形に直した
- [ ] `just ci` が通る

## 参考

- T0852 のメモ (起票の候補)、docs/studio.md (プロファイルの相対パス)、docs/adr/0033-studio-profile-relative-paths.md
- apps/studio/scripts/sample-profile.mjs・sample-profile.test.mjs、scripts/check-pack.mjs、assets/samples/README.md

## メモ

- T0077 で起票した (元の候補: T0852 (見本のプロファイルを見本のモデルと同じフォルダに相対パスでコミットできる形にする))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- `assets/` に置くファイルは小さいもの (JSON 1 つ) なので、AGENTS.md の「小さいものだけ」に収まる。
