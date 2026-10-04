---
id: T0013
title: CI で feature の下のコードを lint し、Windows と macOS で Rust のテストを流す
status: done
milestone: M9
crates: [justfile, scripts, .github/workflows, docs]
depends_on: [T0829]
assignee: claude-worker-T0013
---

# T0013: CI で feature の下のコードを lint し、Windows と macOS で Rust のテストを流す

## 目的

M9 のコードレビューの横断の所見 (T0211・T0667・T0816。まとめは docs/workflow/code-review.md#m9-の結果)。CI が見ていないコードに誤りが溜まっていた。

- **feature の下のコード (T0211):** CI の clippy は既定の feature だけを見るので、`gpu-tests` などの feature の下の警告が溜まっていた (T0211 で直した)。
- **Windows と macOS (T0667):** 「大文字小文字だけが違う名前を同じファイルとして消す」誤り (T0667 で直した) は、Linux だけの CI では見つからない。
  Rust のテスト (少なくともファイルを書き出す `ugoe-editor-core`・`ugoe-format`・`ugoe-cli`・`ugoe-mcp`) を Windows と macOS でも流す。
- **アプリの OS ごとのコード (T0816):** 仮想カメラは T0829 で `--target` を変えた clippy を足す。エディタ・ビューア・スタジオの `src-tauri` の
  `#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中も同じく型検査されていない。

## 受け入れ条件

- [x] `just lint-group` (と CI の rust-group) が、テストで使う feature (`gpu-tests` など) を有効にした clippy も流す (`scripts/ci-groups.mjs` に feature の組を持たせるなど)
- [x] Windows と macOS のランナーで、ファイルを扱うクレートの Rust のテストを流すジョブを足した (毎回か夜間か、どのクレートかを決めて docs/testing.md#ci-のジョブ に書く)
- [x] アプリの `src-tauri` の OS ごとのコードを、T0829 と同じ `--target` の clippy で検査できるかを確かめ、できるものは足し、できないものは理由をメモに書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0211・T0667・T0816 のメモの「横断」、T0829

## メモ

### やったこと

- **feature の下の lint:** `just lint` と `just lint-group` が、既定の feature の clippy に続けて `--all-features` の clippy も流す
  (`node scripts/ci-groups.mjs args <group> --all-features`。グループでは選んだクレートの feature だけが有効になる)。
  `gpu-tests`・`clap` (ugoe-headless) などの下のコードが対象になる。CI の rust-group は `just lint-group` なのでそのまま入る。
  見つかった警告は `crates/ugoe-ai/tests/normal_render.rs` の manual checked division の 1 件だけで、`NonZeroU32` で直した
  (crates: の外だが指示どおり)。5 つのグループすべてとワークスペースの `--all-features` の clippy が通ることを確かめた。
- **Windows と macOS:** `scripts/ci-groups.mjs` に `OS_CRATES` (`ugoe-format`・`ugoe-editor-core`・`ugoe-cli`・`ugoe-mcp`) を足し、
  `just lint-os` (すべての feature の clippy) と `just test-os` を作った (Node から cargo を呼ぶので Windows の PowerShell でも動く)。
  ci.yml に `rust (windows)` (`windows-latest`)・`rust (macos)` (`macos-latest`) のジョブを足した。毎回の PR と main への push で流す
  (夜間にしなかったのは、T0667 の類の誤りをレビューの時点で止めたいため)。`rust` のまとめのジョブの needs にも入れた。
  docs/testing.md#windows-と-macos に書いた。`OS_CRATES` がワークスペースのクレートかは `just groups` が検査する。
- **アプリの src-tauri の OS ごとのコード:** `scripts/cross-check.mjs` の `CHECKS` に `apps` (エディタ・ビューア・スタジオ、
  `ugoe-studio/gpu-tests` 込み) を足した。
  - **Windows: できる (windows-gnu で)。** msvc は依存の ring (ugoe-ai の ureq → rustls) の C と tauri-build のリソース (.rc) を
    ターゲット向けにコンパイルするので、ターゲットの標準ライブラリだけでは通らない (cc-rs が `lib.exe`・`clang-cl` を探して失敗。
    cargo-xwin で MSVC の SDK を落とせば通るが、CI で Microsoft の SDK を落とすのは避けた)。MinGW (`gcc-mingw-w64-x86-64`。
    windres も入る) で済む `x86_64-pc-windows-gnu` なら通る。アプリのコードは `target_os` で分けていて `target_env` に依存しないので、
    検査の範囲は msvc と同じ。`CHECKS` に `commands` (無ければ apt の入れ方を出して止まる) と `apt` (CI の `rust (cross)` が
    `node scripts/cross-check.mjs apt` で入れる) を足した。手元で `just cross-check` (vcam の 2 つと apps) が通ることを確かめた。
  - **macOS: --target ではできない。** ring と objc2-exception-helper の C が Apple の SDK (`TargetConditionals.h` など) を要る
    (clang + `-ffreestanding` でも通らない)。SDK は ubuntu に入れられないので、`rust (macos)` のジョブが macOS の上で
    `just lint-group apps` を流す (仮想カメラも macOS の上で検査される)。
- actionlint (1.7.12) で ci.yml を検査した。新しく出る指摘は無い (shellcheck の SC2012 の info が 2 件出るが、既存の gpu と
  conformance のジョブの `ls` のもの)。`just lint-os`・`just test-os`・`just cross-check` は手元の Linux で通ることを確かめた。

### 確かめていないこと

- GitHub Actions の上では動かしていない (push できない)。特に Windows と macOS のランナーのジョブは手元で再現できない:
  - Windows のランナーで `shell: bash` (Git Bash) の `rustup`・`just` (レシピは PowerShell で動く) が期待どおり動くか、ビルドの時間
    (timeout-minutes: 60 にした)。
  - macOS のランナーで `just lint-group apps` が通るか (macOS の cfg の中のエディタの資格情報の保存先・Tauri の macOS のコード・
    仮想カメラは、aarch64-apple-darwin の --target の clippy でも一部しか見ていない。アプリは手元で macOS の型検査をできていない)。
  - macOS のテストは実行していない。
- **Windows のテストの近似として、Wine で `x86_64-pc-windows-gnu` のテストを流した** (手順は docs/testing.md#windows-と-macos)。
  `ugoe-format`・`ugoe-editor-core`・`ugoe-cli` はすべて通った。`ugoe-mcp` は 1 件だけ失敗した:
  `http::tests::slow_requests_time_out_with_408` が `no status in ""` (crates/ugoe-mcp/src/http.rs の 2 つ目の場面)。
  クライアントが 1 バイトずつ送り続ける間にサーバが 408 を書いて閉じるので、読んでいないデータを残した close が RST になり、
  Windows の流儀では受け取り済みの 408 が捨てられる (Wine はこれを真似る)。本物の Windows でも落ちる見込みが高い。
  `ugoe-mcp` は crates: の外で、直すにはサーバ側の lingering close (誤りを書いたら書き込み側を閉じ、相手の残りを時間と量の上限まで
  読み捨ててから閉じる) かテストの作り直しが要り、HTTP の振る舞いに関わるので手を付けなかった。`rust (windows)` はこの 1 件で
  失敗する見込み。

### 起票の候補

- ugoe-mcp の HTTP の誤りの答え (408・413・431) の lingering close。Windows のクライアントが RST で答えを失う
  (上の Wine の失敗。`body_over_the_limit_is_413` なども同じ条件になりうる)。直したら `rust (windows)` で確かめる。
- ugoe-ai・ugoe-mcp も `rust (cross)` の windows-gnu で clippy する (今は `rust (windows)` のランナーの上の clippy に任せている)。
