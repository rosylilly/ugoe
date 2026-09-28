// コミットメッセージの規約。詳細は AGENTS.md#コミット を参照。
// - Conventional Commits 形式 (@commitlint/config-conventional)
// - スコープは下の SCOPES から選ぶ (省略可)
// - メッセージはすべて英語 (日本語などの CJK 文字・全角文字を含めない)

const SCOPES = [
  // crates/ (ugoe-* の接頭辞を除いた名前。`ugoe` クレートは sdk)
  "core",
  "model",
  "runtime",
  "format",
  "physics",
  "motion",
  "render",
  "sdk",
  "ffi",
  "wasm",
  "psd",
  "mesher",
  "editor-core",
  "ai",
  "cli",
  "rigging",
  // apps/
  "editor",
  // docs/
  "spec",
  "adr",
  "tasks",
  "workflow",
  // リポジトリ全体
  "repo",
  "deps",
  "ci",
  "hooks",
];

// CJK 統合漢字・ひらがな・カタカナ・ハングル・CJK 記号・全角形
const NON_ENGLISH =
  /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}　-〿＀-￯]/u;

export default {
  extends: ["@commitlint/config-conventional"],
  plugins: [
    {
      rules: {
        "english-only": (parsed) => {
          const lines = (parsed.raw ?? "").split("\n").filter((line) => !line.startsWith("#"));
          const bad = lines.find((line) => NON_ENGLISH.test(line));
          return [
            bad === undefined,
            `commit messages must be written in English (found: "${bad?.trim()}")`,
          ];
        },
      },
    },
  ],
  rules: {
    "english-only": [2, "always"],
    "scope-enum": [2, "always", SCOPES],
    "header-max-length": [2, "always", 100],
  },
};
