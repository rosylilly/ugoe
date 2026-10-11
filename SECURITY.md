# セキュリティ / Security Policy

[日本語](#日本語) | [English](#english)

## 日本語

### 報告のしかた

脆弱性は **公開の Issue・Discussions・PR に書かず**、GitHub の
[Private vulnerability reporting](https://github.com/rosylilly/ugoe/security/advisories/new) で非公開に報告してください
(リポジトリの「Security」タブの「Report a vulnerability」)。

GitHub を使えない場合の連絡先: [INSERT CONTACT METHOD]

報告は公開されず、メンテナだけが読みます。受け取ったら確認の返事をし、直し方と公開の時期を報告した人と相談します。
返事と修正の期限は約束できません (少人数で開発しているため)。

### 対象にする版

ugoe は開発中で、1.0 より前です。修正は `main` と、次に出すリリースで行います。
古いリリースへの修正の取り込み (バックポート) はしません。報告の前に、できれば `main` か最新のリリースで再現するかを確かめてください。

対象は、このリポジトリのすべてです (`crates/` のランタイムとツール・C ABI・wasm・`bindings/`・エディタ・ビューア・スタジオ・MCP サーバ・CLI)。
特に、外から来るデータを読む箇所 (`.ugoe` のバンドル・モジュール・PSD・画像・JSON・C ABI の引数・アプリの IPC・スタジオの外部 API・MCP サーバ) の
不具合 (メモリの破壊・停止しない処理・過大な確保・意図しないファイルの読み書き) を報告してください。

依存するライブラリそのものの脆弱性は、そのライブラリへ報告してください。ugoe が影響を受ける場合は、こちらにも知らせてください。

### 報告に書いてほしいこと

- 影響を受けるもの (クレート・アプリ・バインディングの名前) と版 (リリースの版か、`main` のコミット)
- 脆弱性の種類と、起こりうる影響 (誰が何をできるか)
- 再現の手順。再現に使うファイル (細工した `.ugoe` など) があれば添付してください
- 環境 (OS・GPU・ブラウザ・ゲームエンジンなど、関係があるもの)
- 直し方の案があれば、その案

## English

### Reporting a vulnerability

Please **do not report vulnerabilities in public issues, discussions or pull requests.**
Report them privately through GitHub's
[Private vulnerability reporting](https://github.com/rosylilly/ugoe/security/advisories/new)
("Report a vulnerability" in the repository's Security tab). Reports in English or Japanese are welcome.

If you cannot use GitHub: [INSERT CONTACT METHOD]

Only the maintainers can read the report. We will acknowledge it and discuss the fix and the disclosure timeline with you.
We cannot promise a response or fix deadline (ugoe is developed by a small team).

### Supported versions

ugoe is under development and has not reached 1.0. Fixes go to `main` and the next release.
We do not backport fixes to older releases. If possible, please check that the issue reproduces on `main` or the latest release.

The whole repository is in scope: the runtime and tools in `crates/`, the C ABI, the WebAssembly build, `bindings/`,
the editor, the viewer, the studio, the MCP server and the CLI. We are especially interested in bugs in code that reads external data
(`.ugoe` bundles, modules, PSD files, images, JSON, C ABI arguments, the apps' IPC, the studio's external API, the MCP server):
memory corruption, unbounded loops, excessive allocation and unintended file access.

Vulnerabilities in a dependency should be reported to that project. Please let us know as well if ugoe is affected.

### What to include

- The affected component (crate, app or binding) and version (a release version or a `main` commit)
- The kind of vulnerability and its impact (who can do what)
- Steps to reproduce, with any file needed to reproduce it (for example, a crafted `.ugoe` file)
- The environment, where relevant (OS, GPU, browser, game engine)
- A suggested fix, if you have one
