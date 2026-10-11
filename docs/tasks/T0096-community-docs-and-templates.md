---
id: T0096
title: "公開: CONTRIBUTING.md・SECURITY.md・CODE_OF_CONDUCT.md と、Issue と PR のテンプレートを書く"
status: done
milestone: Release
crates: [docs, .github, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, README.md]
depends_on: []
assignee: claude-worker-T0096
---

# T0096: 公開の前のコミュニティの文書

## 目的

リポジトリを公開する前に、外の人が貢献・報告するための文書がまだ無い (docs/release.md#公開の前の点検 の「文書」)。
公開の時点で外の人が迷わないよう、貢献の手順・脆弱性の報告先・行動規範と、Issue と PR のテンプレートを用意する。

## 受け入れ条件

- [x] `CONTRIBUTING.md`: 開発の始め方 (`pnpm install`・`just`)、`just ci` を通すこと、Conventional Commits (英語・scope・`--no-verify` を使わない)、
      ライセンスの方針 (`crates/`・`bindings/`・`docs/` は MIT、アプリと `packages/` は GPL-3.0-only。外部 SDK のコードを写さない)、
      AGENTS.md と docs/ への案内。内容は AGENTS.md と食い違わず、詳しいことは写さずにリンクする
- [x] `SECURITY.md`: 脆弱性の報告先 (GitHub の Private vulnerability reporting)、対象にする版、報告に書いてほしいこと
- [x] `CODE_OF_CONDUCT.md`: 既存の行動規範 (Contributor Covenant など) を使うなら版と出所の URL を書く。連絡先は利用者に確かめる (決まらなければメモに書いて止まる)
- [x] `.github/ISSUE_TEMPLATE/` (不具合・要望。`config.yml` で脆弱性は SECURITY.md へ案内) と `.github/pull_request_template.md` (タスク ID・`just ci`・ライセンスの確認)
- [x] 文書の言語を決めて書く (公開仕様 `spec/` は英語。外の人向けの文書を英語にするか、日本語と英語の両方にするか。推奨と理由をメモに)
- [x] README.md から 3 つの文書へリンクし、docs/release.md#公開の前の点検 の「文書」の行を直す
- [x] `just ci` が通る

## 参考

- docs/release.md#公開の前の点検、AGENTS.md、ADR 0018 (ライセンス)
- T0090 のメモ (起票の候補)

## メモ

- T0095 で起票した (元の候補: T0090 (公開の前に CONTRIBUTING.md・SECURITY.md・CODE_OF_CONDUCT.md と Issue / PR のテンプレートを書く))。
- Release のマイルストーン (docs/roadmap.md の Release)。T0090 と同じ。
- 公開も 1.0 の日程も決まっていないので (利用者の方針 2026-10-11)、API の安定や互換の約束は書かない。
- 2026-10-11 (claude-worker-T0096) 完了。
  - **やったこと:** `CONTRIBUTING.md` (人の貢献者向けの短い案内。正は AGENTS.md と docs/ で、要点だけ書いてリンクする)・`SECURITY.md`・
    `CODE_OF_CONDUCT.md`・`.github/ISSUE_TEMPLATE/` (`bug_report.yml`・`feature_request.yml` の Issue のフォームと、空の Issue を切って
    脆弱性を SECURITY.md へ案内する `config.yml`)・`.github/pull_request_template.md` (タスク ID・`just ci`・コミット・テスト・仕様・ライセンスの確認) を書いた。
    README の「ドキュメント」から 3 つの文書へリンクし、「ライセンス」に行動規範の出所を足した。docs/release.md#人がすること の「文書」の行を、
    公開の前に人が決めて直すもの (連絡先・リポジトリの URL・Private vulnerability reporting・ラベル) の一覧に書き直した。
  - **文書の言語 (決めたこと):** README・AGENTS.md・docs/ は日本語、公開仕様 `spec/` は英語 (ugoe の外の実装者が単独で読む文書) という今の分け方にそろえた。
    - `CONTRIBUTING.md` は **日本語** (先頭に英語の短い要約)。貢献の手順は日本語の AGENTS.md と docs/ に頼るので、手順を英語で書いても先で日本語を読むことになり、
      2 つの言語で写すと AGENTS.md と食い違いやすい。英語の Issue と PR を受け付けること、コミットは英語であることを要約に書いた。
    - `SECURITY.md` は **日本語と英語の両方**。報告する人は npm・crates.io の利用者など日本語を読まない人を含み、文書が短いので両方を保てる。
    - `CODE_OF_CONDUCT.md` は Contributor Covenant 2.1 の **英語の原文のまま** (訳は原文の Attribution の節が公式の翻訳へ案内する)。
    - Issue と PR のテンプレートは、見出しと説明を **日本語と英語の併記** にした (外の人が最初に見る画面なので)。
  - **行動規範:** Contributor Covenant 2.1 (https://www.contributor-covenant.org/version/2/1/code_of_conduct/) の Markdown を公式のサイトから取り、
    文を変えずに置いた。版と出所の URL は原文の Attribution の節にある。ライセンスは CC BY 4.0 (2022 年の Contributor Covenant の
    リポジトリの LICENSE.md と README の記載。今のリポジトリの LICENSE.md は v3 のサイトのコードの Hippocratic License 3.0 に変わっているので、README には 2022 年のコミットの LICENSE.md へリンクした)。
  - **連絡先 (仮の印):** オーケストレーターの指示 (利用者が決める値は仮の印にし、公開の前の点検に書く) に従い、`CODE_OF_CONDUCT.md` の Enforcement の節と
    `SECURITY.md` (GitHub を使えない人の連絡先) を Contributor Covenant の印 `[INSERT CONTACT METHOD]` のままにした。受け入れ条件の
    「決まらなければメモに書いて止まる」は、この指示で docs/release.md#人がすること に書くことに置き換えたので、blocked にはしていない。
  - **決めた小さな方針:** CONTRIBUTING.md に「対応するタスクが無い変更は先に Issue で相談 (メンテナがタスクを起票)。誤字の修正などの小さな変更はタスクが無くてもよい」と書いた
    (AGENTS.md の 1 タスク = 1 PR を外の人に当てはめるための補い)。違うなら直してほしい。
  - **SECURITY.md の対象の版:** 1.0 より前なので `main` と次のリリースだけを直し、バックポートしないと書いた。返事と修正の期限は約束しない (API の安定や互換の約束は書かない方針に合わせた)。
  - **確かめたこと:** `just ci` が通る。Issue のフォームの YAML が読めること、`node scripts/check-md-links.mjs` でルートの文書のリンクと見出しが切れていないこと。
    GitHub の上で Issue のフォームがどう表示されるかは、リポジトリに push していないので確かめていない (公開の前の点検で見る)。
  - **やらなかったこと:** `[INSERT CONTACT METHOD]` が残ったまま公開しないための自動の検査 (`scripts/` は範囲の外)。
  - **起票の候補:** 公開の直前に `[INSERT CONTACT METHOD]` などの仮の印が残っていないかを確かめる検査 (release-checks か `just` のレシピ)。
    リポジトリの URL (`rosylilly/ugoe`) を `Cargo.toml` の `repository` と食い違わないか確かめる検査。
