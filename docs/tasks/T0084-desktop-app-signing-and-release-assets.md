---
id: T0084
title: "公開: デスクトップアプリの署名と公証の手順を書き、タグのときにインストーラを GitHub Release に添付する"
status: done
milestone: Release
crates: [.github/workflows, scripts, apps/editor, apps/viewer, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0084
---

# T0084: デスクトップアプリの署名と GitHub Release への添付

## 目的

エディタ・ビューア・スタジオのインストーラは `.github/workflows/editor-build.yml` が OS ごとに作るが、署名をしておらず
(apps/editor/README.md#配布物の署名。Windows は SmartScreen、macOS は Gatekeeper が止める)、成果物 (artifact) に置くだけで GitHub Release には添付していない。
macOS で署名と公証の手順があるのはスタジオの仮想カメラ (docs/release.md#macos-の仮想カメラ) だけで、アプリ本体の手順は無い。
公開の前に、署名と添付をワークフローでできる形にしておく。

公開の時期は v1.0.0 の完全リリースと判断するまで決めない (docs/roadmap.md の Release)。**タグを打つ・Release を公開する・証明書を買って登録する
(Authenticode の証明書、Apple Developer Program の Developer ID) のは人の手** で、このタスクはその手前 (手順とワークフロー) までを作る。

## 受け入れ条件

- [x] docs/release.md に「デスクトップアプリの署名」の節を足した: Windows の Authenticode (証明書の種類・タイムスタンプ・Tauri の設定
      (`bundle.windows` の `signCommand` か `certificateThumbprint`) のどれを使うかと理由)、macOS の Developer ID Application の署名と公証
      (notarytool。Tauri が読む環境変数)、Linux (署名しない。AppImage と .deb の扱い)。スタジオの Windows の仮想カメラの DLL と macOS の
      Camera Extension (docs/release.md#macos-の仮想カメラ) との関係も書いた
- [x] 証明書 (と公証の資格情報) を GitHub の Secrets に置く手順と、Secrets の名前の一覧を書いた。置くのは人の手と書いた
- [x] `editor-build.yml` (と `scripts/app-build.mjs`): Secrets があるときだけ署名・公証し、無ければ今と同じく署名せずに作る (フォークや手元のビルドが失敗しない)。
      署名したかをジョブの要約に出す
- [x] `editor-build.yml`: `v*` のタグのときに、できたインストーラを GitHub Release (下書き) に添付する。Release を公開するのは人。
      T0421 (UPM の tarball と Godot の zip) と同じ下書きの Release に添付する形にそろえる (先に入った方の形に合わせる)
- [x] apps/editor/README.md#配布物の署名 と各アプリの README の該当の文を、署名する場合と署名しない場合 (Secrets が無いビルド) に分けて直した
- [x] 署名と公証そのもの (証明書が要る) は確かめられないので、確かめていないことをメモと docs/manual-check.md の D 節に書いた
      (Windows の SmartScreen の評価・macOS の `spctl --assess`・`stapler validate`)
- [x] `just ci` が通る

## 参考

- docs/release.md (配布物の表・macOS の仮想カメラ)、apps/editor/README.md#配布物の署名・#各-os-でのビルド、`.github/workflows/editor-build.yml`、
  `scripts/app-build.mjs`、T0421・T0005・T0830 (スタジオの macOS の仮想カメラ)
- Tauri の署名の文書: <https://v2.tauri.app/distribute/sign/windows/>・<https://v2.tauri.app/distribute/sign/macos/>

## メモ

- T0083 で起票した (元の候補: オーケストレーターの点検 (デスクトップアプリの署名と添付: Windows の Authenticode と macOS のアプリ本体の
  Developer ID の署名・公証の手順を release.md に書き、editor-build.yml でタグのときにインストーラを GitHub Release に添付する。
  証明書がなければ署名を飛ばす形にする。実際の署名は人の手))。
- Release のマイルストーン (docs/roadmap.md の Release)。公開の時期が決まるまで着手しない。
- 外部のアクション (Release に添付するものなど) を足すなら、版を固定し、`gh release upload` で済むならそれを使う (依存を増やさない)。
- T0084 の作業 (claude-worker-T0084):
  - やったこと:
    - `scripts/lib/app-signing.mjs` (新): 環境変数から OS ごとに署名・公証するかを決める (`signingPlan`)。Windows は
      `UGOE_WINDOWS_SIGN_COMMAND` (`%1` 入り) があれば Tauri の `signCommand` の設定を `target/app-signing/windows.conf.json` に書いて重ねる。
      macOS は `APPLE_SIGNING_IDENTITY` で署名、`APPLE_API_*` か `APPLE_ID` の組が揃えば公証。半端な設定は何も作らずに止まる。
      空の値は無いのと同じにし、`tauri build` に渡す前に消す (GitHub Actions は無い Secrets を空で渡し、Tauri は空でも設定済みと読むため)。
    - `scripts/app-build.mjs`: 上を使い、macOS では `.dmg` も notarytool で公証してステープル (Tauri は `.app` だけ)。最後に署名したかを出し、
      `$GITHUB_STEP_SUMMARY` に表を書く (`DRY_RUN=1` は表示だけ)。テストは `scripts/app-build.test.mjs` に足した。
    - `apps/studio/scripts/build-vcam.mjs`: 仮想カメラの DLL は NSIS のフックが入れて Tauri が署名しないので、同じコマンドで署名する。
    - `.github/workflows/editor-build.yml`: Windows は Secrets の `AZURE_CLIENT_SECRET` があれば artifact-signing-cli 0.11.0 を入れて
      コマンドを設定、macOS は `APPLE_API_PRIVATE_KEY` を `.p8` に書く。`v*` のタグで `release` のジョブが成果物を集め、
      `scripts/github-release.mjs` (新) で下書きの Release に添付。ワークフローの既定の権限を `contents: read` にし、release のジョブだけ write。
    - `scripts/github-release.mjs`: 下書きが無ければ作り、同時に作って重なった下書きはいちばん古いものに寄せ (添付の無い余分は消す)、
      `gh release upload --clobber`。公開済みには添付しない。T0421 と T1408 も同じスクリプトで同じ下書きに添付できる (T1408 が先に別の形を
      入れたら、マージのときにどちらかに寄せる)。テストは `scripts/github-release.test.mjs` (`just app-build-check` に足した。justfile は
      範囲の外だが 1 行の小さな変更)。
    - 文書: docs/release.md に「デスクトップアプリの署名」(Windows・macOS・Linux・Secrets・確かめ方) と「GitHub Release の下書き」の節、
      配布物の表のインストーラの行。apps/editor/README.md#配布物の署名 を署名する/しないで分け、ビューア・スタジオの README・
      docs/manual/mcp.md (同梱の `ugoe-mcp` の署名)・docs/manual-check.md (D10〜D12) を直した。
  - 判断: Windows は `certificateThumbprint` でなく `signCommand`。OV/EV の鍵は 2023 年 6 月から HSM に置く決まりで `.pfx` を Secrets に
    置けず、ランナーの証明書ストアも使えないため。既定のサービスは Azure Artifact Signing (Tauri の文書の経路)。使える条件 (国・事業の実績)
    があるので、使えなければ CA のクラウドの署名の CLI に替える (ワークフローの 1 つの手順を替える) と書いた。Secrets はリポジトリの
    Secrets にした (environment に絞る方法は文書に書いた)。
  - 確かめたこと: `just app-build-check` (署名の計画・空の値・半端な設定・要約・Release の選び方)、actionlint 1.7.12 (shellcheck 付き)、
    `DRY_RUN=1 just app::build` (署名なしの表示) と `UGOE_WINDOWS_SIGN_COMMAND='x %1' DRY_RUN=1 just app::build TARGET=studio PLATFORM=windows`
    (設定を重ねる)。
  - 確かめていないこと (証明書と GitHub の実行が要る。manual-check.md の D10〜D12): 実際の署名と公証 (Authenticode のタイムスタンプ・
    SmartScreen の表示・Tauri がサイドカーの `ugoe-mcp.exe` と MSI に署名すること・`spctl --assess`・`stapler validate`・`.dmg` の公証)、
    artifact-signing-cli がランナーの signtool と Azure CLI で動くこと、Tauri が `APPLE_CERTIFICATE` を一時のキーチェーンに入れること、
    タグでの release のジョブ (`gh release view`/`upload` が下書きをタグで引けること・下書きの重なりの寄せ方)。Secrets が無いときに
    今までどおり署名せずに作れることも、GitHub の上では流していない (空の値を消すのはテストで確かめた)。
  - 起票の候補:
    - 実際に Secrets を置いた後の最初のタグ (または手動の実行) で D10〜D12 を確かめ、違っていたら docs/release.md とワークフローを直す。
    - スタジオの macOS の仮想カメラ (VCAM=1) を CI で作って署名・公証する (プロビジョニングプロファイルを Secrets に置く。配布物に入れると決めてから)。
    - Linux のパッケージの署名 (apt/dnf のリポジトリ・AppImage の GPG) を配り方と一緒に決める。
    - 署名の Secrets を environment (タグだけ) に移すかを決める。
