---
id: T0096
title: "公開: CONTRIBUTING.md・SECURITY.md・CODE_OF_CONDUCT.md と、Issue と PR のテンプレートを書く"
status: todo
milestone: Release
crates: [docs, .github, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, README.md]
depends_on: []
assignee:
---

# T0096: 公開の前のコミュニティの文書

## 目的

リポジトリを公開する前に、外の人が貢献・報告するための文書がまだ無い (docs/release.md#公開の前の点検 の「文書」)。
公開の時点で外の人が迷わないよう、貢献の手順・脆弱性の報告先・行動規範と、Issue と PR のテンプレートを用意する。

## 受け入れ条件

- [ ] `CONTRIBUTING.md`: 開発の始め方 (`pnpm install`・`just`)、`just ci` を通すこと、Conventional Commits (英語・scope・`--no-verify` を使わない)、
      ライセンスの方針 (`crates/`・`bindings/`・`docs/` は MIT、アプリと `packages/` は GPL-3.0-only。外部 SDK のコードを写さない)、
      AGENTS.md と docs/ への案内。内容は AGENTS.md と食い違わず、詳しいことは写さずにリンクする
- [ ] `SECURITY.md`: 脆弱性の報告先 (GitHub の Private vulnerability reporting)、対象にする版、報告に書いてほしいこと
- [ ] `CODE_OF_CONDUCT.md`: 既存の行動規範 (Contributor Covenant など) を使うなら版と出所の URL を書く。連絡先は利用者に確かめる (決まらなければメモに書いて止まる)
- [ ] `.github/ISSUE_TEMPLATE/` (不具合・要望。`config.yml` で脆弱性は SECURITY.md へ案内) と `.github/pull_request_template.md` (タスク ID・`just ci`・ライセンスの確認)
- [ ] 文書の言語を決めて書く (公開仕様 `spec/` は英語。外の人向けの文書を英語にするか、日本語と英語の両方にするか。推奨と理由をメモに)
- [ ] README.md から 3 つの文書へリンクし、docs/release.md#公開の前の点検 の「文書」の行を直す
- [ ] `just ci` が通る

## 参考

- docs/release.md#公開の前の点検、AGENTS.md、ADR 0018 (ライセンス)
- T0090 のメモ (起票の候補)

## メモ

- T0095 で起票した (元の候補: T0090 (公開の前に CONTRIBUTING.md・SECURITY.md・CODE_OF_CONDUCT.md と Issue / PR のテンプレートを書く))。
- Release のマイルストーン (docs/roadmap.md の Release)。T0090 と同じ。
- 公開も 1.0 の日程も決まっていないので (利用者の方針 2026-10-11)、API の安定や互換の約束は書かない。
