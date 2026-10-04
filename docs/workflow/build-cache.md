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
- **incremental のコンパイルは共有の置き場を通らない。** ワークスペースのクレートの dev のビルドは各 worktree で作られる
  (依存クレートだけが置き場から戻る)。テストの実行ファイルのリンクも worktree ごとに行われるので、worktree ごとの managed target には
  なお 15GB ほどの固有の成果物が溜まる (T0018 の計測)。

## 計測 (T0018、Linux・ext4・WSL2、mbx 1.22.0)

| 場面 | 時間 | mbx の統計 |
| --- | --- | --- |
| 従来の `just wt-add` (target/ のコピー) | 1 つ 2〜5 分、約 30GB | - |
| mbx が有効な `just wt-add` | 約 1 秒 (git worktree と `pnpm install --offline` だけ) | - |
| 新しい worktree で最初の `just crate ugoe-runtime` | 3〜14 秒 | 27〜49 hits、0 misses |
| 新しい worktree で最初の `just ci` (別の worktree で一度通した後) | 195 秒 | clippy 744 hits / 31 misses、test 675 hits / 35 misses |

`just ci` は前の worktree で通っていない状態からだと 297 秒でした (置き場にあったのは main で一度ビルドした分だけ)。

## CI には入れない

GitHub Actions には入れません (T0018 で決めた)。

- mbx の効き目は、1 つの機械の中で複数のチェックアウトが成果物を共有することにある。CI のランナーは毎回新しく、
  ジョブをまたいだ共有には mbx のリモートのキャッシュ (S3 互換の置き場や HTTP のサーバ) と、その資格情報の管理が要る。
- グループごとのジョブ ([testing.md](../testing.md#ci-のジョブ)) は `Swatinem/rust-cache` で依存クレートの成果物をすでに持ち越している。
- 入れるとランナーごとに mbx のインストール (`cargo install` のビルドかバイナリの取得) と、上の dash の回避が要る。

CI の時間が問題になり、リモートのキャッシュの置き場を用意できるようになったら見直します。
