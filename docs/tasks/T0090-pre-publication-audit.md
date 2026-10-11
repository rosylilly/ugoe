---
id: T0090
title: "公開の前の点検: 作業ツリーと履歴に、機械に固有のパス・秘密の値・外部 SDK・個人の情報が無いかを調べ、作業ツリーの分を直す"
status: done
milestone: Release
crates: [docs, scripts, justfile, .github/workflows]
depends_on: []
assignee: claude-worker-T0090
---

# T0090: 公開の前の点検

## 目的

利用者の方針 (2026-10-11): 先々はオープンソースとして公開する。公開すると作業ツリーだけでなく履歴もすべて見える。公開の前に、見せてはいけないもの・
見せる意味の無いものが入っていないかを調べ、作業ツリーの分は直し、履歴の分は報告する。利用者は「マシンローカルなパスなどが文書に埋まっているのを見かけた」と言っている
(例: docs/tasks/ の 44 ファイルに `/home/<名前>/…` やエージェントの一時ディレクトリ `/tmp/claude-<番号>/…` が入っている)。

## 受け入れ条件

- [x] **作業ツリーの点検と直し:** 次を `git grep` などで洗い出し、直した (直さないものは理由をメモに)。
  - 機械に固有のパス: ホームのディレクトリ (`/home/<名前>`・`/Users/<名前>`・`C:\Users\<名前>`)、mbx のキャッシュ (`~/.cache/mbx/...`)、エージェントの一時ディレクトリ (`/tmp/claude-*`)、worktree の絶対パス
    (`.worktrees/T…` の絶対パス)。リポジトリの中の相対パスか、`<repo>`・`$HOME` などの書き方に直す。タスクファイルの「参考」の一時ディレクトリへのリンクは、もう存在しないので外す
  - 画像 (docs/manual/images・tests の期待画像) に写ったパス (T0969 で直した種類。OCR は要らないが、撮る手順が固定のパスを使うことを確かめる)
  - 秘密の値: API キー・トークン・秘密鍵・証明書・パスワードの形 (gitleaks か trufflehog を使うなら版を固定し、使った道具と結果をメモに)
  - 外部 SDK のソースや資産 (Live2D Cubism SDK など。AGENTS.md の規則) と、ライセンスの分からない画像・モデル・フォント (assets/ の出所)
  - 個人の情報: メールアドレス (コミットの著者を除く)・本名・社内の URL やホスト名
  - 大きなファイル (公開に不向きな生成物・バイナリ)
- [x] **再発の防止:** 機械に固有のパス (少なくともホームのディレクトリと `/tmp/claude-`) が入ったら落ちる軽い検査を足し、`just ci` と pre-commit (lefthook) に入れた
- [x] **履歴の点検 (報告だけ):** `git log -p --all` 全体に対して同じ検査 (秘密の値・外部 SDK・大きなファイル・パス) を行い、見つかったものを一覧にした。
      履歴の書き換え (git filter-repo など) は **行わない**。書き換えが要るかどうか・どの方法がよいかを推奨と一緒にメモに書く (判断は利用者)
- [x] 公開の前に人がすること (リポジトリの設定・Secrets・ブランチの保護・フォークからの PR の扱い・Issue と Discussions・CONTRIBUTING・SECURITY.md・CODE_OF_CONDUCT の有無) を
      docs/release.md に「公開の前の点検」として書いた (無いファイルは後続の候補に)
- [x] `just ci` が通る

## 参考

- AGENTS.md (外部のコードとライセンス)、T0969 (マニュアルの画像のパス)、docs/release.md、docs/tasks/README.md (タスクファイルの書き方)

## メモ

- 利用者の指示 (2026-10-11) でオーケストレーターが起票した。
- 並行の注意: 20 回目の起票 (T0089) が docs/tasks を触っている。タスクファイルのパスの直しは、T0089 が main に入った後に rebase して残りも直す。

### T0090 の作業 (2026-10-11)

#### 作業ツリーの点検と直し

- **機械に固有のパス:** `git grep` で 46 ファイル (すべて docs/tasks) に見つかった。直した:
  - エージェントの一時ディレクトリ (`/tmp/claude-<番号>/<プロジェクト>/<セッション>/scratchpad/…`) は `<scratchpad>/…` に、
    「(scratchpad = …)」の注は「scratchpad は作業したエージェントの一時ディレクトリ。もう残っていない」に、
    「参考」の「オーケストレーターの集計: …」(T0019〜T0089 の起票のタスク 20 個) は行ごと消した。
  - worktree の絶対パス (T1632) は `<repo>/.worktrees/…` に。Windows のホーム (T0007 の `C:\Users\<名前>\ugoe-check`) は `%USERPROFILE%\ugoe-check` に。
  - docs/tasks 以外の `/home/…`・`/Users/…`・`C:\Users\…` は、テストと文書の例の名前 (`u`・`a`・`me`・`you`・`audit`・`ugoe`) だけで、直していない。
    `~/.cache/mbx/…` (docs/workflow/build-cache.md・scripts/worktree.mjs) は道具の決まった置き場の書き方で、機械に固有ではないので残した。
- **画像に写ったパス:** マニュアルの画像は `just e2e --manual` が `/tmp/ugoe-manual/` の決まった場所で撮る (`tests/e2e/run.mjs` の `MANUAL_ROOT`。T0969)
  ことを確かめた。パスの写りうる画像 (mcp-connect・mcp-claude-desktop・studio-overview・studio-load・studio-settings) を目で見て、
  `/tmp/ugoe-manual/…` とファイル名だけであることを確かめた。ui-audit の偽のホストのパスも例の名前 (`/home/audit`・`/home/ugoe`)。
  すべてのバイナリ (215 ファイル) の文字列 (`strings`) にホームのパス・ユーザー名は無い。
- **秘密の値:** gitleaks 8.24.3 (GitHub の Release の linux_x64、sha256 `9991e0b2…4ee29c` を照合。リポジトリの外に置いた) で
  `gitleaks dir .` と `gitleaks git . --log-opts=--all` (1367 コミット)。見つかったのは 3 つで、どれも WebSocket の RFC 6455 の例の
  `Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==` (`apps/studio/src-tauri/src/api/ws.rs`・`tests.rs`。秘密ではない)。
  正規表現 (秘密鍵・証明書・`sk-`・`ghp_`・`AKIA`・`AIza`・`xox`) でも、docs/release.md の Secrets の説明の 1 行だけ。
  道具はリポジトリに足していない (公開の直前に人が流す手順を docs/release.md に書いた)。
- **外部 SDK・出所:** Live2D Cubism SDK のソース・資産 (`.moc3`・`.cmo3`・`model3`・Framework など) は作業ツリーにも履歴にも無い。
  Live2D への言及は仕様の比較と公式の文書・フォーラムの URL だけ。画像・モデル・PSD は assets/ の README の表に出所とライセンスがある。
  フォントのファイルは無い。アプリのアイコン (`apps/*/src-tauri/icons`) の出所の書き付けは見当たらない (起票の候補)。
  **利用者に見てほしい点:** `assets/fixtures/psd-real/` の 2 つの PSD は「ChatGPT で生成した画像を ComfyUI の See-Through で分けたもの / MIT」と
  書いてある。生成した絵を MIT で出してよいか (生成サービスの規約・元にした絵の有無) は利用者の判断で、ここでは変えていない。
  PSD の XMP は作成ツール (Photoshop 27.10 (Windows)) と日時だけで、名前やパスは無い。
- **個人の情報:** メールアドレスは例 (`example.com` など) と、ライセンスの表記 (third-party-notices) の依存の作者のものだけ。
  本名は LICENSE・spec の著作権者の表記 (「Sho KUSANO and the ugoe contributors」) にある。著作権者の表記なので意図したものとみなし、残した。
  社内の URL・ホスト名は無い (URL は公開のサイトだけ)。GitHub の `rosylilly/ugoe` の URL はリポジトリそのもの。
- **大きなファイル:** 作業ツリーの最大は PSD 3.3MB・1.8MB と見本の `.ugoe` 1.3〜1.5MB (README に理由がある)。履歴の pack は約 52MB。

- main (T0089 の 20 回目の起票、cd3d8400) に rebase した。T0089 の「参考」の集計の行 (今はもう無い一時ファイルの名前) も消した。新しいタスクファイル (T0092〜T0094・T0195・T0739・T0881〜T0884・T1409・T1410・T1678) にも `just local-paths` が効き、機械のパスは無かった。

#### 再発の防止

- `scripts/check-local-paths.mjs` (`just local-paths`): ホームのディレクトリ (`/home/<名前>`・`/Users/<名前>` (`/mnt/c/Users/…` を含む)・
  `C:\Users\<名前>` (`\\`・`/` の区切りも)・WSL の `\home\<名前>`) と `/tmp/claude-<数字>` を見つけたら落ちる。例のパスは 1 文字の名前か
  `PLACEHOLDER_USERS` (`audit`・`me`・`name`・`runner`・`runneradmin`・`ugoe`・`user`・`you`) を使う。`runneradmin` は GitHub Actions の
  Windows のランナーのホーム (T0967・T0973 のログの写し)。テストは `scripts/check-local-paths.test.mjs`。
- `just ci` (`local-paths`)・CI の `rust-checks` の「Local paths」・lefthook の pre-commit (`{staged_files}`) に入れた。
  docs/testing.md (検査の表と CI のジョブ)・AGENTS.md (pre-commit)・docs/tasks/README.md (タスクファイルに機械のパスを書かない) を直した。
- **範囲の外の変更 (小さなもの):** `lefthook.yml` (受け入れ条件の pre-commit のため)・`AGENTS.md` (pre-commit の一覧の 1 行)。

#### 履歴の点検 (報告だけ。書き換えていない)

`git log -p --all` (1367 コミット。ブランチは main と手元の task/… だけ) に同じ検査をした。

| 種類 | 見つかったもの |
| --- | --- |
| 機械に固有のパス | docs/tasks の 46 ファイル・46 コミットで、エージェントの一時ディレクトリ 57 か所 (ディレクトリ名に WSL のユーザー名とセッションの UUID が入る)、WSL のホーム (`/home/<ユーザー名>/src/github.com/<ユーザー名>/ugoe/.worktrees/T1632/…`) 2 か所、Windows のホーム (`C:\Users\<Windows のユーザー名>`) 2 か所。コード・ほかの文書・コミットメッセージには無い |
| 秘密の値 | なし (gitleaks の 3 件は上の RFC 6455 の例) |
| 外部 SDK | なし (Cubism SDK のファイル名・Live2D の著作権表記の行は履歴のどの差分にも無い) |
| 大きなファイル | 1MB を超える blob は 12 個 (PSD 2 つと見本の `.ugoe` の版違い 10 個。最大 3.3MB)。消したファイルは 85 個あるが、すべてコード・タスクファイル・改名で、生成物やダウンロードは無い |
| 個人の情報 | コミットの著者・コミッターは 1 人 (名前と 1 つのメールアドレス) で、すべてのコミットに入る。差分のメールアドレスは例と依存の作者だけ |

- **推奨: 履歴は書き換えない。** 履歴のパスから分かるのは WSL と Windows のユーザー名・ディレクトリの置き方・エージェントのセッションの UUID で、
  ユーザー名は GitHub のアカウント名と同じか近く、秘密の値は無い。書き換えるとすべてのコミットの ID が変わり、タスクファイルや文書の
  コミットの ID (`376f1cd` など)・CI の実行への参照が切れる。
- 書き換えるなら: 公開の直前に一度だけ、`git filter-repo --replace-text <式のファイル>` で `/tmp/claude-…/scratchpad` → `<scratchpad>` などを置き換える
  (道具は版を固定してリポジトリの外に置く)。そのあと手元のすべての clone・worktree を作り直す。著者のメールアドレスを隠すなら
  `--mailmap` で同時に行う (そうしないなら、今後のコミットだけを GitHub の noreply にする。docs/release.md#公開の前の点検)。

#### 公開の前に人がすること

docs/release.md に「公開の前の点検」を書いた (自動の検査・履歴の扱い・コミットの著者・push するもの・リポジトリの設定・Rulesets・
Secrets の environment・フォークからの PR・セキュリティの機能・文書)。ワークフローは `pull_request_target` を使っておらず、Secrets を使う
ジョブはタグか手動の実行だけで動くことを確かめた。

#### やらなかったこと・確かめていないこと

- 画像は OCR していない (撮る手順と、パスの写りうる画像を目で見ただけ)。
- trufflehog は使っていない (gitleaks だけ)。
- GitHub の設定の項目の名前は GitHub の画面では確かめていない (公開の前に人が設定するときに合わせる)。

#### 起票の候補

- `CONTRIBUTING.md`・`SECURITY.md`・`CODE_OF_CONDUCT.md`・Issue と PR のテンプレート (`.github/`) を書く (公開の前)。
- アプリのアイコン (`apps/*/src-tauri/icons/icon.png`) の出所とライセンスを README か assets の表に書く。
- `assets/fixtures/psd-real/` の生成した PSD のライセンスの書き方を利用者と確かめる (生成サービスの規約)。
- 公開の直前に gitleaks を CI (または `just`) で流すか決める (版を固定した道具を入れる手間と、誤検知の扱い)。
