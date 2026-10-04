// タスク用 worktree (.worktrees/ の下) だけに効く cargo の設定 `.worktrees/.cargo/config.toml`。
// 理由と計測は docs/workflow/build-cache.md#worktree-のデバッグ情報。
//
// cargo は実行したディレクトリから上へ `.cargo/config.toml` を探すので、`.worktrees/.cargo/config.toml` は
// .worktrees/ の下の worktree で動くすべての cargo (just のレシピ・lefthook の pre-push・rust-analyzer) に効き、
// main の作業ツリーには効かない。.worktrees/ は gitignore されているので、どのチェックアウトにも未追跡のファイルは増えない。

/** 書いたファイルの 1 行目。これで始まるファイルだけを書き直す (手で書いた設定は上書きしない)。 */
export const MARKER = "# scripts/worktree.mjs (`just wt-add`) が書いた設定。";

/** `.worktrees/.cargo/config.toml` の内容。 */
export const CARGO_CONFIG = `${MARKER}手で直すときは 1 行目を消す (消すと書き直さなくなる)。
# .worktrees/ の下の worktree だけに効く。理由と計測は docs/workflow/build-cache.md#worktree-のデバッグ情報。
# デバッグ情報を行の表だけにして、worktree ごとのビルドの成果物を約 21GB から約 10GB に減らす。
# パニックのバックトレースのファイルと行は残る。デバッガで変数や型を見るなら、その場で
# \`CARGO_PROFILE_DEV_DEBUG=true\` を付けて cargo を呼ぶ (環境変数はこの設定より強い)。
[profile.dev]
debug = "line-tables-only"
`;

/**
 * 今のファイルの内容 (無ければ undefined) から、どうするかを決める。
 * - "write": 無いか、前に書いたもの (1 行目が MARKER) で内容が違う
 * - "keep": すでに同じ内容
 * - "skip": 手で書いたもの (1 行目が MARKER でない)。上書きしない
 */
export function cargoConfigAction(current) {
  if (current === undefined) return "write";
  if (current === CARGO_CONFIG) return "keep";
  return current.startsWith(MARKER) ? "write" : "skip";
}
