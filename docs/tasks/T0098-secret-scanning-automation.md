---
id: T0098
title: "公開: 秘密の値の検査 (gitleaks) を自動で回すかを決め、決めた形で入れる"
status: done
milestone: Release
crates: [.github/workflows, scripts, justfile, docs]
depends_on: []
assignee: claude-worker-T0098
---

# T0098: 秘密の値の検査を自動で回すか

## 目的

T0090 は gitleaks 8.24.3 を手で流して作業ツリーと履歴を調べた (秘密の値は無かった)。今は「公開の直前に人が流す」だけで、自動の検査は無い
(docs/release.md#自動の検査)。版を固定した道具を CI か `just` で回すかを決め、決めた形で入れる。

**判断が要る (利用者)。** 着手したら、下の案と推奨を利用者に確かめてから実装する。

| 案 | 内容 | 利点 | 欠点 |
| --- | --- | --- | --- |
| A | `ci.yml` の `rust-checks` で push・PR ごとに作業ツリー (と push の差分) を見る | 入った直後に気づく | 道具の取得と実行が push ごとに増える (軽い CI の時間・課金。T0091)。誤検知で push が止まる |
| B | `just secrets` (版と sha256 を固定して `target/` などに取得し、作業ツリーと履歴を見る) を足し、`release-checks.yml` (リリースの前・月 2 回の定期・手動) で回す | 課金が少ない。リリースの前に必ず通る。手元でも同じ手順 | 入ってから気づくまで最長で半月 |
| C | 今のまま (公開の直前に人が流す)。公開してからは GitHub の Secret scanning と push protection に任せる | 手間が無い | 公開の前の非公開の間は、人が流すのを忘れると気づかない |

- **推奨: B。** 公開するまでは非公開のリポジトリで、外からの push は無い。リリースの前の検査に入れれば公開の前に必ず通り、軽い CI を重くしない。
  公開してからは GitHub の push protection (docs/release.md#人がすること) が push ごとの守りになる。
- 誤検知 (WebSocket の RFC 6455 の例の `Sec-WebSocket-Key`。`apps/studio/src-tauri/src/api/`) は、設定ファイル (`.gitleaks.toml` の allowlist か
  `gitleaks:allow` の注記) で理由を書いて外す。

## 受け入れ条件

- [x] 利用者が案を選んだ (選んだ案と日付をメモに書く)
- [x] 選んだ案を入れた (B なら: `just secrets`、道具の版と sha256 の固定、`release-checks.yml` のジョブ、誤検知の allowlist と理由)
- [x] 作業ツリーと履歴 (`--log-opts=--all`) で、今の main が通る (見つかったものがあれば直すか、理由を書いて allowlist に入れる)
- [x] docs/release.md#自動の検査 の「秘密の値」と docs/testing.md の検査の表を直した
- [x] `just ci` が通る

## 参考

- docs/release.md#公開の前の点検、docs/testing.md#重い検査とリリースの前の検査
- T0090 のメモ (gitleaks の結果と起票の候補)、T0091 (軽い CI と重い検査の分け方)

## メモ

- T0095 で起票した (元の候補: T0090 (公開の直前に gitleaks を CI (または `just`) で流すか決める))。
- Release のマイルストーン。道具はリポジトリに置かない (取得して版と sha256 を確かめる。`scripts/` の既存の取得の書き方に合わせる)。

- 判断 (利用者、2026-10-11): 案 A (push・PR ごとに軽い CI で回す)。手元でも同じ検査を流せるよう `just secrets` (版と sha256 の固定) も用意する。

### 結果 (2026-10-11)

- **入れたもの (案 A):**
  - `scripts/secrets.mjs` (`just secrets`): gitleaks **8.30.1** (2026-03 の Release。T0090 の 8.24.3 より新しい規則) を、機械ごと
    (linux x64・arm64、macOS x64・arm64、Windows x64) の sha256 を固定して取得し、メインの作業ツリーの `target/tools/gitleaks-<版>/` に置く
    (worktree でも共有。リポジトリには置かない)。作業ツリー (`gitleaks dir`。`.gitignore` で無視するものは、`git ls-files --ignored` から
    一時的な設定の allowlist を作って外す) とコミット (`gitleaks git`) を見る。値は `--redact` でログに出さない。
  - 既定 (手元の `just secrets` と `just ci`) は履歴のすべて (`--log-opts=--all`)。`--since <rev>` で `<rev>..HEAD` だけ。
  - `scripts/gitleaks.toml`: 既定の規則 + RFC 6455 の例のキー (`Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==`) だけを外す allowlist (理由と出典つき)。
  - `ci.yml` の `rust-checks` に `Secrets` の段 (PR は `HEAD^1`、push は `github.event.before` からのコミット)。
  - 単体テスト `scripts/secrets.test.mjs` (`just script-check`)。`just ci` に `secrets` を入れた。
- **CI で履歴のすべてを見ない (差分だけ) 理由:** 軽さを優先した。手元の測定では、gitleaks そのものは履歴のすべて (約 1400 コミット) でも
  約 2 秒だが、CI の checkout は深さ 1 なので、すべてを見るには毎回すべての履歴を取り直す (今は約 40 MB、手元の clone で浅いものより
  約 3.4 秒多い) ことになり、それは履歴とともに伸びる。入る前のコミットは前の検査を通っているので、作業ツリーと push・PR のコミットで足りる。
  範囲がそろうまで `git fetch --deepen=50` (最大 20 回) で深くし、push の前のコミットが取れない (新しいブランチ・履歴の書き換え) ときは
  `--unshallow` して履歴のすべてを見る。
- **確かめたこと:** 今の main (とこのブランチ) の作業ツリーと履歴のすべて (`--all`、約 1400 コミット) で見つかるものは無い
  (allowlist の前は RFC 6455 の例の 4 件だけ)。Git が無視するディレクトリ (`dist/`) の偽のトークンは見ず、無視されていない新しいファイルの
  偽のトークンは見つけて落ちる。浅い clone (file://) で、`--since <120 前のコミット>` (50 ずつ 3 回深くする)・マージのコミットの `--since HEAD^1`
  (PR の 3 コミットだけ)・取れないコミット (履歴のすべてに切り替える) を確かめた。actionlint (1.7.12、shellcheck 0.9.0) は通る。
- **確かめていないこと:** GitHub の Actions の上での実行 (push・workflow_dispatch はしていない。actions/checkout の浅い clone で
  `git fetch --deepen <sha>` が通るかは、手元の file:// でだけ確かめた)。macOS と Windows での取得と展開 (Windows は bsdtar で zip を展開する想定)。
- **注意:** `just ci` の履歴の検査は `--all` なので、ほかの worktree のタスクのブランチと stash も見る (そこに偽の秘密の値があると落ちる)。
- **範囲の外:** なし (設定は `scripts/gitleaks.toml` に置き、ルートの `.gitleaks.toml` は作らなかった)。
- **起票の候補:**
  - AGENTS.md のコマンドの表に `just secrets` を足す (範囲の外なので触っていない)。
  - gitleaks の新しい版を知る仕組み (今は手で `GITLEAKS_VERSION` と sha256 を上げる)。
