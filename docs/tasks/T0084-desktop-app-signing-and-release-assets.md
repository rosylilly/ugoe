---
id: T0084
title: "公開: デスクトップアプリの署名と公証の手順を書き、タグのときにインストーラを GitHub Release に添付する"
status: in-progress
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

- [ ] docs/release.md に「デスクトップアプリの署名」の節を足した: Windows の Authenticode (証明書の種類・タイムスタンプ・Tauri の設定
      (`bundle.windows` の `signCommand` か `certificateThumbprint`) のどれを使うかと理由)、macOS の Developer ID Application の署名と公証
      (notarytool。Tauri が読む環境変数)、Linux (署名しない。AppImage と .deb の扱い)。スタジオの Windows の仮想カメラの DLL と macOS の
      Camera Extension (docs/release.md#macos-の仮想カメラ) との関係も書いた
- [ ] 証明書 (と公証の資格情報) を GitHub の Secrets に置く手順と、Secrets の名前の一覧を書いた。置くのは人の手と書いた
- [ ] `editor-build.yml` (と `scripts/app-build.mjs`): Secrets があるときだけ署名・公証し、無ければ今と同じく署名せずに作る (フォークや手元のビルドが失敗しない)。
      署名したかをジョブの要約に出す
- [ ] `editor-build.yml`: `v*` のタグのときに、できたインストーラを GitHub Release (下書き) に添付する。Release を公開するのは人。
      T0421 (UPM の tarball と Godot の zip) と同じ下書きの Release に添付する形にそろえる (先に入った方の形に合わせる)
- [ ] apps/editor/README.md#配布物の署名 と各アプリの README の該当の文を、署名する場合と署名しない場合 (Secrets が無いビルド) に分けて直した
- [ ] 署名と公証そのもの (証明書が要る) は確かめられないので、確かめていないことをメモと docs/manual-check.md の D 節に書いた
      (Windows の SmartScreen の評価・macOS の `spctl --assess`・`stapler validate`)
- [ ] `just ci` が通る

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
