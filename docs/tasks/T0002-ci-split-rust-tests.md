---
id: T0002
title: CI: Rust のテストをクレートのまとまりごとのジョブに分け、ランナーのディスク不足を防ぐ
status: in-progress
milestone: M7
crates: [.github/workflows, justfile, scripts, docs]
depends_on: []
assignee: claude-worker-T0002
---

# T0002: CI の Rust のテストを分ける

## 目的

GitHub Actions の CI の `rust` ジョブが、push のたびに失敗している。

- run 36464556766 (2026-09-28): テストのビルド中に `No space left on device` (リンカーも Bus error) で失敗。
- run 36801608426 (2026-10-01): Test のステップが約 15 分で取り消され、ジョブが止まったまま。

`cargo test --workspace` を 1 つのジョブで行うので、ワークスペースが大きくなるとランナーのディスク (約 14GB) と時間を超える。テストをクレートのまとまりごとの別のジョブに分けて、今後クレートが増えても 1 つのジョブが肥大化しないようにする。

## 受け入れ条件

- [ ] ワークスペースのクレートを、いくつかのまとまり (例: ランタイム層・ツール層・アプリ (Tauri のエディタとビューア)・バインディング/適合テスト) に分け、まとまりごとに clippy とテストを行うジョブにする (matrix など)。まとまりの定義は 1 か所 (scripts の JSON や mjs、justfile のレシピ) に置き、CI とローカルで同じものを使う
- [ ] すべてのクレートが、どれか 1 つのまとまりに入っていることを検査する (クレートを足したときの入れ忘れを防ぐ。scripts/check-layers.mjs の登録と同じように)
- [ ] ディスクの対策: 各ジョブの最初にランナーの不要なもの (.NET・Android SDK・Haskell・CodeQL など) を消す。CI ではデバッグ情報を減らす (`CARGO_PROFILE_DEV_DEBUG` など) か、そうしない理由を書く。rust-cache の key をまとまりごとに分ける
- [ ] Tauri のシステムの依存 (webkit など) は、アプリのまとまりのジョブだけで入れる
- [ ] fmt・生成物・layers・licenses・publish-check・tasks・wasm32・C ABI の検査は、どれかのジョブに残す (今の rust ジョブの検査を落とさない)
- [ ] `just ci` (ローカル) の振る舞いは変えない (全部を走らせる)。まとまりごとに走らせる `just test-group <名前>` のようなレシピを足してよい
- [ ] actionlint で検査する。各ジョブに timeout-minutes を付ける
- [ ] docs/testing.md (と必要なら AGENTS.md) に、CI のジョブの分け方と、クレートを足したときの手順を書く
- [ ] `just ci` が通る。実際の CI は利用者の push の後に確かめる

## 参考

- `.github/workflows/ci.yml`、`justfile` の lint・test、`scripts/check-layers.mjs`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
