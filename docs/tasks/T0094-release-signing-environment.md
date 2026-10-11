---
id: T0094
title: "公開: デスクトップアプリの署名の Secrets を、v* のタグからだけ使える release environment に移す"
status: todo
milestone: Release
crates: [.github/workflows, scripts, docs]
depends_on: []
assignee:
---

# T0094: 署名の Secrets を release environment に移す

## 目的

T0084 は署名と公証の Secrets (docs/release.md#secrets-を置く-初回だけ人の手) をリポジトリの Secrets に置く形にした。リポジトリに書き込める人の
ワークフローならどこからでも読めるので、タグで動くジョブからだけ使えるように絞る。

## 決まっていること (利用者、2026-10-11)

- 署名の Secrets は、`v*` のタグからだけ使える environment `release` に移す。
- 手での起動 (workflow_dispatch)・ブランチ・PR では署名せずに作る (今の「Secrets が無ければ署名しない」と同じ振る舞い)。
- environment の作成と Secrets の移動は人の手。手順は docs/release.md に書く。

## 受け入れ条件

- [ ] `editor-build.yml`: 署名するジョブは `v*` のタグのときだけ `environment: release` を使い、それ以外 (手での起動・ブランチ・PR) では
      environment を付けずに署名せずに作る (ブランチから動かしても environment の保護の規則で失敗しない)。
      ジョブの要約の「Code signing」の表は今のまま (署名しなかった理由が分かる形なら、理由も出す)
- [ ] スタジオの仮想カメラの DLL の署名 (`apps/studio/scripts/build-vcam.mjs`) も同じ条件で動く (Secrets を読む場所が変わるだけで済むことを確かめる)
- [ ] docs/release.md#secrets-を置く-初回だけ人の手 を書き直した: environment `release` を作る (Deployment branches and tags を `v*` のタグに絞る。
      required reviewers を付けるかは任意と書く)、Secrets を environment の Secrets に置く、リポジトリの Secrets に残っていれば消す。
      「タグのときだけに絞るなら」の段落は、この形に置き換える。`crates-io` の environment (crates.io の節) との違いも 1 行で
- [ ] 手での起動で署名を試したいときの手順 (タグを選んで起動する、など) を書いた。確かめていないこと (environment の保護の規則での実行) を
      docs/manual-check.md の D 節 (D10〜D12 の近く) に足した
- [ ] `just ci` が通る (actionlint を流せるなら流す)

## 参考

- docs/release.md (#デスクトップアプリの署名・#secrets-を置く-初回だけ人の手・#cratesio の environment)、T0084 のメモ、`.github/workflows/editor-build.yml`・`crates-release.yml`
- <https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-deployments/managing-environments-for-deployment>

## メモ

- T0089 で起票した (元の候補: T0084 (署名の Secrets をタグだけに絞った environment に移すかを決める)。利用者の決定 2026-10-11)。
- Release のマイルストーン (docs/roadmap.md の Release)。
- `environment:` は式で書ける。タグでないときに空にする書き方が environment を付けない扱いになるかを、actionlint と GitHub の文書で確かめる
  (できなければ、署名するジョブと署名しないジョブを分ける)。
- **並行の注意:** T0093 (チェックサムと成果物の証明) と同じ `editor-build.yml`・docs/release.md の署名の節を触る。T0091 (CI の見直し) も `editor-build.yml` などのリリースのワークフローを触る。並べて進めるときは片方が入ってから追従する。
