---
id: T0059
title: "e2e: 3 つのアプリを 1 回の cargo でビルドし、e2e の CI は rust (apps) のキャッシュを読むだけにする"
status: done
milestone: M10
crates: [tests/e2e, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0059
---

# T0059: e2e: 3 つのアプリを 1 回の cargo でビルドし、e2e の CI は rust (apps) のキャッシュを読むだけにする

## 目的

T0057 の調べ (メモの C) で、e2e のビルドを 3 つのアプリをまとめた 1 回の cargo
(`cargo build -p ugoe-editor -p ugoe-viewer -p ugoe-studio -p ugoe-studio-vcam --features tauri/custom-protocol`) にすれば、
rust (apps) の後のコンパイルは依存 17 個 (手元で約 31 秒) で済むことが分かった。そうすれば e2e は自分のキャッシュを持たずに
`group-apps` と `conformance` のキャッシュを読むだけにでき、CI のキャッシュの 1 組が約 1.1GB 減る (約 4.6GB)。

## 受け入れ条件

- [x] `tests/e2e/run.mjs` が、各アプリのフロントエンドを作って (`pnpm build`、要るものは `pnpm wasm`) から、3 つのアプリを 1 回の cargo でビルドする
- [x] できた実行ファイルが `tauri build` と同じ振る舞いになることを確かめ、確かめ方をメモに書いた (frontendDist の埋め込み・tauri.conf.json の読み込み・
      `--ci` の扱い。違いがあれば直したか、理由を書いた)
- [x] `just e2e` の全筋書き (3 つのアプリ) と `just e2e --manual` が通る
- [x] e2e.yml が e2e の専用のキャッシュを保存せず、rust (apps) (`group-apps`) と `conformance` のキャッシュを読むだけになった。
      docs/testing.md の「CI のキャッシュ」と tests/e2e/README.md を今の形に直した
- [x] actionlint で増えた指摘が無い
- [ ] 1 組の実際の合計は push の後に `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list` で測る。測れなければ「CI で確かめること」としてメモに書く
- [x] `just ci` が通る

## 参考

- T0057 のメモ (調べたこと C・起票の候補)、T0051 のメモ (見積もり)
- tests/e2e/run.mjs・tests/e2e/README.md、.github/workflows/e2e.yml・ci.yml、docs/testing.md の「CI のキャッシュ」

## メモ

- T0058 で起票した (元の候補: T0057 (e2e のビルドを 1 回の cargo にする))。

### やったこと

- `tests/e2e/run.mjs`: アプリごとの `pnpm --filter <app> tauri build --debug --no-bundle --ci` と `cargo build -p ugoe-mcp` をやめ、
  選んだアプリごとに `tauri.conf.json` の `beforeBuildCommand` (`pnpm wasm && pnpm build`) をアプリのディレクトリで `sh -c` で流してから、
  1 回の `cargo build --bins --features tauri/custom-protocol -p ugoe-editor -p ugoe-mcp -p ugoe-viewer -p ugoe-studio` でビルドする。
  `--only` のときは選んだアプリ (と隣に置くもの) だけを `-p` に並べる。`ugoe-studio-vcam` は `-p` に入れない (スタジオの依存なので feature の
  統合は変わらず、`--bins` では何も作らない)。`ugoe-mcp` はエディタの依存なので、入れても統合は変わらない。
- e2e.yml: `shared-key: e2e` の rust-cache をやめ、`shared-key: conformance` と `shared-key: group-apps` の 2 つを `save-if: false` で読む。
  env のコメントに、CARGO で始まる環境変数は読むキャッシュと揃えることを書いた。
- docs/testing.md の「CI のキャッシュ」(表に e2e.yml を足し、e2e の項を書き直し、容量の目安を約 4.6GB に)、tests/e2e/README.md の「流れ」を直した。

#### tauri build と同じになるかの確かめ方 (受け入れ条件 2)

- **cargo の引数と環境:** PATH の先頭に cargo を包むスクリプト (引数・cwd・環境変数を記録してから本物の cargo を呼ぶ) を置いて
  `pnpm --filter @ugoe/viewer tauri build --debug --no-bundle --ci` を流した。tauri-cli 2.12.0 の cargo のビルドは
  cwd `apps/viewer/src-tauri` で `cargo build --bins --features tauri/custom-protocol` の 1 回だけ (ほかは `cargo metadata`)。
  シェルの環境との違いは pnpm の `npm_*`・corepack・asdf の変数と、`TAURI_CLI_VERBOSITY=0`・`MACOSX_DEPLOYMENT_TARGET=10.13` だけ。
  tauri-build 2.7・tauri-codegen 2.7・tauri 2.12 のソースで読む環境変数を調べ、`TAURI_CONFIG` (`--config` のときだけ CLI が置く) と
  `TAURI_ENV_TARGET_TRIPLE` (tauri-build が自分で置く) のほかに CLI から渡るものは無いことを確かめた。`TAURI_ENV_*` は
  beforeBuildCommand にだけ渡り (`tauri build -v` の出力)、3 つのアプリの Vite の設定とフロントエンドは読んでいない (`TAURI_DEV_HOST` だけ)。
- **frontendDist の埋め込み:** tauri の build.rs は `custom-protocol` の feature で `cfg(dev)` を外し、tauri-codegen は `dev` でなければ
  frontendDist を埋め込む。tauri build で作った 3 つの実行ファイルと、新しい 1 回の cargo で作った実行ファイルの両方に、
  `apps/<app>/dist/` のすべてのファイルのパス (エディタ 15・ビューア 23・スタジオ 26) が入っていた。`custom-protocol` 無しの
  `cargo build -p ugoe-viewer --bins` の実行ファイルには 23 個のうち 1 個 (index.html の文字列) しか入っていなかった (確かめ方として効く)。
  大きさは feature の統合の違いで少し違う (エディタ 247.0MB と 247.9MB など)。
- **tauri.conf.json の読み込み:** tauri-build (build.rs) と `generate_context!` がコンパイルのときに `src-tauri/tauri.conf.json` と
  OS ごとの `tauri.<OS>.conf.json` を読む (tauri-cli は関わらない)。エディタの `tauri.bundle.conf.json` はリリースのビルドの `--config` で
  だけ使うもので、e2e には元から関係しない。`bundle` の設定は `--no-bundle` で元から使っていない (`externalBin`・`resources` も無い)。
- **`--ci`:** tauri-cli の確認の問いと進み具合の表示を省くだけで、ビルドの結果に関わらない。新しい形では tauri-cli を通らないので不要。
- **差:** beforeBuildCommand は tauri-cli と同じく `sh -c` でアプリのディレクトリ (`apps/<app>`) で流すが、`TAURI_ENV_*` は渡さない
  (読むものが無い)。読むようになったら run.mjs で渡す必要がある。
- `just e2e` (エディタ 16・ビューア 6・スタジオ 13 の筋書き。rebase の後にもう一度) と `just e2e --manual` (47・4・13) が通った。
  `--manual` の /tmp/ugoe-manual の決まったパスの扱いは変えていない (実行ファイルを写す処理はそのまま)。`just e2e --only viewer` も通った。

#### CI を真似た手元の計測

mbx なし (`MBX_DISABLE=1`、PATH から shim を外す)・`CI=true`・`CARGO_PROFILE_DEV_DEBUG=0`・`CARGO_INCREMENTAL=0`、空の target。
rust (apps) の手順 (clippy 2 回・`cargo test --no-run`・doc・`just gen` の 2 つの example) が 215 秒。続けて conformance の分の
`cargo build -p ugoe-wasm --target wasm32-unknown-unknown --release` (99 個、47 秒。CI では conformance のキャッシュから戻る) の後に、
新しい e2e の cargo は 26 秒で、ugoe-* 以外のコンパイルは 17 個 (rustix・tauri・tauri-macros・tauri-plugin-fs/os/opener/global-shortcut/
dialog/single-instance・gethostname・polling・x11rb・async-io・async-signal・async-process・zbus・global-hotkey)。T0057 の見積もりと同じ。

#### rust-cache を 2 つ続けて戻すこと (v2.9.2 の src/restore.ts・src/cleanup.ts を読んだ)

- 2 つの restore は同じ `target/` に tar を展開して重ねる。同じ名前の成果物は同じハッシュの同じ中身なので、重なっても壊れない。
- restore が部分一致のときの前掃除 (`cleanTargetDir(..., checkTimestamp = true)`) は、プロファイルの直下のファイル (実行ファイルなど) を
  消すだけで、`deps`・`build`・`.fingerprint` は 1 週間より古いものしか消さない (しかもループの最初の 1 つで戻る)。1 つ目が戻した依存を
  2 つ目が消すことは実質ない。
- 環境のハッシュ: 1 つ目の restore が `CARGO_INCREMENTAL=0` を `GITHUB_ENV` に書くので、2 つ目のキーにも入る。ci.yml の rust (apps)・
  conformance も restore がキーを作る前に置くので入る (T0057 のメモと同じ)。e2e.yml の env (`CARGO_TERM_COLOR`・`CARGO_PROFILE_DEV_DEBUG`)
  は ci.yml と同じ値。ランナー (e2e は `ubuntu-24.04`、ci は `ubuntu-latest`) はキーの OS・アーキテクチャ (`Linux-x64`) が同じ。
- `save-if: false` なので Post は何もしない。

### CI で確かめること (push の後。このタスクでは確かめていない)

- main の run の E2E で、2 つの rust-cache が `v0-rust-conformance-…` と `v0-rust-group-apps-…` を戻す (同じ `Cargo.toml` のハッシュの
  ci.yml が先に保存していれば `full match: true`)。キーの環境のハッシュの部分が ci.yml の rust (apps)・conformance のものと同じこと。
- E2E の `just e2e` の cargo が、ugoe-* 以外では上の 17 個ほどしかコンパイルしないこと、`pnpm wasm` の cargo が依存をコンパイルしないこと。
  run の時間 (今の完全一致の run は約 8 分)。
- Post の手順で保存しないこと。`gh cache list` に `v0-rust-e2e-…` が新しく増えないこと (古いものは 7 日使われなければ消える。急ぐなら
  `gh cache delete`)。
- 1 組の合計 (受け入れ条件 6): `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list --limit 100` で、同じ末尾のハッシュの
  キャッシュ (と `godot-gdextension`) を足す。見込みは約 4.6GB。
- `Cargo.toml` が変わった push では、e2e.yml は ci.yml と並んで動くので新しいキーがまだ無く、前のキーから前方一致で戻して差分を
  コンパイルする (想定どおり。e2e が前より遅くなりすぎないか)。

### やらなかったこと

- 1 組の実際の合計の計測 (受け入れ条件 6)。push しないと測れない。上の「CI で確かめること」。

### 起票の候補

- なし。
