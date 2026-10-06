---
id: T0051
title: "CI: rust-cache が失敗したジョブで保存されないことと、容量の上限を超えて消されることを直す"
status: done
milestone: M10
crates: [.github/workflows, docs]
depends_on: []
assignee: claude-worker-T0051
---

# T0051: CI の rust-cache の当たりを良くする

## 目的

GitHub Actions の CI で、依存のクレートのキャッシュが効かず、毎回ゼロからコンパイルするジョブがある。
run 37468408049 (2026-10-06、main) のジョブごとのコンパイル数は次のとおり。

| ジョブ | キャッシュ | コンパイル数 (うち依存) |
| --- | --- | --- |
| rust (runtime/tools/apps/assets)・gpu・conformance | 完全一致 | 13〜26 (0) |
| rust (windows) | No cache found | 203 (186) |
| rust (macos) | No cache found | 373 (350) |

Linux のジョブの再コンパイルはワークスペースのメンバー (`ugoe-*`) だけで、rust-cache の想定どおり。
問題は次の 2 つ。

1. **失敗したジョブではキャッシュが保存されない。** `Swatinem/rust-cache` は既定で成功したジョブでだけ保存する。
   `rust (windows)` は 2026-10-06 の main で 6 回続けてテストで落ち (37429758787〜37464594287)、`rust (macos)` も 37464594287 で落ちたので、
   キャッシュが一度も作られず、毎回 186〜350 個の依存をビルドしていた (Windows は 1 回 16 分ほど)。
   依存のビルドはテストの成否と関係ないので、テストが落ちても依存のキャッシュは残したい。
2. **キャッシュの合計が上限を超え、古いものから消される。** リポジトリのキャッシュは 10.8GB (27 個) で、既定の上限 10GB を超えている。
   rust-cache のキャッシュは約 20 個 (unity だけで 8 個) あり、1 組で約 8GB になる。キーの末尾のハッシュは
   すべての `Cargo.toml` と `Cargo.lock` から作られ、`Cargo.toml` は 1 日に何度も変わるので、変わるたびに新しい組が保存され、
   2 組で上限を超える。Linux のジョブは前のキーから部分的に戻せるが、前の組が消えた Windows・macOS のジョブはゼロからになる。
   **上限は引き上げない** (利用者の判断)。キャッシュの数と大きさを減らして、1 組が上限に十分収まるようにする。

## 受け入れ条件

- [x] `rust-os` (Windows・macOS) のジョブは、テストや clippy が落ちても依存のキャッシュを保存する (`cache-on-failure: true` など)。
      ほかのジョブにも付けるかを判断し、付けないならその理由をメモに書く
- [x] PR でキャッシュを保存しているジョブ (`gpu`・`conformance`・`godot`・`web`、unity.yml の `check`・`native` など、`save-if` の無いもの) は、
      main でだけ保存する (`save-if: ${{ github.ref == 'refs/heads/main' }}`)。PR のキャッシュは main から見えず、容量を食うだけなので
- [x] キャッシュの数を減らす。候補: unity.yml の `native` の matrix (7 個)・`check` の分け方、`gpu`・`conformance`・`godot` のように
      同じ OS で依存の重なるジョブのキーの共有、editor-build.yml・release 系のワークフローのキー。
      共有するとジョブのディスクやキャッシュ 1 個の大きさが増えるので、計測して判断する (ランナーのディスクは約 14GB。T0002)
- [x] キャッシュ 1 個の大きさを減らせるか調べる (例: unity の release ビルドの成果物、`cache-targets`・`cache-directories` の範囲)
- [ ] 変更後に main で 1 組のキャッシュの合計を `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list` で測り、
      上限 10GB の半分 (5GB) 程度に収まるか、収まらないならどこまで減ったかをメモに書く (2 組が並んでも消されないように)
- [x] actionlint で検査する
- [x] docs/testing.md の CI のキャッシュの説明 (「`Swatinem/rust-cache` の key は〜」の段落) を、キャッシュの方針 (失敗時の保存・main だけで保存・キーの共有と容量の目安) に合わせて更新する
- [x] `just ci` が通る。実際の CI のキャッシュの当たりは利用者の push の後に確かめる

## 参考

- `.github/workflows/ci.yml`・`unity.yml`・`e2e.yml`・`editor-build.yml`・`crates-release.yml`・`web-runtime-release.yml`
- [docs/testing.md#ci-のジョブ](../testing.md#ci-のジョブ)、T0002 (CI のジョブの分け方とディスクの対策)
- rust-cache の入力 (`cache-on-failure`・`save-if`・`shared-key`): https://github.com/Swatinem/rust-cache

## メモ

### やったこと

方針の説明は docs/testing.md の「CI のキャッシュ」に書いた。ここには判断の根拠を残す。

- **調べたこと (2026-10-06 の main の run 37468408049 (CI)・37468407995 (Unity)・37460942051 (E2E) のログと `gh cache list`):**
  - rust-cache (v2.9.2) のキーは `v0-rust-<shared-key か key と job>-<OS>-<arch>-<環境のハッシュ>-<Cargo.toml と Cargo.lock のハッシュ>`。
    環境のハッシュは rustc の版と `CARGO`・`CC`・`CFLAGS`・`CXX`・`CMAKE`・`RUST` で始まる環境変数から作る。
    `CARGO_PROFILE_DEV_DEBUG=0` のジョブ (447c32e2) と無いジョブ (516c6f62) でハッシュが違っていた。
    出典: https://github.com/Swatinem/rust-cache (src/config.ts)
  - 保存の前に target から「ワークスペースの依存 (`cargo metadata` のワークスペースの外のパッケージ)」でないものを消す (src/save.ts・src/cleanup.ts)。
    そのため `godot` のジョブの GDExtension (bindings/godot はワークスペースの外) の依存 (godot-rust など 22 個) は、キャッシュが完全一致でも
    毎回コンパイルされていた (dev 3 分 34 秒、release 2 分 28 秒)。キャッシュに残っていたのは参照の画像を描く ugoe-cli などの依存と release の分。
  - `save-if` が `true` 以外なら保存しない。`cache-on-failure` は既定で `false` (action.yml)。キーが既にあれば (完全一致で戻したら) 保存しない。
  - GitHub のキャッシュ: 上限は既定で 10GB、超えると最後に使われた日の古いものから消す。PR・タグの run が保存したキャッシュは、
    その ref からしか読めない (タグの run は既定のブランチのキャッシュは読める)。
    出典: https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache 、
    https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#usage-limits-and-eviction-policy
  - 変える前の 1 組 (`a5e2094c`) の大きさ: rust-group 5 個 2344MB (apps 936)、cross 468、rust-os 844 (windows 348・macos 496)、
    gpu 589、godot 511、conformance 560、wasm 355、unity 1843 (check 327・native 7 個 1516)、e2e 1095 (調べた時点では既に消されていた)、
    pnpm 97。合計 約 8.7GB (rust-cache が 21 個)。この時点で PR のキャッシュは無かった (PR の run が無い)。
  - unity の native はキャッシュがあっても速くならない: キャッシュ無しの linux-x86_64 が 1 分 00 秒、キャッシュありの android が
    46〜53 秒、windows が 1 分 54 秒。ugoe-ffi の依存は少なく (約 25 個)、時間の大半はキャッシュされない ugoe-* のコンパイル。
- **失敗時の保存:** `rust (windows)`・`rust (macos)` に `cache-on-failure: true`。依存がそろってから保存されるように、`Test` と
  `Clippy (apps)` を `if: ${{ !cancelled() }}` にした (clippy が落ちてもテストのビルドで依存を作る。落ちた検査が増えて見えるのも利点)。
  ほかのジョブには付けない。理由: Linux のジョブは main で前に保存したキャッシュから前方一致で戻せる (`Cargo.toml` が変わっても
  依存の大半は当たる)。一方、コンパイルの誤りで途中で止まったビルドを保存すると、次に `Cargo.toml` が変わるまでその不完全な
  キャッシュが完全一致で使われ続け、保存もし直さない。Windows・macOS は main で一度も成功しておらず戻すものが無かったので、付ける利点が上回る。
- **main でだけ保存・キーの共有:**
  - `rust-group` の key を `key: <group>` (キーに job の id が入る) から `shared-key: group-<group>` にした。
  - `gpu` は `shared-key: group-tools`・`save-if: false`。手元で、空の target に `CARGO_PROFILE_DEV_DEBUG=0` で tools のテストのビルド
    (`cargo test --no-run` に `node scripts/ci-groups.mjs args tools`) をした後に、CI の gpu のテストのビルド (`CI=1 node scripts/gpu-test-args.mjs`
    の引数) をすると、コンパイルしたのは ugoe-* の 6 個だけで、依存は 0 個だった (含まれる)。
  - `conformance` は `save-if: main`。`web`・`godot`・web-runtime-release.yml は `shared-key: conformance`・`save-if: false`。
    web の cargo のビルド (`scripts/wasm.mjs build` の release と bindings/web の web-dist) は conformance でも同じコマンドで行う。
    godot の参照のビルド (`cargo build -p ugoe-cli -p ugoe-conformance -p ugoe-ffi`。tests/conformance/run.mjs) も conformance と同じ。
    godot の release の bench で ugoe の依存 (glam・serde など少し) を毎回コンパイルするようになる。
  - キーを共有するため `CARGO_PROFILE_DEV_DEBUG: "0"` を ci.yml のワークフロー全体の env に移した (gpu・godot・conformance・web にも
    効く。dev のキャッシュがおよそ半分になる見込み)。web-runtime-release.yml にも同じ値を置いた (無いと環境のハッシュが違い conformance のキャッシュを読めない)。
  - crates-release.yml の package は `save-if: main` (タグの run では保存しない。main で手で起動したときだけ)。publish は元から `save-if: false`。
  - e2e.yml は元から main だけ。変えていない。
- **キャッシュを使わない:** unity.yml の `check` と `native` (上の計測。7+1 個・約 1.8GB)、editor-build.yml (タグか手での起動でだけ動き、
  タグの run のキャッシュはそのタグからしか読めない。アプリ × OS の 9 個のリリースビルドのキャッシュは上限を圧迫する)。
- **キャッシュ 1 個の大きさ:** `CARGO_PROFILE_DEV_DEBUG=0` を広げたほかは減らす手が無かった。`cache-targets: false` は target を
  持ち越さないので意味が無い。registry は rust-cache がそのジョブでダウンロードしたものだけを残すので小さい。unity の release の
  成果物はキャッシュをやめたので無くなった。`cache-directories` は使っていない。
- **見積もり (変えた後の 1 組):** rust-group 約 2.3GB + cross 0.47 + rust-os 0.84 + conformance 約 0.4〜0.56 + e2e 1.1 + pnpm 0.1 =
  **約 5.3GB** (rust-cache が 10 個。変える前は 21 個・約 8.7GB)。5GB を少し超える。2 組で約 10.6GB になり、2 組目の保存が終わった後に
  古い組の一部 (最後に使われた日が古いもの) が消され得るが、消されるのは新しい組がすでにある古い組の分なので、当たりへの影響は小さい。
- actionlint 1.7.12 (shellcheck 0.9.0) で 6 つのワークフローを検査した。指摘は gpu・godot・conformance の既存の `ls` の SC2012 (info) の
  3 件だけで、変えていない行 (T0002・T0013 のメモと同じ)。
- `just ci` は通った。

### CI で確かめること (push の後。このタスクでは確かめていない)

- main の 2 回目の CI で、`rust (windows)`・`rust (macos)` の rust-cache が完全一致 (`full match: true`) で、依存をコンパイルしないこと
  (1 回目は環境のハッシュとキーが変わるので、すべてのジョブで部分一致か No cache found になる)。テストが落ちた run でも Post の手順で
  `... Saving cache ...` が出ること。
- `gpu (vulkan)`・`gpu (gl)` が `v0-rust-group-tools-...` を完全一致で戻し、依存 (ugoe-* 以外) をコンパイルしないこと。
- `web`・`godot` が `v0-rust-conformance-...` を戻し、web は依存をコンパイルしないこと、godot は godot-rust と release の少しの依存だけを
  コンパイルすること。ただし保存する側 (rust (tools)・conformance) と並んで動くので、キーが変わった run では前のキーから部分一致で戻す。
- Unity の check・native の時間が大きく延びないこと (見込み: 数十秒以内)。
- 1 組の合計: `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list --limit 100` で、同じ末尾のハッシュのキャッシュを足す。
  conformance・e2e の大きさ (DEBUG=0 で減ったか) もここで分かる。古い形のキャッシュ (`v0-rust-runtime-rust-group-…`・`v0-rust-gpu-…`・
  `v0-rust-unity-…` など) は 7 日使われなければ消える。急ぐなら `gh cache delete` で消す。

### やらなかったこと

- 変えた後の 1 組の合計を main で測ること (受け入れ条件 5)。push しないと測れない。上の見積もりだけ。

### 起票の候補

- godot の GDExtension の依存 (godot-rust) がキャッシュに残らない (毎回 dev 3.5 分・release 2.5 分)。bindings/godot/build.mjs で
  GDExtension を別の target (`bindings/godot/target`) でビルドし、rust-cache の `workspaces` に `bindings/godot -> target` を足せば残せる
  (target を共有したまま 2 つの workspace を書くと、rust-cache は workspace ごとに相手の依存を消すので残らない)。crates: bindings/godot・.github/workflows。
- 5GB に収めきるなら、e2e (約 1.1GB) と rust (apps) (約 0.94GB) の重なり (どちらも Tauri の依存) を調べる。e2e は release と dev の両方を
  ビルドするので、そのまま共有はできない。
