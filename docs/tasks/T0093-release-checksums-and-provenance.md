---
id: T0093
title: "公開: 配布物に SHA-256 のチェックサムと GitHub の成果物の証明 (build provenance) を添える"
status: done
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

- [x] `editor-build.yml`: インストーラごとに SHA-256 のファイルを作り (形は C ABI の `<ファイル>.sha256` にそろえる。別の形にするなら理由をメモに)、
      タグのときにインストーラと一緒に下書きの Release に添付する
- [x] `editor-build.yml`・`ffi-release.yml`: できた配布物に `actions/attest-build-provenance` で証明を付ける (版を固定し、ジョブに要る権限
      `id-token: write`・`attestations: write` は付けるジョブだけに置く)。証明を付けるのはタグと手での起動のときだけか、PR でもかを決めてメモに書く
- [x] npm の `@ugoe/runtime`: `web-runtime-release.yml` の provenance (`id-token: write`。npm の来歴) で足りるかを確かめ、足りなければ同じ形にする。
      Unity の tarball と Godot の zip は T0421 が Release に添付するので、T0421 のメモに「同じ形でチェックサムと証明を付ける」と書き足す
- [x] docs/release.md: 「Linux」の節を書き直し (署名しない理由と、代わりにチェックサムと証明で確かめること)、確かめ方の節を足す
      (`sha256sum -c`・PowerShell の `Get-FileHash`・`gh attestation verify <ファイル> --repo rosylilly/ugoe`)。配布物の表と
      「GitHub Release の下書き」の添付の表も直す。利用者向けの文書 (apps/*/README.md の配布物の説明、docs/manual のインストールの節があれば) にも 1〜2 行
- [x] 確かめていないこと (タグでの添付・実際の証明の検証) を docs/manual-check.md の D 節に足した
- [x] `just ci` が通る (actionlint を流せるなら流す)

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

### 作業の結果 (claude-worker-T0093)

やったこと:

- `scripts/lib/checksum.mjs` (新): `<ファイル>.sha256` の読み書き (`sha256sum` の形の 1 行)。`scripts/ffi-dist.mjs` の書き方・確かめ方を
  これに寄せ、`scripts/app-build.mjs` (`just app::build`) がインストーラごとに同じ形の `.sha256` を `target/dist/<アプリ>/<OS>/` に書く
  (macOS の `.dmg` の公証のステープルの後)。テストは `scripts/app-build.test.mjs` (`just app-build-check`)。
- `editor-build.yml` の `build`・`build-signed` (T0094 が分けた 2 つ。権限 `&build-permissions` と手順をアンカーで共有)・`ffi-release.yml` の `archive`: `.sha256` をまとめたファイルを `actions/attest-build-provenance@v4`
  (2026-08 の v4.2.2 が最新。リポジトリのほかのアクションと同じく主版で固定) の `subject-checksums` に渡して証明を付ける。
  `id-token: write`・`attestations: write` はこれらのジョブだけに置き、`release.yml` の `apps`・`ffi` にも同じ権限を足した。
  タグの下書きへの添付は、成果物の中の `.sha256` もそのまま添付する (`release` のジョブは `find dist -type f` なので変更なし)。
- **証明を付ける場面の決定:** タグ (release.yml から) と手での起動の両方で付ける。PR では 2 つのワークフローが動かない (配布物を作らない)
  ので付けない。手での起動でも付けるのは、タグの前に証明の付け方を予行できるようにするため。ブランチのビルドの証明と区別するため、
  利用者向けの確かめ方では `--source-ref refs/tags/v<版>` を案内した。
- **非公開のリポジトリ:** 今のリポジトリは非公開で、成果物の証明は GitHub のプランによって作れない。証明のステップは
  `if: ${{ !github.event.repository.private }}` で公開のときだけ動かし、非公開のうちはチェックサムだけを作る (公開してから打つタグに証明が付く)。
- **npm:** `@ugoe/runtime` は `bindings/web/scripts/release.mjs` が GitHub Actions で `npm publish --provenance` を付けており、
  `web-runtime-release.yml` は `id-token: write` を持つ。npm の来歴は同じ Sigstore の SLSA の来歴で、tarball のダイジェストもレジストリが持つ
  ので、足りると判断して変更しなかった (確かめ方は `npm audit signatures`)。
- T0421 のメモに、T0093 の形 (checksum.mjs・`subject-checksums`・権限・公開のときだけ) で付けることを書き足した。
- docs/release.md: 「Linux」を書き直し、「配布物を確かめる」(と「確かめ方 (利用者)」) の節を足した。冒頭の署名の表・下書きの添付の表・
  公開の前の人の手順・C ABI の節も直した。apps/editor・viewer・studio の README に 1〜2 行。docs/manual にはインストールの節が無いので触らなかった。
- docs/manual-check.md の D 節に D14 (タグでの添付と証明の生成)・D15 (利用者の確かめ方の手順が通る。D13 は T0094 が使った) を足した。
- actionlint (1.7.12、shellcheck 0.9.0 あり) が通る。

やらなかったこと・確かめていないこと:

- 実際のタグでの添付と証明の生成、`gh attestation verify` での検証 (push・workflow_dispatch をしていない。D14・D15)。
  `subject-checksums` に渡すファイルの形 (`<ダイジェスト>  <名前>` の行) は action.yml の説明と `sha256sum` の形から判断した。
- crates.io のクレートには何も足さない (索引がダイジェストを持ち cargo が確かめる)。

起票の候補: なし。
