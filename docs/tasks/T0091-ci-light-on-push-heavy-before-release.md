---
id: T0091
title: "CI を見直す: push ごとは軽い検査だけにし、重いビルド (Windows・macOS・GPU・適合テスト・e2e・Unity・Godot など) はリリースの前と定期・手動に回す"
status: done
milestone: Release
crates: [.github/workflows, scripts, justfile, docs]
depends_on: []
assignee: claude-worker-T0091
---

# T0091: CI を、push ごとは軽く、重いものはリリースの前に

## 目的

非公開のリポジトリの GitHub Actions の無料枠 (月 3000 分) を、2026-10-11 に使い切った。1 回の push で CI・E2E・Unity が動き、Windows (×2)・macOS (×10) の倍率を含めて
約 200 分を使う (オーケストレーターの測定。main に月 60 回近く動いていた)。利用者の方針 (2026-10-11): **公開・非公開にかかわらず、重いビルドを毎 push 行わない。
リリースのパッケージのビルドの前に検査すれば足りる。** 先々はオープンソースとして公開する (公開すれば標準のランナーは無料になるが、方針は同じ)。

## 受け入れ条件

- [x] 今の各ワークフロー・ジョブの所要時間と起動の条件を一覧にした (`gh api .../actions/runs/<id>/jobs` で測る。倍率を掛けた分も)
- [x] **push・PR ごと (軽い):** Linux の 1 つのジョブ (か少数) で、fmt・clippy・テスト・依存方向・refs・ライセンス・フロントエンドの検査など、手元の `just ci` に相当するものだけにする。
      変わったパスで絞れるもの (docs だけの変更では Rust を動かさない、など) は `paths` で絞る。目安と、どれくらい減るかの見積もりをメモに書く
- [x] **重いもの:** Windows・macOS・cross・GPU (vulkan・gl)・conformance・e2e・Unity・Godot・perf・fuzz・nostd-build-std・アプリのビルドなどは、push では動かさず、
      (a) 手での起動 (workflow_dispatch)、(b) 定期 (週 1 回などの頻度を決める。無料枠に収まるか見積もる)、(c) **リリースの前**、で動かす
- [x] **リリースの前の検査:** `v*` のタグで動くリリースのワークフロー (editor-build・crates-release・web-runtime-release・ffi-release・release-notes) が、パッケージを作る前に
      重い検査一式を通すようにする (再利用できるワークフロー `workflow_call` にまとめ、リリースの各ワークフローが `needs` で待つ、など)。検査が落ちたらパッケージを作らない
- [x] 同じコミットに同じ検査が重ならないよう `concurrency` と起動の条件を整理した。キャッシュ (rust-cache など) がジョブの分割で効かなくならないことを確かめた
- [x] docs/testing.md#ci-のジョブ・docs/release.md・AGENTS.md (CI の説明があれば)・scripts/ci-groups.mjs の説明を、新しい分け方に合わせた
- [x] actionlint が通る。`just ci` が通る
- [ ] (オーケストレーター) 課金が戻った後か公開の後に、push の CI と、手での起動の重い検査を 1 度ずつ流して確かめる

## 参考

- .github/workflows/*.yml、scripts/ci-groups.mjs、docs/testing.md#ci-のジョブ、docs/release.md、T0084・T0086・T1408 (リリースのワークフロー)

## メモ

- 利用者の指示 (2026-10-11) でオーケストレーターが起票した。
- 自前のランナー (self-hosted) は、このタスクでは入れない (公開すると、フォークからの PR で自前の機械が使われる危険がある)。要るなら後続の候補に。
- (claude-worker-T0091, 2026-10-11) 実装した。

  **測定 (`gh api .../actions/runs/<id>/jobs`。課金はジョブごとに分に切り上げ、Windows ×2・macOS ×10):**

  | ワークフロー (起動の条件、T0091 の前) | run | 実時間の合計 | 課金の分 |
  | --- | --- | --- | --- |
  | CI (main への push・PR) | 38098330681 | 112 分 | 157 |
  | E2E (main への push・毎晩・手動) | 38098330684 | 24 分 | 24 |
  | Unity (main・PR で crates/ などを触ったとき・タグ・手動) | 38098330830 | 13 分 | 49 |
  | Editor build (タグ・手動。Windows 2 つ・macOS 1 つが失敗) | 38098550160 | 137 分 | 346 |
  | Fuzz (週 1・手動。ビルドで失敗) | 38098547009 | 4 分 | 6 (動けば 5×約 8 分で約 40 の見込み) |
  | Perf (週 1・手動) | 38098545387 | 5 分 | 6 |
  | No-std build-std (週 1・手動) | 38098548567 | 2 分 | 3 |
  | Crates release・Web runtime release (タグ・手動) | 38098551862・38098553415 | 2 分ずつ | 3 ずつ |
  | C ABI release・Release notes (タグ・手動・PR) | 成功した run が無い | — | 見込み 約 30・1 |

  CI のジョブ (run 38098330681): rust-checks 1:06・rust-groups 0:13・rust (runtime) 9:36・(bindings) 4:28・(assets) 3:07・
  (tools) 12:45・(apps) 25:55・web 5:51・rust (cross) 2:07・rust (windows) 8:41 (×2)・rust (macos) 2:28 (×10)・gpu (vulkan) 9:35・
  gpu (gl) 9:39・godot 9:20・conformance 7:24。この run は rust-cache がすべて外れていた (ランナーの stable が 1.99.0 になり、環境の
  ハッシュが変わった)。当たった run (10-06・10-08) では rust (apps) 5〜9 分・(tools) 6 分・(runtime) 3 分・(bindings)・(assets) 3 分・web 5 分。
  Unity: check 1:32・native は Linux・Android 3 つが 2 分前後、Windows 3:04 (×2)、macOS universal 1:16・iOS 0:46 (×10)。
  9/11〜10/11 の run は CI の push が 18 回・E2E が 12 (push)+7 (毎晩)・Unity が 14 で、1 回の push が課金で約 160〜230 分 (CI・E2E・Unity)。

  **分け方 (やったこと):**
  - `ci.yml` (push・PR、軽い): commitlint・rust-checks (ビルドしない検査に、`web` から公開仕様の検査を移した)・rust-groups・
    rust-group (Linux のグループだけ)・rust (まとめ)・web。cross・rust-os・gpu・godot・conformance は `release-checks.yml` へ移した。
  - 変わったパスでの絞り込みは、ワークフローの `paths` ではなく `rust-groups` のジョブで行う (`node scripts/ci-groups.mjs changed <base> HEAD`、
    `affected`)。`paths` で丸ごと飛ばすと、文書だけの変更で `just refs`・文書のリンク・`just tasks` も流れなくなり、必須の検査の名前
    (`rust`) も出なくなるため。文書だけ → rust-checks だけ。クレートの変更 → そのクレートと依存するクレートのグループ (`covers` で
    `runtime` は `ugoe-ffi` の変更でも動く)。`apps/<アプリ>/` のフロントエンド → `apps` と web。`packages/` など → web だけ。
    テストが読む文書 (`DOCS_READ_BY_TESTS`: `docs/editor.md`・`docs/spec/`・`docs/studio-api.schema.json`) → 読むクレート。
    それ以外 (Cargo.lock・scripts/・.github/・assets/・spec/ など) → すべて。Rust のソースが読む文書の書き漏れは `just groups` が見つける。
    単体テストは `scripts/ci-groups.test.mjs`。
  - `release-checks.yml` (重い): 手での起動・定期 (毎月 1 日と 15 日 03:17 UTC。`gate` が、main が前に通った run と同じなら飛ばす)・
    `workflow_call`。cross・rust-os・gpu・godot・conformance と、e2e.yml・unity.yml・perf.yml・nostd-build-std.yml・fuzz.yml を呼ぶ
    (この 5 つは push・PR・schedule の起動をやめ、`workflow_call` と手での起動だけにした)。
  - `release.yml` (`v*` のタグ): `checks` (release-checks.yml) の後に、release-notes・editor-build・ffi-release・crates-release・
    web-runtime-release を `needs: checks` で呼ぶ。5 つのタグの起動はやめ、`workflow_call` と手での起動 (予行) にした。
    ffi-release の PR の起動 (Windows・macOS を使う) もやめた。release-notes の PR の起動 (Linux で 1 分) は残した。
  - 呼ばれたワークフローの `github.workflow` は呼んだ側の名前になるので、`concurrency` のグループの名前を各ワークフローで書いて決めた
    (`${{ github.workflow }}` のままだと呼んだ側と同じグループになり、互いを待って止まる)。タグでは取り消さない。
  - キャッシュ: 鍵 (`shared-key`・ワークフロー全体の `CARGO_PROFILE_DEV_DEBUG`) はジョブを移しても変わらない (rust-cache の鍵は
    ワークフローの名前を含まない)。conformance が push ごとに保存しなくなるので、`ugoe-wasm` の wasm32 のビルドは `web` のジョブの
    新しいキー `web` (main で保存) に置き、web-runtime-release.yml と e2e.yml はそれを読むようにした。perf.yml と fuzz.yml の保存を
    main に限った (タグの run で保存すると使えないキャッシュが上限を圧迫する)。gpu は今までどおり ci.yml の `group-tools` を読む。
    実際の run で当たるかは、課金が戻ってから確かめる (下の「確かめていないこと」)。
  - 動いている `ls` の shellcheck の info (SC2012) は `find` に直した。actionlint 1.7.12 (shellcheck 0.9.0) で警告 0。

  **見積もり (課金の分):**
  - push 1 回: ci.yml だけ。キャッシュが当たって約 35 分、外れて約 70 分 (T0091 の前は約 160〜230 分。ほかに毎晩の E2E が 1 日 24 分)。
    文書だけの push は rust-checks と rust-groups とまとめで約 4 分。グループの絞り込みは、9/28〜10/11 の 17 回の push に当てはめると
    2 回だけ効いた (`tools`・`apps` だけ)。push が大きなまとめ (数十〜数百のコミット) で、ほぼ毎回 scripts/・.github/・Cargo.lock を含むため。
  - release-checks.yml 1 回: 約 210 分 (キャッシュが外れると約 330 分)。定期は月 2 回で約 420〜660 分 (main が変わらなければ 0)。
  - リリース 1 回: release-checks 約 210 + editor-build 約 350 + ffi 約 30 + そのほか 約 10 = 約 600 分。
  - 月の合計: push が月 40 回なら 約 1400〜2800 + 定期 約 420〜660 で、約 1800〜3500。キャッシュが当たっていれば無料枠 (3000) に収まり、
    リリースのある月はその分 (約 600) が足される。

  **確かめていないこと:** GitHub Actions が止まっているので、どのワークフローも実際には流していない (actionlint と、`changed` を過去の
  push の差分に当てた手元の確認だけ)。特に: (1) rust-group の `if` が空の matrix の展開より先に評価されること、(2) release.yml から
  入れ子で呼んだワークフローの権限・Secrets (`secrets: inherit`)・environment `crates-io` の承認、(3) npm の trusted publishing が
  呼んだ側 (`release.yml`) の名前で照合すること (bindings/web/README.md は `release.yml` を登録するように書き換えた)、(4) `gate` の
  `gh api` (権限 `actions: read`)、(5) キャッシュの当たり。受け入れ条件の最後 (オーケストレーター) で確かめる。

  **crates: の外の小さな変更:** 文書の同期のため、AGENTS.md (just perf の行)・crates/ugoe-ffi/README.md・crates/ugoe/examples/puppet_timing.rs
  (コメント)・bindings/{web,unity,godot}/README.md・bindings/godot/build.mjs・bindings/web/examples/timing/measure.mjs (コメント)・fuzz/README.md・
  fuzz/run.mjs (コメント)・tests/e2e/README.md・tests/conformance/README.md の CI の説明を直した。ADR (0025・0027・0032) と roadmap の M11 の
  経緯の節は、当時の記録として残した (ADR 0032 の「週に 1 度」は release-checks.yml の頻度に変わった)。

  **起票の候補:**
  - rust-cache の環境のハッシュに、ランナーに入っている stable の rustc の版が入るため、ランナーのイメージが stable を上げるとすべての
    キャッシュが外れる (10-11 の run で rust (apps) が約 6 分 → 約 26 分)。`rustup default` を固定の版にする・`add-rust-environment-hash-key`
    を外して鍵に rust-toolchain.toml の版を自分で入れる、などで避けられるか調べる。
  - push が月に多いと、軽い CI でも無料枠に近い。main への push をまとめる (オーケストレーターの運用)、または PR を通したものは main への
    push で再び流さない、などの運用を決める。
  - editor-build.yml の Windows (editor・studio) と macOS (editor) のビルドが 38098550160 で失敗している (リリースの前の検査では見ない部分)。
  - release-checks.yml の結果を 1 つにまとめるジョブ (ブランチの保護の必須の検査にするなら)。

