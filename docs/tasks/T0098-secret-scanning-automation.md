---
id: T0098
title: "公開: 秘密の値の検査 (gitleaks) を自動で回すかを決め、決めた形で入れる"
status: in-progress
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

- [ ] 利用者が案を選んだ (選んだ案と日付をメモに書く)
- [ ] 選んだ案を入れた (B なら: `just secrets`、道具の版と sha256 の固定、`release-checks.yml` のジョブ、誤検知の allowlist と理由)
- [ ] 作業ツリーと履歴 (`--log-opts=--all`) で、今の main が通る (見つかったものがあれば直すか、理由を書いて allowlist に入れる)
- [ ] docs/release.md#自動の検査 の「秘密の値」と docs/testing.md の検査の表を直した
- [ ] `just ci` が通る

## 参考

- docs/release.md#公開の前の点検、docs/testing.md#重い検査とリリースの前の検査
- T0090 のメモ (gitleaks の結果と起票の候補)、T0091 (軽い CI と重い検査の分け方)

## メモ

- T0095 で起票した (元の候補: T0090 (公開の直前に gitleaks を CI (または `just`) で流すか決める))。
- Release のマイルストーン。道具はリポジトリに置かない (取得して版と sha256 を確かめる。`scripts/` の既存の取得の書き方に合わせる)。

- 判断 (利用者、2026-10-11): 案 A (push・PR ごとに軽い CI で回す)。手元でも同じ検査を流せるよう `just secrets` (版と sha256 の固定) も用意する。
