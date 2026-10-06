---
id: T0057
title: "CI: e2e と rust (apps) のキャッシュの重なり (Tauri の依存) を調べ、1 組を 5GB に収めるか決める"
status: done
milestone: M10
crates: [.github/workflows, docs]
depends_on: [T0056]
assignee: claude-worker-T0057
---

# T0057: CI: e2e と rust (apps) のキャッシュの重なり (Tauri の依存) を調べ、1 組を 5GB に収めるか決める

## 目的

T0051 の見積もりで、変えた後のキャッシュの 1 組は約 5.3GB で、目標の 5GB を少し超える。大きいのは e2e (約 1.1GB) と rust (apps) (約 0.94GB) で、
どちらも Tauri の依存を持つ。e2e は release と dev の両方をビルドするので、そのまま共有はできない。重なりを調べ、減らせるなら減らす。

## 受け入れ条件

- [x] e2e と rust (apps) が作る依存の成果物 (プロファイル・feature・環境のハッシュ) の違いを調べ、共有できる部分と大きさの見込みをメモに書いた
- [x] 減らす手 (キーの共有・e2e の dev のビルドをやめる・保存する target の絞り込みなど) を選んで行うか、行わない理由を書いた
      (行うなら ci.yml・e2e.yml を直し、docs/testing.md の「CI のキャッシュ」を直した)
- [x] actionlint で増えた指摘が無い。`just ci` が通る
- 1 組の実際の合計は push の後に `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list` で測る。測れなければ「CI で確かめること」としてメモに書く

## 参考

- T0051 のメモ (見積もり・CI で確かめること・起票の候補)、docs/testing.md の「CI のキャッシュ」
- .github/workflows/ci.yml (`rust-group` の apps)・e2e.yml

## メモ

- T0052 で起票した (元の候補: T0051 (e2e と rust (apps) の重なり))。
- T0051 の「CI で確かめること」(rust (windows)・rust (macos) の完全一致・gpu と web が依存をコンパイルしない・1 組の合計) は起票していない
  (push の後に確かめる)。その結果が出ていれば、ここで使う。
- 同じワークフローを触る T0056 の後にした。

### やったこと

方針の説明は docs/testing.md の「CI のキャッシュ」に書いた。ここには判断の根拠を残す。結論: **キャッシュは分けたまま (変えない)**。
e2e.yml の rust-cache に理由のコメントだけを足した。

- **CI のログ (2026-10-06 の main):** E2E の run 37460942051 (`v0-rust-e2e-…-57e72178` を完全一致で戻した。約 1095MB) はコンパイルが
  ugoe-* の 38 個だけで、ビルドは editor 45 秒・viewer 10 秒・studio 25 秒 (と wasm の release 49 秒)、全体 8 分。run 37468408005 は
  `Cargo.lock` が変わり、前方一致で戻せる古い e2e のキャッシュも上限で消されていて `No cache found.`。771 個をコンパイルし、
  wasm の release 2 分 36 秒・editor の dev 15 分 25 秒・viewer 52 秒・studio 8 分 47 秒、全体 34 分 37 秒 (保存は 973MB)。
  rust (apps) (job 112285334275) は `v0-rust-apps-rust-group-…` (937MB) を完全一致で戻し、clippy 2 回・テスト・doc・`just gen`。
- **e2e のビルドの中身:** `tests/e2e/run.mjs` がアプリごとに `pnpm --filter <app> tauri build --debug --no-bundle --ci` を流す。
  その中で beforeBuildCommand の `pnpm wasm` (`scripts/wasm.mjs build`: `cargo build -p ugoe-wasm --target wasm32-unknown-unknown --release`)
  と、`-p <app>` の dev のビルド。tauri-cli (2.12.0) は build で `tauri/custom-protocol` の feature を足す (CLI の実行ファイルの文字列で確認)。
  `cargo install tauri-driver` は target/ の外でビルドするのでキャッシュに関係しない (33 秒)。
- **違い (成果物のハッシュが変わるもの):**
  1. `tauri/custom-protocol` で tauri・tauri-macros・プラグインが rust (apps) と別物になる。
  2. rust (apps) は 4 つのクレートを 1 回の cargo でビルドする (feature は 4 つと dev-dependencies で統合)。e2e はアプリを 1 つずつ
     ビルドするので、アプリごとに feature の統合が違う。スタジオだけのビルドでは serde・toml などの feature が違い、それに依存する
     system-deps から gtk・webkit2gtk の `-sys`、wgpu、tao・wry まで別物になる。
  3. wasm32 の release (`target/wasm32-unknown-unknown/release` と、ビルドスクリプト・proc-macro の `target/release`) は rust (apps) に無い。
     conformance の `just player-browser` が同じコマンドで作る (conformance のキャッシュに入る)。
  プロファイル (dev・`CARGO_PROFILE_DEV_DEBUG=0`) と環境のハッシュ (`CARGO_TERM_COLOR`・`CARGO_PROFILE_DEV_DEBUG`・rust-cache が
  置く `CARGO_INCREMENTAL=0`) は同じ (どちらのキーも 447c32e2)。
- **手元で測った (CI を真似た環境: mbx なし・`CI=true`・`CARGO_PROFILE_DEV_DEBUG=0`・`CARGO_INCREMENTAL=0`、空の target。24 コア):**
  - A: rust (apps) の手順 (clippy 2 回・`cargo test --no-run`・doc・`just gen` の 2 つの example) → 3.5 分、target/debug 3.6GB。
  - A の後に e2e のビルド (3 つのアプリの tauri build): ugoe-* 以外で editor 約 100 個 (大半は wasm32 の release)・viewer 9 個・
    studio 約 150 個をコンパイル。dev は editor 23 秒・viewer 17 秒・studio 76 秒。
  - 空の target から e2e のビルドだけ: dev は editor 111 秒・viewer 15 秒・studio 81 秒 (と wasm の release 41 秒)。713 個。
    共有で減る dev の時間は約 44% だけ (207 秒 → 116 秒)。
  - 大きさ: e2e だけの target を rust-cache の掃除を真似て数える (プロファイルの deps・build・.fingerprint、ugoe* を除く) と圧縮前
    約 3.1GB。そのうち A と同じ名前 (同じハッシュ) で A が作ったものは debug の 1.66GB。e2e だけのものは debug 1.03GB・wasm32 の
    release 0.30GB・release (ホスト) 0.11GB。CI の e2e のキャッシュ (約 1.0〜1.1GB) を同じ割合で分けると、共有できる分は約 0.55GB、
    e2e だけの分は約 0.5GB。
  - 参考 (C): A の後に `cargo build -p ugoe-editor -p ugoe-viewer -p ugoe-studio -p ugoe-studio-vcam --features tauri/custom-protocol`
    (4 つを 1 回で) をすると、ugoe-* 以外のコンパイルは 17 個 (tauri・tauri-macros・プラグイン 6 個・rustix・zbus・async-io など) で 31 秒。
- **選んだこと: 変えない。** 検討した手:
  - e2e が `group-apps` (と wasm の分は `conformance`) を `save-if: false` で読むだけにし、e2e のキャッシュをやめる: 1 組が約 1.1GB 減り
    (T0056 の後の見積もり 約 5.7GB → 約 4.6GB)、5GB に収まる。ただし e2e の run ごとに dev の依存を約 170 個コンパイルし直す。
    手元の 116 秒は、CI の空の target の時間 (dev 計約 25 分) と手元 (207 秒) の比 (約 7 倍) で見ると CI で約 13 分。今の完全一致の
    run (約 8 分) が 20 分ほどになる。main への push ごとと毎晩に流れるので、容量 1GB と引き換えにするには高い。
    (2 つの rust-cache を 1 つのジョブで続けて使っても、どちらも環境のハッシュに `CARGO_INCREMENTAL=0` が入るのでキーは合う。
    rust-cache v2.9.2 の src/restore.ts は `CacheConfig.new()` の前に `CARGO_INCREMENTAL` を書く。)
  - キーの共有 (e2e と rust (apps) を同じキーにして片方が保存): 保存する側に無い分を毎回コンパイルするので上と同じ。
  - e2e の dev のビルドをやめる (release にする): release の依存は rust (apps) とまったく揃わず、ビルドも重くなる。
  - 保存する target の絞り込み: rust-cache は保存の前にワークスペースの依存でないものを消すだけで、「ほかのキャッシュにあるもの」を
    除いて保存する手段が無い (src/save.ts・src/cleanup.ts)。wasm の release の分 (約 0.14GB) を除くには、e2e を 2 つの target に
    分ける変更が要り、得るものが小さい。
  - 5GB を超えたままの影響: 2 組が並んで 10GB を超えたとき、GitHub は最後に使われた日の古いものから消す
    (https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#usage-limits-and-eviction-policy)。
    消えるのは新しい組がすでに保存された古い組の分なので、当たりへの影響は小さい (T0051 と同じ判断)。
  - 出典: rust-cache v2.9.2 のソース https://github.com/Swatinem/rust-cache (src/config.ts・src/restore.ts・src/save.ts・src/cleanup.ts)。
- docs/testing.md の「CI のキャッシュ」に e2e を分けたままにする理由の項を足し、「容量の目安」に 5GB を少し超える見積もりと判断を書いた。
- actionlint 1.7.12 (shellcheck 0.9.0): e2e.yml は指摘なし。ci.yml は変えておらず、既存の SC2012 (info) の 3 件だけ。

### CI で確かめること (push の後。このタスクでは確かめていない)

- 1 組の合計: T0051・T0056 が main に入った後の 2 回目の run の後に、`gh api repos/{owner}/{repo}/actions/cache/usage` と
  `gh cache list --limit 100` で、同じ末尾のハッシュのキャッシュ (と `godot-gdextension`) を足す。見込みは約 5.7GB
  (e2e は約 1.0〜1.1GB のまま)。調べた時点 (2026-10-07) では T0051 の変更はまだ CI に流れておらず、古い形のキーで 24 個・約 10.4GB だった。

### やらなかったこと

- 1 組の実際の合計の計測 (受け入れ条件の 4 つ目)。push しないと測れない。上の「CI で確かめること」。

### 起票の候補

- e2e のビルドを 3 つのアプリをまとめた 1 回の cargo にする (`tests/e2e/run.mjs`): 各アプリのフロントエンドを `pnpm build` (と `pnpm wasm`)
  で作ってから `cargo build -p ugoe-editor -p ugoe-viewer -p ugoe-studio -p ugoe-studio-vcam --features tauri/custom-protocol` で
  ビルドすれば、上の C のとおり rust (apps) の後のコンパイルは依存 17 個 (手元 31 秒) になる。そうなれば e2e は `group-apps` と
  `conformance` を読むだけにでき、1 組が約 1.1GB 減る (約 4.6GB)。tauri build と同じ実行ファイルになるか (frontendDist の埋め込み・
  tauri.conf.json の読み込み・`--ci`) と `just e2e` の全筋書きで確かめる。crates: tests/e2e・.github/workflows・docs。
