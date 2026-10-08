---
id: T0865
title: "スタジオ: 見本のプロファイルを、見本のモデルと同じフォルダに相対パスで書いてコミットする"
status: done
milestone: M8
crates: [apps/studio, assets, scripts, docs]
depends_on: []
assignee: claude-worker-T0865
---

# T0865: スタジオ: 見本のプロファイルを相対パスでコミットする

## 目的

プロファイルはモデルを相対パスでも持てるようになった (T0852。プロファイルのフォルダからの相対。`..` は読まない)。ところが見本のプロファイル
(`pnpm --filter @ugoe/studio sample-profile`・`just check-pack` の `ugoe-sample-profile.json`) は、出力先がモデルのフォルダの外なので
今も絶対パスで書き、機械ごと (WSL・Windows) に作り直す必要がある (docs/manual-check.md の「プロファイルはモデルを絶対パスで持つので…」)。
見本のプロファイルを `assets/samples/` の中に相対パスで置いてコミットし、どの機械でもそのまま開けるようにする。

## 受け入れ条件

- [x] 見本のプロファイルを `assets/samples/` (見本のモデルと同じフォルダ) に、モデル・モジュール・背景を相対パスで書いてコミットした
      (ファイル名は docs/manual/studio.md#見本のプロファイル に合わせる)。スタジオの読み込み (T0852 の規則) で開けることをテストで確かめた
- [x] `apps/studio/scripts/sample-profile.mjs` は、コミットしたファイルを作る (か検査する) 形にし、コミットしたものと作ったものが食い違えば
      テスト (sample-profile.test.mjs) が落ちる。絶対パスで書き出す引数を残すかは作業者が決め、残さないなら呼んでいる所 (docs) を直した
- [x] `just check-pack` (scripts/check-pack.mjs) は、見本のプロファイルを `samples\` の中にそのまま写す (絶対パスに書き直さない)。
      `ugoe-check` のフォルダを動かしても作り直さずに開ける
- [x] docs/manual-check.md (作り方・Linux と macOS の手順・「フォルダーを動かしたら作り直す」の行)・docs/manual/studio.md#見本のプロファイル・
      apps/studio/README.md を今の形に直した
- [x] `just ci` が通る

## 参考

- T0852 のメモ (起票の候補)、docs/studio.md (プロファイルの相対パス)、docs/adr/0033-studio-profile-relative-paths.md
- apps/studio/scripts/sample-profile.mjs・sample-profile.test.mjs、scripts/check-pack.mjs、assets/samples/README.md

## メモ

- T0077 で起票した (元の候補: T0852 (見本のプロファイルを見本のモデルと同じフォルダに相対パスでコミットできる形にする))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- `assets/` に置くファイルは小さいもの (JSON 1 つ) なので、AGENTS.md の「小さいものだけ」に収まる。
- やったこと (claude-worker-T0865):
  - `assets/samples/ugoe-sample-profile.json` (日本語、4120 バイト) と `ugoe-sample-profile.en.json` (英語、4100 バイト) をコミットした。
    モデル・モジュール・ホットキーの着せ替えのモジュールは `avatar.ugoe`・`dress-up/long-hair.ugoemod` のような相対パス (区切りは `/`)。
    背景の画像は見本に無い。撮った機械のパスは入っていない。英語のものも置いたのは、前の `--lang en` の代わり (英語の画面の人がそのまま開けるように)。
  - `apps/studio/scripts/sample-profile.mjs`: `sampleProfile(lang)` は相対パスの形を返す。引数なしの `pnpm --filter @ugoe/studio sample-profile`
    は 2 つのファイルを書き直し (自動の保存で変えたものを元に戻すのにも使う)、`--check` は食い違えば 1 で終わる。
    **絶対パスで書き出す引数 (`<出力>`・`--lang`・`--force`) は残さなかった** (呼んでいた docs を直した)。見本のモデルのフォルダの外に書くとき
    (e2e・ui-audit の偽のホスト) のために `resolveSampleProfile(profile, dir)` (絶対パスに解いた写し) を置いた。
  - テスト: `sample-profile.test.mjs` がコミットしたものと作ったものを比べる (食い違えば落ちることを手で確かめた)。パスが `..` を含まない相対パスで、
    ファイルがあることも確かめる。`apps/studio/src-tauri/tests/sample_profile.rs` (新規) が `Profile::read` で 2 つを開き、すべてのパスが
    `assets/samples` の下のあるファイルに解けること・シーンとホットキーを読み落とさないこと、フォルダを一時ディレクトリへ写しても写した先を指すことを確かめる。
  - `scripts/check-pack.mjs`: 見本のプロファイルを `samples\` の中にそのまま写す (`packSamples`)。Windows のパスに書き直す `windowsSampleProfile` は消した。
    プロファイルは自動の保存で書き換わるので、確認表と同じく `--force` が無ければ前のものを残す。`check-pack.test.mjs` に写し方のテストを足した。
  - 文書: docs/manual/studio.md#見本のプロファイル (作る手順を消し、2 つのファイルの場所と元に戻す方法)、docs/manual-check.md (一式の表・
    「フォルダーを動かしたら作り直す」の行・Linux と macOS の手順・S01 の場所)、apps/studio/README.md、assets/samples/README.md (表に 2 つ)。
  - `crates:` の外の小さな変更: `tests/e2e/specs/studio.test.mjs`・`studio.manual.mjs`・`tests/ui-audit/lib/studio-host.mjs` を新しい API
    (`resolveSampleProfile(sampleProfile(lang))`) に合わせた。ui-audit の偽のホストのプロファイルの場所を `assets/samples/ugoe-sample-profile(.en).json`
    にした (tests/ui-audit/README.md・tests/e2e/README.md も直した)。
  - 確かめたこと: `just e2e --only studio` と `just e2e --manual --only studio` が通った (マニュアルの画像に差分は出なかった)。`just ci` が通った。
- やらなかったこと / 確かめていないこと:
  - Windows の実機で `ugoe-check\samples\ugoe-sample-profile.json` を開くことは確かめていない (`\` の区切りと Windows のパスの解き方は
    T0852 の単体テストの範囲。`just check-pack` は動かしていない。写し方は check-pack.test.mjs で確かめた)。
  - 前の `just check-pack` が作った `ugoe-check\ugoe-sample-profile.json` (絶対パスの古いもの) は消さない (残っていても害は無い)。
- 注意: リポジトリの見本のプロファイルをスタジオで開くと、自動の保存でコミットしたファイルが書き換わる (git の差分になる)。
  マニュアルには `pnpm --filter @ugoe/studio sample-profile` で戻す方法を書いた。
- 起票の候補:
  - スタジオで、見本のプロファイル (や読み取り専用の場所のプロファイル) を開いたときに自動の保存で書き換えない (「写しを保存」に誘う) か検討する。

