---
id: T0056
title: "CI: godot のジョブで GDExtension の依存 (godot-rust) をキャッシュに残す"
status: done
milestone: M10
crates: [bindings/godot, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0056
---

# T0056: CI: godot のジョブで GDExtension の依存 (godot-rust) をキャッシュに残す

## 目的

`godot` のジョブは、GDExtension (bindings/godot。ワークスペースの外、ADR 0026) の依存 (godot-rust など 22 個) を、キャッシュが完全一致でも
毎回コンパイルしている (dev 3 分 34 秒・release 2 分 28 秒。T0051 で調べた)。rust-cache は保存の前に、`cargo metadata` のワークスペースの
依存でないものを target から消すため (src/cleanup.ts)。target を共有したまま 2 つの workspace を書くと、workspace ごとに相手の依存を消すので残らない。

## 受け入れ条件

- [x] `bindings/godot/build.mjs` が CI では GDExtension を別の target (`bindings/godot/target` など) でビルドする (今の「mbx を通さないときは分ける」
      規則との関係を整理し、手元の `just godot-check` (mbx あり・なし) の振る舞いを変えるなら理由をメモに書く)
- [x] ci.yml の `godot` のジョブの rust-cache に `workspaces` (`bindings/godot -> <その target>`) を足し、main でだけ保存する。
      参照の画像を描く ugoe のクレートは今のとおり conformance のキャッシュを読む (1 つの手順で両方を書くか、2 つに分けるかを決め、理由を書く)
- [x] 手元で、別の target でのビルドと `just godot-check` が通る。キャッシュの大きさの見込み (1 組の合計への増分) をメモに書いた
- [x] docs/testing.md の「CI のキャッシュ」と bindings/godot/README.md を直した
- [x] actionlint で増えた指摘が無い。`just ci` が通る
- 確かめるのは push の後 (このタスクでは確かめられない): main の 2 回目の run で godot のジョブが godot-rust をコンパイルしないこと。
  メモに「CI で確かめること」として書く

## 参考

- T0051 のメモ (調べたこと・起票の候補)、docs/testing.md の「CI のキャッシュ」
- bindings/godot/build.mjs、.github/workflows/ci.yml (`godot`)、https://github.com/Swatinem/rust-cache (`workspaces`)

## メモ

- T0052 で起票した (元の候補: T0051 (godot の依存がキャッシュに残らない))。

### やったこと

方針の説明は docs/testing.md の「CI のキャッシュ」と bindings/godot/README.md に書いた。ここには判断の根拠を残す。

- **target の選び方 (bindings/godot/build.mjs):** `CARGO_TARGET_DIR` が無いとき、
  1. mbx を通さないのに mbx が入っている・ルートの target/ がシンボリックリンク → 今のとおり GDExtension も参照のクレートも
     `bindings/godot/target/` (mbx の読み取り専用のハードリンクを避ける)。
  2. それ以外で `CI` がある → GDExtension だけ `bindings/godot/target/`、参照の画像を描くクレート (`--conformance` で run.mjs に渡す環境)
     はルートの `target/` (conformance のキャッシュを使う。`target/conformance` の結果の置き場も今のまま)。
  3. それ以外 → 今のとおりルートの `target/` を共有。
  手元の `just godot-check` は `CI` が無い限り振る舞いを変えていない (mbx あり・なしとも)。CI の判定は scripts/gpu-test-args.mjs と同じく
  `process.env.CI` (GitHub Actions は `CI=true` を置く)。手元でも `CI` を置けば CI と同じ分け方になる。
- **rust-cache は 2 つの手順に分けた (ci.yml の godot):** 1 つ目は今のとおり `shared-key: conformance`・`save-if: false`。2 つ目は
  `shared-key: godot-gdextension`・`workspaces: bindings/godot -> target`・`save-if: main`。1 つの手順に両方の workspace を書くと、
  キーを conformance と共有すれば保存できず (conformance が保存する。同じキーは 1 度しか保存されない)、別のキーにすれば
  参照のクレートの分 (約 0.4〜0.5GB) を conformance と重ねて保存することになるため。
  - 根拠 (rust-cache v2.9.2 のソースを読んだ。https://github.com/Swatinem/rust-cache の src/save.ts・src/cleanup.ts・src/workspace.ts・
    src/config.ts・src/restore.ts):
    保存の前に、workspace ごとに `cargo metadata` で manifest がその workspace の root の外にあるパッケージだけを残して target を掃除する
    (`getPackagesOutsideWorkspaceRoot`)。`bindings/godot` を root にすると godot-rust などの registry のクレートに加えて、
    `crates/` の ugoe-* (path の依存で root の外) も残る (dev・release で約 70MB。CI ではチェックアウトで mtime が変わるので毎回作り直す)。
    registry (`~/.cargo/registry`) もその workspace の依存の分だけに掃除される。キーの最後のハッシュは `workspaces` の各 root の
    メンバーの `Cargo.toml` と `Cargo.lock` (path のパッケージを除く) と `rust-toolchain.toml` から作るので、2 つ目のキーはルートの
    `Cargo.toml` が変わっても変わらない。
  - 同じアクションを 1 つのジョブで 2 回使う: 状態 (`core.saveState`) はアクションの手順ごとに分かれ、Post の手順は逆順に動く
    (2 つ目の保存 → 1 つ目は `save-if: false` で何もしない)。1 つ目の restore が `CARGO_INCREMENTAL=0` を `GITHUB_ENV` に書くので、
    2 つ目の環境のハッシュにはそれが入るが、毎回同じなのでキーは安定する。
    出典: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#sending-values-to-the-pre-and-post-actions
- **手元で確かめたこと:**
  - CI を真似た環境 (mbx の shim を PATH から外し `XDG_DATA_HOME` を無い場所にして mbx が入っていないことにし、`CI=true`・
    `CARGO_PROFILE_DEV_DEBUG=0`・`CARGO_INCREMENTAL=0`) で、空の `bindings/godot/target/` に dev (55 個のコンパイル、59 秒) と release
    (55 個、62 秒) をビルドした。rust-cache の保存の前の掃除を真似て (プロファイルの直下のファイルと ugoe-godot の成果物を消す) から
    もう一度ビルドすると、dev・release ともコンパイルしたのは ugoe-godot だけだった。
  - `just godot-check` (mbx あり、ルートの target を共有) が通った (69 ケース)。`CI=true just godot-check --only masks-nested` (mbx あり、
    GDExtension だけ別の target、参照はルート) も通った。`node scripts/check-licenses.mjs --godot` も通った (55 個)。
  - mbx なしで `CI` も無いとき (分岐 1・3) は変えていないので流していない。
- **キャッシュの大きさの見込み:** 上の掃除を真似た target (dev と release の deps・build・.fingerprint、1.64GB) を gzip して約 0.37GB。
  registry の .crate は約 55 個分で小さい。**1 組への増分は約 0.4GB** (T0051 の見積もりの約 5.3GB → 約 5.7GB)。ただしこのキーは
  ルートの `Cargo.toml` では変わらないので、`Cargo.toml` が変わった直後に 2 組が並ぶときも 1 個のまま (2 組の合計は約 10.6 → 11.0GB で、
  増分は 0.4GB の 1 個分だけ)。CI のジョブのディスクは、GDExtension の target が分かれて約 1.6GB 増える (ugoe-* を 2 度ビルドする分を含む)。
- actionlint 1.7.12 (shellcheck 0.9.0): ci.yml の指摘は gpu・godot・conformance の既存の `ls` の SC2012 (info) の 3 件だけで、main と同じ。
- `just ci` は通った。

### CI で確かめること (push の後。このタスクでは確かめていない)

- main の 1 回目の run の `godot` で、2 つ目の rust-cache の Post に `... Saving cache ...` が出て `v0-rust-godot-gdextension-Linux-x64-...` が
  保存されること (`gh cache list` で大きさ。見込み約 0.4GB)。
- main の 2 回目の run の `godot` で、2 つ目の rust-cache が `full match: true` で戻り、`Godot binding`・`Benchmark` の手順の cargo が
  godot-rust (`godot-core` など) をコンパイルせず、ugoe-* と ugoe-godot だけをコンパイルすること (dev 約 3.5 分・release 約 2.5 分の短縮)。
- 1 つ目の rust-cache (`conformance`) が今のとおり戻り、参照のビルド (`ugoe-cli` など) の依存をコンパイルしないこと。
- PR の run では 2 つ目も保存しないこと (`save-if` が false)。

### やらなかったこと

- CI での確かめ (上)。push しないと確かめられない。

### 起票の候補

- なし。
