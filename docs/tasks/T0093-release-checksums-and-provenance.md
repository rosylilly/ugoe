---
id: T0093
title: "公開: 配布物に SHA-256 のチェックサムと GitHub の成果物の証明 (build provenance) を添える"
status: in-progress
milestone: Release
crates: [.github/workflows, scripts, docs]
depends_on: []
assignee: claude-worker-T0093
---

# T0093: 配布物のチェックサムと成果物の証明

## 目的

Linux の `.deb`・`.rpm`・`.AppImage` は署名しない (docs/release.md#linux)。T0084 は、apt・dnf のリポジトリや AppImage の GPG の署名を
「配り方と一緒に決める」と残した。利用者が決めた形 (下) で、ダウンロードした配布物が CI で作ったものと同じかを利用者が確かめられるようにする。

## 決まっていること (利用者、2026-10-11)

- GPG の署名や apt・dnf のリポジトリは 1.0 では作らない。
- 代わりに、**SHA-256 のチェックサム** と **GitHub の成果物の証明** (`actions/attest-build-provenance`。利用者は `gh attestation verify` で確かめる)
  を、アプリ (エディタ・ビューア・スタジオのインストーラ)・C ABI・npm などの配布物に添える。

## 受け入れ条件

- [ ] `editor-build.yml`: インストーラごとに SHA-256 のファイルを作り (形は C ABI の `<ファイル>.sha256` にそろえる。別の形にするなら理由をメモに)、
      タグのときにインストーラと一緒に下書きの Release に添付する
- [ ] `editor-build.yml`・`ffi-release.yml`: できた配布物に `actions/attest-build-provenance` で証明を付ける (版を固定し、ジョブに要る権限
      `id-token: write`・`attestations: write` は付けるジョブだけに置く)。証明を付けるのはタグと手での起動のときだけか、PR でもかを決めてメモに書く
- [ ] npm の `@ugoe/runtime`: `web-runtime-release.yml` の provenance (`id-token: write`。npm の来歴) で足りるかを確かめ、足りなければ同じ形にする。
      Unity の tarball と Godot の zip は T0421 が Release に添付するので、T0421 のメモに「同じ形でチェックサムと証明を付ける」と書き足す
- [ ] docs/release.md: 「Linux」の節を書き直し (署名しない理由と、代わりにチェックサムと証明で確かめること)、確かめ方の節を足す
      (`sha256sum -c`・PowerShell の `Get-FileHash`・`gh attestation verify <ファイル> --repo rosylilly/ugoe`)。配布物の表と
      「GitHub Release の下書き」の添付の表も直す。利用者向けの文書 (apps/*/README.md の配布物の説明、docs/manual のインストールの節があれば) にも 1〜2 行
- [ ] 確かめていないこと (タグでの添付・実際の証明の検証) を docs/manual-check.md の D 節に足した
- [ ] `just ci` が通る (actionlint を流せるなら流す)

## 参考

- docs/release.md (#linux・#github-release-の下書き・#c-abi-の配布物)、T0084・T1408 のメモ、`scripts/github-release.mjs`、`scripts/ffi-dist.mjs` (`.sha256` の作り方)
- <https://github.com/actions/attest-build-provenance>、<https://cli.github.com/manual/gh_attestation_verify>

## メモ

- T0089 で起票した (元の候補: T0084 (Linux のパッケージの署名を配り方と一緒に決める)。利用者の決定 2026-10-11)。
- Release のマイルストーン (docs/roadmap.md の Release)。タグを打つ・Release を公開するのは人の手。
- 成果物の証明は、リポジトリが非公開だと GitHub のプランによって使えない。使えない条件ならメモに書き、チェックサムだけを先に入れる。
- **並行の注意:** T0094 (署名の Secrets を `release` environment に移す) と同じ `editor-build.yml`・docs/release.md の署名の節を触る。
  T0091 (CI の見直し) も `editor-build.yml` などのリリースのワークフローを触る。
  T1409 (C ABI の soname) と同じ docs/release.md#c-abi-の配布物 を触ることがある。並べて進めるときは片方が入ってから追従する。
