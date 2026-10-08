---
id: T0194
title: "性能: 時間の上限の検査 (just perf・perf.yml) に wasm (ブラウザ) の計測を入れる (判断が要る)"
status: done
milestone: M7
crates: [bindings/web, scripts, justfile, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0194
---

# T0194: 性能: 時間の上限の検査に wasm (ブラウザ) の計測を入れる (判断が要る)

## 目的

T0191 の `just perf` と perf.yml (週 1 回と手動) はネイティブの `Puppet` の 1 フレームの評価だけを上限と比べる。T0192 で wasm (ブラウザ) の
計測のページ (`bindings/web/examples/timing/`) を足したが、上限の判定は無い。wasm で遅くなったときにも気づけるように、検査に入れるかを決めて入れる。

手元の中央値は回ごとに 3 割ほど揺れた (生成のモデルの `update` で 67〜87µs)。上限はネイティブと同じ 300µs が目安 (揺れに対して 3 倍以上の余裕)。

## 判断が要る

- **案 A (推奨): `just perf` と perf.yml の両方に入れる。** `measure.mjs` に上限 (例: `--max-us 300`) の判定を足し、中央値が超えたら 1 で終わる。
  perf.yml は Chrome のあるランナー (ubuntu-latest には Chrome が入っている。tests/ui-audit と同じ Chrome の探し方) で `just web-runtime` の後に流す。
  ネイティブの検査と同じく PR ごとの CI には入れない。
- 案 B: `just perf` (手元) にだけ入れ、perf.yml には入れない。CI のビルド (wasm-pack) の時間が増えない代わりに、定期の検査では気づけない。
- 案 C: 入れない (計測のページで手で測るだけ)。
- 上限の値: ネイティブと同じ 300µs (推奨) か、wasm だけ別の値か。

## 受け入れ条件

- [x] 判断 (案と上限の値) をメモに書いた
- [x] (A・B) `measure.mjs` が上限を受け取り、どのモデル・どの行が上限を超えたかを出して 1 で終わる。上限を下げて失敗することを手で確かめた
- [x] (A・B) `just perf` から wasm の計測も流れる (Chrome・wasm のビルドが無いときに何が起きるかを決めて書いた。例: 分かる文で失敗する)
- [x] (A) perf.yml に wasm の計測の段を足した (`timeout-minutes` に収まる見込みを書いた)
- [x] docs/testing.md#遅くなったら気づく検査 と #性能の計測 の表を書き直した
- [x] `just ci` が通る

## 参考

- T0191・T0192 のメモ、docs/testing.md#遅くなったら気づく検査・#1-フレームの評価
- bindings/web/examples/timing/measure.mjs、justfile の `perf`、.github/workflows/perf.yml

## メモ

- T0078 で起票した (元の候補: T0192 (時間の上限の検査 (`just perf`・perf.yml) に wasm を入れるか決める))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- CI のランナーで揺れて落ちないか・ビルドの時間が収まるかは、perf.yml を手で流して確かめる (作業者の環境では確かめられない。メモに残す)。
- 判断 (2026-10-08): 利用者が案 A (just perf と perf.yml の両方に wasm の上限の検査を入れる。上限は 300µs) を選んだ。
- やったこと (2026-10-08):
  - `measure.mjs` に `--max-us <µs>` を足した。`update` と `follow` (ネイティブの `LIMITS` と同じ計測。物理を止めた `update` は比べない) の
    中央値 (`--runs` 回の中央値) を上限と比べ、超えたらページを開き直して 2 回まで測り直し (T0191 の `RETRIES` と同じ形。行ごとに最も速い
    中央値で判定)、`check (us): median / limit` の表にモデル・計測・`ok`/`OVER` を出して、超えたものがあれば終了コード 1。判定は
    `bindings/web/examples/timing/check.mjs` に分け、`check.test.mjs` (vitest。`pnpm --filter @ugoe/runtime test` で流れる) で確かめる。
  - justfile: `perf` を `perf-native` (前の `perf` の中身) と `perf-wasm` に分けた。`perf-wasm` は古いビルドを測らないように
    `just web-runtime` と `puppet_timing -- --write` で生成物を毎回作り直し、`measure.mjs --max-us 300` を流す (ページは 1 回。手元で 16 秒ほど)。
  - Chrome・wasm のビルドが無いとき: wasm-bindgen-cli が無ければ `just web-runtime` が入れ方を出して失敗、Chrome が無ければ (CHROME の
    指すファイルが無いときも) `measure.mjs` が `Chrome is not found ...` と出して終了コード 2。飛ばして成功にはしない (docs/testing.md に書いた)。
  - perf.yml: Node・pnpm の依存とビルド済みの wasm-bindgen-cli を入れ、`just perf-native` と `just perf-wasm` を別の段にした
    (wasm の段は `if: success() || failure()` でネイティブが落ちても流す)。`timeout-minutes: 30` は据え置き (見込みはキャッシュ無しで
    release と web-dist のビルドがそれぞれ 10 分ほど、計測は合わせて 2 分ほど)。
  - docs/testing.md の表 (検査の一覧・性能の計測・遅くなったら気づく検査の上限の表とレシピの表) と、bindings/web/README.md・AGENTS.md の
    `just perf` の説明を書き直した (AGENTS.md は `crates:` の外だが 1 行の説明の更新だけ)。
  - 確かめたこと (手元、ほかのワーカーのビルドが並行して load average 9〜15): `just perf-wasm` は生成のモデルの `update` 70µs 前後で通る。
    `--max-us 50` で生成のモデルの 2 行が `OVER` になり、2 回測り直してから終了コード 1。わざと遅くする一時的な変更
    (`WebPuppet::update` で `Puppet::update` を 6 回呼ぶ。コミットしていない) で `just perf-wasm` が生成のモデルの `update` 335µs の
    `OVER` で失敗した。`--max-us 0`・CHROME が無いファイル・generated.ugoe が無いときに終了コード 2 になることも確かめた。
- 確かめていないこと (CI で確かめる): perf.yml を手で流し (push してから workflow_dispatch)、(1) ubuntu-latest の google-chrome で
  `measure.mjs` が開けること、(2) キャッシュ無しのビルドを含めて 30 分に収まること、(3) ランナーで測った wasm の値 (揺れて落ちないか)。
  値が分かったら docs/testing.md#遅くなったら気づく検査 に書く。
- やらなかったこと: wasm の `avatar.ugoe` に別の (きつい) 上限は置かなかった (利用者の決定は 300µs の 1 つの値)。
- 起票の候補: 無し (CI のランナーで測った値を書き、上限を見直すのは perf.yml を流した後の作業。T0191 の同じ候補とまとめてよい)。
