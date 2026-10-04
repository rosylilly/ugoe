# ビルドのキャッシュ (mbx)

[mbx (mr-boxington)](https://mr-boxington.jdx.dev/) は cargo と rustc を包むビルドのキャッシュです。
ワークスペース・target・レジストリ・ツールチェインのパスを置き換えたキーで成果物を 1 つの置き場 (`~/.cache/mbx`) に持つので、
同じリポジトリの worktree どうしで依存クレートの成果物を共有できます ([How it works](https://mr-boxington.jdx.dev/how-it-works))。

ugoe では **任意** です。入れていなければ、今までどおり `just wt-add` が main の `target/` をコピーします。
並行して多くの worktree を作る機械 (マルチエージェントの作業。[multi-agent.md](multi-agent.md)) では入れることを推奨します。
`target/` のコピー (1 つ 2〜5 分、約 30GB) が要らなくなり、main で cargo が動いている最中にコピーして失敗することもなくなります。

## 使い方

```sh
cargo install mbx --locked   # インストール (https://mr-boxington.jdx.dev/installation。mise でも入れられる)
mbx setup                    # cargo の shim を入れる (Linux では ~/.local/share/mbx/bin/cargo)
mbx doctor                   # 確認する。`cargo on PATH` が shim を指し、0 failures であること
```

- `mbx setup` はシェルの起動ファイルを書き換えません。shim のディレクトリを PATH の先頭に置く行を、自分で起動ファイル
  (`~/.bashrc`・`~/.config/fish/config.fish` など) に足してください。mise を使っているなら `mbx setup --global` で
  mise の `[wrappers.cargo]` に入れられます。
- **justfile は、shim があれば自分で PATH の先頭に足します** (Unix のみ)。起動ファイルを直していなくても、`just crate`・`just ci`・
  `just wt-add` などのレシピの cargo は mbx を通ります。
- 一時的に mbx を通さないときは `MBX_DISABLE=1` を付けます (`MBX_DISABLE=1 just ci`)。shim は素の cargo を呼びます。
- 統計は `mbx stats`、ビルドごとの当たり外れは cargo の出力の最後の `mbx[cache]: N hits, M misses, ...` の行で分かります。

### 片付け

- `mbx gc`: 置き場と managed target を大きさの上限 (既定はディスクの 5〜10%) と期限 (`target.max_age`、既定は 30 日) まで減らす。
  既定で 1 時間ごとに自動で走る (`gc.auto`)。`mbx gc --dry-run` で消すものだけを表示する。
- `mbx clean [WORKSPACE]`: 1 つのワークスペースの managed target と incremental の状態を消す (置き場は残るので、次のビルドはまた置き場から戻る)。
  `just wt-remove` は、worktree の `target` が managed target なら `mbx clean` も行う。

## 何が変わるか

- **managed target:** mbx を通した cargo は、ワークスペースの `target` を `~/.cache/mbx/targets/v1/<ハッシュ>` へのシンボリックリンクにする。
  すでに実体の `target/` があれば、最初に mbx を通したときにそこへ移す。
  check (`cargo clippy`) と build・test の成果物は別の置き場所 (`target/check/`・`target/debug/`) に分かれる。
- **`just wt-add` は、mbx が有効なら `target/` をコピーしない** (`scripts/worktree.mjs`)。判定は `scripts/lib/mbx.mjs` にある:
  `MBX_DISABLE` が無い (または `0`・`false`) こと、かつ PATH で最初に見つかる `cargo` が `mbx setup` の shim (隣に `mbx-target` がある) であること。
  mise の wrapper で包んだ場合は判定できないのでコピーする (遅いだけで壊れはしない)。
  main の `target/` がシンボリックリンク (managed target) のときも、mbx の有無によらずコピーしない。
- **just を通さずに cargo を直接呼ぶとき** (lefthook の pre-push、エディタの rust-analyzer、エージェントが Bash で呼ぶ cargo など) は、
  シェルの PATH で決まる。PATH に shim が無ければ素の cargo が動く。素の cargo も同じ `target` (シンボリックリンクの先) を使うので
  成果物は混ざるが、多くの場合はそのまま動く。下の「注意点」の読み取り専用の失敗が起きたら `mbx clean` で直す。
  エージェントは、反復には `just crate`・`just test-group` など just のレシピを使う。
  セッションの途中で起動ファイルを直した場合、すでに開いているシェルには反映されないので、`export PATH="$HOME/.local/share/mbx/bin:$PATH"` を先頭に付けて呼ぶ。
- **just を通らずに cargo を呼ぶスクリプトのうち `bindings/godot/build.mjs` は、自分で mbx を通す** (T0028)。ワークスペースの外 (ADR 0026) で
  直接 `node bindings/godot/build.mjs` と呼ばれることがあるので、shim が入っていれば justfile と同じく PATH の先頭に足す
  (`scripts/lib/mbx.mjs` の `cargoEnvWithMbx`)。`MBX_DISABLE` で mbx を通さないときは、共有の `target/` を避けて `bindings/godot/target/` でビルドする。

## 注意点

- **ビルドスクリプトのキャッシュを切っている (`.mbx.toml` の `build_script_execution = false`)。** mbx 1.22.0 はビルドスクリプトを
  `#!/bin/sh` のラッパーで包んで実行する。`/bin/sh` が dash (Debian・Ubuntu) だと、名前に `:` を含む環境変数が落ちる。
  Tauri は各プラグインの権限のファイルの場所を `DEP_TAURI_CORE:WINDOW__CORE_PLUGIN___PERMISSION_FILES_PATH` のような名前で渡すので、
  アプリ (`ugoe-editor`・`ugoe-viewer`・`ugoe-studio`) のビルドスクリプトが `Permission core:window:... not found` などで失敗する。
  ワークスペースの `.mbx.toml` でビルドスクリプトのキャッシュを切ると、ラッパーを通らずに cargo がそのまま実行する。
  この設定は、ビルドスクリプトをコンパイルするときに効く。設定より前に mbx でビルドした `target` でこの失敗が出たら、`mbx clean` してからビルドし直す。
- **戻した成果物は読み取り専用になりうる。** reflink の無いファイルシステム (ext4 など) では、置き場から戻した成果物は置き場のオブジェクトへの
  ハードリンクで、読み取り専用になる (`mbx doctor` の `restore ... cannot clone; restores hard link` の行)。mbx はコンパイラが書き直す前に
  リンクを外すが、mbx を通さない cargo が同じ `target` でそれを書き直そうとすると `output file ... is not writeable` で失敗しうる。
  `mbx clean` か `cargo clean` で直す。btrfs・XFS・APFS ではコピーオンライトの複製になるので起きない。
- **worktree ごとに固有の成果物が残る。** テストの実行ファイルのリンクなどは worktree ごとに行われるので、worktree ごとの
  managed target には固有の成果物が溜まる。既定のデバッグ情報 (`debug = true`) では `just ci` の後で 12〜15GB になる (T0018・T0021 の計測)。
  .worktrees/ の下の worktree ではデバッグ情報を減らして小さくする ([下](#worktree-のデバッグ情報))。
  mbx を通したビルドでは、incremental の状態は `target/debug/incremental` ではなく `~/.cache/mbx/incremental/` に置かれ、1 つ数 MB だった。

## 計測 (T0018、Linux・ext4・WSL2、mbx 1.22.0)

| 場面 | 時間 | mbx の統計 |
| --- | --- | --- |
| 従来の `just wt-add` (target/ のコピー) | 1 つ 2〜5 分、約 30GB | - |
| mbx が有効な `just wt-add` | 約 1 秒 (git worktree と `pnpm install --offline` だけ) | - |
| 新しい worktree で最初の `just crate ugoe-runtime` | 3〜14 秒 | 27〜49 hits、0 misses |
| 新しい worktree で最初の `just ci` (別の worktree で一度通した後) | 195 秒 | clippy 744 hits / 31 misses、test 675 hits / 35 misses |

`just ci` は前の worktree で通っていない状態からだと 297 秒でした (置き場にあったのは main で一度ビルドした分だけ)。

## worktree のデバッグ情報

`just wt-add` は `.worktrees/.cargo/config.toml` を書き、**.worktrees/ の下の worktree だけ** dev のデバッグ情報を行の表だけ
(`debug = "line-tables-only"`) にする (`scripts/lib/worktree-cargo-config.mjs`。T0021)。mbx が無効なときは書かない
(コピーする main の `target/` は `debug = true` の成果物なので、設定が違うと使われずに作り直しになる)。

- cargo は実行したディレクトリから上へ `.cargo/config.toml` を探すので、この設定は worktree で動くすべての cargo
  (just のレシピ・lefthook の pre-push・rust-analyzer) に効き、main の作業ツリーには効かない。.worktrees/ は gitignore されているので、
  どのチェックアウトにも未追跡のファイルは増えない。just のレシピだけに環境変数を足す方法は採らない (pre-push など just を通らない cargo と
  設定が食い違うと、別の成果物として両方が溜まり、かえって大きくなる)。
- **残るもの:** パニックのバックトレースのファイルと行 (`at ./tests/foo.rs:4:18`)。テストの失敗の場所はこれまでどおり分かる。
- **失うもの:** デバッガで変数・引数・型を見ること (行の表しか無いので、ブレークポイントとステップ実行はできるが値は見えない)。
  必要なら、その場で `CARGO_PROFILE_DEV_DEBUG=true` を付けて cargo を呼ぶ (環境変数は設定ファイルより強い)。ただし別の成果物として
  ビルドし直すので、終わったら `mbx clean` で片付けるとよい。ずっと要るなら `.worktrees/.cargo/config.toml` の 1 行目を消して手で直す
  (1 行目のしるしが無いファイルは `just wt-add` が書き直さない)。
- **ワークスペースの `Cargo.toml` の `[profile.dev]` は変えない。** main の作業ツリーや、worktree を使わない開発者のデバッグの体験は
  今のまま (`debug = true`) にする。1 つのチェックアウトだけで作業する機械では、成果物の大きさは問題になりにくい。
- 設定を変えた後の最初のビルドは、依存クレートも含めて作り直す (rustc の引数が変わり、mbx の置き場に当たらない)。
  既存の worktree では古い設定の成果物も残るので、`mbx clean` してからビルドすると小さくなる。

### 計測 (T0021)

1 つの worktree で `mbx clean` してから `just ci` を流し、`target/` (managed target) の大きさを測った。
「固有」はリンクの数が 1 のファイル (mbx の置き場へのハードリンクでないもの) の合計で、worktree ごとに増える分。
Linux・ext4・WSL2・24 コア・mbx 1.22.0。**ほかに 7 つ前後のワーカーが並行してビルドしていた** (load average 7〜33) ので、時間は ±数十% の幅で見る。
`just ci` は無関係な vitest の失敗 (web-check) で止まったので、時間は fmt-check から web-check までの分 (Rust の検査はすべて含む)。

| 設定 | target 全体 | 固有 | `just ci` | 備考 |
| --- | --- | --- | --- | --- |
| `debug = true` (今の既定)、1 回目 | 21.0GiB | 11.9GiB | 324 秒 | 依存クレートは置き場から |
| `debug = true`、2 回目 | 21.0GiB | 14.3GiB | 275 秒 | 間に置き場の gc があり 96 misses |
| `debug = "line-tables-only"`、1 回目 | 9.8GiB | 8.0GiB | 459 秒 | 置き場に無く、依存クレートも作り直した |
| **`debug = "line-tables-only"`、2 回目 (採る)** | **9.8GiB** | **2.7GiB** | **138 秒** | test 724 hits / 1 miss |
| `split-debuginfo = "unpacked"` (`debug = true`)、1 回目 | 11.8GiB | 10.2GiB | 372 秒 | mbx が 121 件を置き場に入れられない (bypassed) |
| `debug = 0`、1 回目 | 4.9GiB | 3.9GiB | 359 秒 | バックトレースにファイルと行が出ない |

- `debug = 0` は最も小さいが、バックトレースに関数の名前しか出ず、テストの失敗の場所を追いにくくなるので採らない。
- `split-debuginfo = "unpacked"` は .dwo を分けるだけで合計はあまり減らず、mbx が分けたファイルを置き場に入れられない。
- 時間は、2 回目どうしで `line-tables-only` が `debug = true` の半分ほどだった。ただし `debug = true` の 2 回目は置き場の gc で misses が出ており、負荷も違うので、差は割り引いて見る。遅くならないことは言える。

## CI には入れない

GitHub Actions には入れません (T0018 で決めた)。

- mbx の効き目は、1 つの機械の中で複数のチェックアウトが成果物を共有することにある。CI のランナーは毎回新しく、
  ジョブをまたいだ共有には mbx のリモートのキャッシュ (S3 互換の置き場や HTTP のサーバ) と、その資格情報の管理が要る。
- グループごとのジョブ ([testing.md](../testing.md#ci-のジョブ)) は `Swatinem/rust-cache` で依存クレートの成果物をすでに持ち越している。
- 入れるとランナーごとに mbx のインストール (`cargo install` のビルドかバイナリの取得) と、上の dash の回避が要る。

CI の時間が問題になり、リモートのキャッシュの置き場を用意できるようになったら見直します。
