---
id: T0007
title: 実機で手で確かめるための準備 (インストーラと確認表)
status: done
milestone: M8
crates: [docs, scripts, justfile]
depends_on: [T0004, T0812, T0813]
assignee: claude-worker-T0007
---

# T0007: 手で確かめる準備

## 目的

これまでのタスクの「確かめていないこと」は、各タスクのメモ・docs/studio.md の一覧・T0619 などに散らばっている。利用者が実機 (特に Windows) で効率よく確かめられるように、
インストーラと優先度つきの確認表を 1 つにまとめる。

## 受け入れ条件

- [x] `just app::build PLATFORM=windows` で 3 つのアプリの Windows 向けのインストーラを作り、置き場所を報告する (WSL2 からのクロスビルド)。Linux 向けも作れるなら作る
- [x] 確認表 (docs/manual-check.md など。置き場所は決めて README から辿れるようにする): 各タスクのメモと docs/studio.md・T0619 の「確かめていないこと」を集め、
      重複をまとめ、優先度 (配信で使う流れ・壊れると困るもの・OS ごとにしか分からないもの) を付けて並べる。1 項目ごとに「手順・期待する結果・結果を書く欄」
- [x] 優先度の高い順に、Windows で 30 分ほどで回せる「最初の一巡」を作る (スタジオでモデルを開く・トラッキング・出力を OBS に取り込む・仮想カメラ・ホットキー など)
- [x] 結果を書き戻す流れ (確認表に書いて報告してもらう → 不具合はタスクにする) を書く。T0619 との関係を整理する (T0619 を確認表に吸収するか、残すか)
- [x] `just ci` が通る

## 参考

- docs/tasks/*.md のメモの「確かめていないこと」、docs/studio.md の「実機で確かめていないこと」、docs/tasks/T0619-editor-manual-check.md、docs/release.md

## メモ

- **やったこと:**
  - `just app::build PLATFORM=windows` と `PLATFORM=linux` を実行した (どちらも成功。インストーラはコミットしない)。
    Windows (NSIS、署名なし): `target/dist/editor/windows/ugoe_0.0.0_x64-setup.exe` (28.5 MiB)・`target/dist/viewer/windows/ugoe-viewer_0.0.0_x64-setup.exe` (20.7 MiB)・
    `target/dist/studio/windows/ugoe-studio_0.0.0_x64-setup.exe` (20.1 MiB)。Linux: `target/dist/<アプリ>/linux/` に .deb (36.7・21.6・22.3 MiB)・.rpm・.AppImage (約 100 MiB)。
    (worktree の target/ の下。main のチェックアウトの target/dist/ は古いもの)
  - 確認表 `docs/manual-check.md` (docs/README.md・docs/studio.md の「実機で確かめていないこと」・roadmap の M8 から辿れる)。
    各タスクのメモ (grep で 133 ファイル)・docs/studio.md・T0619・apps/viewer/README.md のカメラの許可の表から集め、重複をまとめて 59 項目
    (I 5・S 27・V 7・E 14・U 3・A 3、開発者向けの D 8 は表の 1 行ずつ)。優先度 A (配信の流れ・壊れると困る) 20・B 29・C 10 (D を除く)。
    各項目に手順・期待・元のタスク・OS ごとの結果の欄。
  - 最初の一巡 (Windows、約 30 分): インストール → 見本のプロファイル → 出力ウィンドウのドラッグと隠す → シーンの切り替え → カメラとマイク →
    固まらないこと (T0812) と最小化 → 前面に無いときのホットキー → OBS のウィンドウキャプチャと裏に隠す → ブラウザソース → 仮想カメラ → 起動し直し。
  - `just check-pack` (`scripts/check-pack.mjs`、テストは `just app-build-check` に足した): インストーラ・見本・PSD・Windows のパスの見本のプロファイル・確認表の写しを
    Windows の `%USERPROFILE%\ugoe-check` にまとめる。スタジオの `sample-profile` は WSL の中で実行すると `/home/...` のパスになり Windows のスタジオで読めないため
    (Windows の node で実行すれば Windows のパスになるが、利用者の Windows にリポジトリと node が要る)。
    `%USERPROFILE%\ugoe-check` に作ってある (インストーラ 3 つ・samples・fixtures・ugoe-sample-profile.json・manual-check.md)。
  - T0619 は確認表の「E. エディタ」に吸収した (T0619 のファイルに一行。2026-09-28 の結果は E02・E03 に写した)。T0619 の status は変えていない (done にするかは team-lead)。
  - T0006 と重なる Linux の項目 (S01・S05・S06・S08・S16・S25・V01・E01・U01) は「自動 (T0006)」の印。worker-T0006 に一覧を知らせた。
- **確かめていないこと:** 作ったインストーラを Windows に入れて起動すること (利用者の最初の一巡 I01)。見本のプロファイル (Windows のパス) を Windows のスタジオで開くこと (S01)。
  確認表の手順の文言 (画面の名前・ショートカット・macOS の ⌘W など) は manual/ と docs/studio.md から書いたもので、実機とは突き合わせていない。
  「自動 (T0006)」の印は T0006 の予定から付けた候補で、T0006 の実装と食い違えば直す。
- **後続の候補:** 操作 API (S25) を Windows で手早く試すクライアント (PowerShell の小さなスクリプトなど) があると一巡に入れられる。macOS 向けの一式 (check-pack の macOS 版)。
- 利用者が Windows ですぐ始められるよう、check-pack の一式を `%USERPROFILE%\ugoe-check` に置いた (インストーラ・見本・Windows のパスの見本のプロファイル・確認表の写し。リポジトリの外)。
