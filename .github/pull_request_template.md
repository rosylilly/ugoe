<!-- 貢献の手順は CONTRIBUTING.md にあります。日本語でも英語でもかまいません。See CONTRIBUTING.md. English is fine. -->

## 概要 / Summary

<!-- 何を、なぜ変えたか。What changed and why. -->

## タスク / Task

<!-- 対応するタスクの ID (docs/tasks/) と Issue。無ければ「なし」。The task ID (docs/tasks/) and issue, or "none". -->

## 確認 / Checklist

- [ ] `just ci` が通る / `just ci` passes
- [ ] コミットメッセージは英語の Conventional Commits で、`--no-verify` を使っていない / Commit messages follow Conventional Commits in English, without `--no-verify`
- [ ] 振る舞いを変えたところにテストを書いた / Added tests for changed behavior
- [ ] モデルの型やファイル形式を変えたなら、仕様書 (`docs/spec/`・`spec/`) も直した / Updated the specifications if the model or file formats changed
- [ ] ライセンス: 外部 SDK (Live2D Cubism SDK など) のコードを写していない。MIT の場所 (`crates/`・`bindings/`) にコピーレフトのコードや依存を入れていない。新しい依存には理由を書いた /
      License: no code copied from external SDKs, no copyleft code or dependencies in MIT directories, and reasons given for new dependencies
