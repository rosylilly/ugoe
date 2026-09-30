---
id: T0515
title: MCP: ツールが触れるパスを 1 つのディレクトリの下に限る
status: done
milestone: M6
crates: [ugoe-mcp, docs]
depends_on: [T0514]
assignee: claude-worker-T0515
---

# T0515: MCP: ツールが触れるパスを 1 つのディレクトリの下に限る

## 目的

`ugoe-mcp` を HTTP (T0514) で外に出すと、`open_project`・`save_project`・`import_psd` などが利用者の権限で任意のパスを読み書きできる。
ツールが触れるパスを、起動時に指定した 1 つのディレクトリの下に限る (T0514 のメモ)。

## 受け入れ条件

- [x] `--root <dir>` で、ツールが読み書きするパスを `<dir>` の下に限る (正規化してから判定し、`..` やシンボリックリンクで外に出られない)
- [x] HTTP で待ち受けるときの既定の扱いを決めて書く (例: `--root` か `--read-only` のどちらかを必須にする)。stdio の既定は変えない
- [x] 外に出るパスを拒否するテスト (相対パス・`..`・シンボリックリンク)
- [x] README と `docs/ai-integration.md` を更新し、`just ci` が通る

## 参考

- `docs/tasks/T0514-mcp-http-transport.md` のメモ

## メモ

- やったこと
  - `src/root.rs` の `Root::resolve`: 相対パスは root から解決し、存在する部分は `canonicalize`、まだ無い末尾は存在する
    最も深い祖先を正規化して結合してから、正規化した root と部品ごとに比べる (`root-other` は外)。無い部分の `..`・`.` と、
    行き先の無いシンボリックリンク (書くとリンク先に作られる) は拒否する。Windows のドライブ・UNC も正規化の結果で比べる
    (Windows のテストは `cfg(windows)` で書いたが、手元では走らせていない)。
  - 対象のパス (tools.rs と autorig.rs を洗い出した): 全ツールの `path` (開く・作る・保存する・自動で開く)、`import_psd` と
    `get_psd_layers` の `psd`、`save_project` の `bundle` (保存の前に確かめる)、`render_png` の `motion`・`expression`
    (プロジェクトの中の名前でなければファイルのパスとして解決する。無いファイルなら、描画が作業ディレクトリから読まないように
    ここで断る)。プロジェクトのディレクトリの中 (直下と motions/expressions/textures) の、外を指すシンボリックリンクも
    読む前と保存の前に拒否する (`Project::load`/`save` はパスをたどるので)。
  - HTTP の既定: `--http` は `--root` か `--allow-any-path` のどちらかを必須にした (無ければ起動しない)。例にあった
    `--read-only` は条件にしなかった: 書けなくても任意のプロジェクトや PSD を読んで画像で返せるので。stdio の既定は変えない。
    起動のログに `paths confined to <dir>` か `any path` を足した。
  - TOCTOU: 1 つの呼び出しの中で、判定の後にほかのプロセスが root の中をリンクに差し替える競合は防がない (`openat`/`O_NOFOLLOW`
    には unsafe か cap-std などの依存が要り、editor-core もパスで読み書きする)。ツールはリンクを作れないので MCP クライアント
    だけでは起こせない。判定は呼ぶたびにし直す。README と docs/ai-integration.md に書いた。
  - テスト: `src/root.rs` の単体テスト (相対・`..`・絶対・隣の名前・無い部分の `..`・リンク・行き先の無いリンク・root 自体が
    リンク)、`tests/stdio.rs` の `root_confines_tool_paths` (全ツールの外へのパスの拒否と書かれていないこと、リンク、中のファイルが
    外を指すプロジェクト)、`tests/http.rs` (`--root` か `--allow-any-path` が無いと起動しない・`--root` で動かす流れ)。
    `just ci` と `cargo test -p ugoe-mcp --features gpu-tests` が通る。
  - 挙動の変化: `--root` が無くても、`psd` と `bundle` は正規化したパスで覚える・返す (前は絶対パスにするだけ)。
- 後続の候補
  - openat 相当でたどる (cap-std など) と TOCTOU も防げる。editor-core の `Project::load`/`save` をディレクトリのハンドルで
    読み書きする形にする必要がある。
  - `--root` を複数とる (作品のディレクトリと PSD の置き場を分けたいとき)。
