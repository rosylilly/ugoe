---
id: T0191
title: "性能: Puppet の評価が遅くなったら気づける検査を足す (判断が要る)"
status: done
milestone: M7
crates: [ugoe, scripts, justfile, .github/workflows, docs]
depends_on: [T0190]
assignee: claude-worker-T0191
---

# T0191: 性能: 評価が遅くなったら気づける検査 (判断が要る)

## 目的

T0190 の計測の例 (`crates/ugoe/examples/puppet_timing.rs`) は手で動かして数を見るだけで、変更で遅くなっても誰も気づかない。
M7 の性能の目標「1 体を 1ms 以内」(docs/roadmap.md) を守るために、遅くなったら失敗する検査を足す。

時間は機械と負荷で大きく揺れる (共有の CI のランナーでは 2〜3 割) ので、`just ci` (PR ごと) に時間の比べ合いを入れると、
関係の無い PR が落ちる。どこで何を比べるかを決める必要がある。

**判断が要る** (CI の方針と、案によっては新しい開発時の依存・CI に入れる道具)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. 時間の上限を、定期の CI のジョブと `just perf` で確かめる (依存を足さない):** `puppet_timing` に上限の判定 (例: `--check` で、
  生成のモデルの `update` の中央値が上限を超えたら終了コード 1) を足し、`just perf` で流す。CI は定期 (週に 1 度) と手動の
  ジョブ (`.github/workflows/perf.yml`。fuzz.yml・nostd-build-std.yml と同じ扱い) で流す。上限は目標の 1ms (ランナーが遅ければ
  余裕を持たせた値) にする。大きな後退 (2 倍など) と、目標を割ったことに気づける。細かい後退 (1〜2 割) は揺れに埋もれて分からない。
- **B. 前の値と比べるベンチマーク (`criterion` か `divan` を開発時の依存に):** 前の実行の値と比べて有意に遅くなったら知らせる。
  手元で前後を比べるのに向く。CI のランナーでは前の値が別の機械のものになり、揺れで誤報が多い。新しい依存 (ライセンスは
  MIT / Apache-2.0)。
- **C. 命令の数を数える (`iai-callgrind` と valgrind):** 時間ではなく実行した命令の数を比べるので、共有のランナーでも揺れがほぼ無く、
  PR ごとに 1〜2% の後退まで気づける。新しい開発時の依存と、CI に valgrind が要る (Linux だけ)。キャッシュやメモリの待ちは数に出ない。
- **D. 手元だけ:** `just perf` で上限を確かめるだけで、CI には入れない。忘れると気づかない。

### 推奨

A。依存を増やさずに「目標を割った」「大きく遅くなった」に気づけ、M7 の目標そのものを確かめる形になる。PR ごとに細かい後退を
止めたくなったら、そのときに C を足す (別のタスク)。上限の値と、ランナーで測った値はメモと docs/testing.md に書く。

## 受け入れ条件

- [x] 利用者に案を示して方針を決め、決めたことと理由を docs/testing.md の「性能の計測」に書いた (新しい依存や道具を入れるなら、
      その理由も。ランタイム層に重い依存を入れるなら ADR)
- [x] 決めた方針の検査を足した。上限を超える (か前より遅い) ときに失敗することを、一時的に遅くした変更 (コミットしない) で確かめ、
      メモに書いた
- [x] CI で流すなら、ジョブを足し、push した後の CI で確かめることをメモに書いた。`just` のレシピを足したら AGENTS.md のコマンドの表にも足した
- [x] `just ci` が通る

## 参考

- T0190 (計測の例)、docs/testing.md#性能の計測、docs/roadmap.md の M7
- .github/workflows/fuzz.yml・nostd-build-std.yml (定期のジョブの例)、scripts/ (CI の補助)

## メモ

- T0075 で起票した (元: docs/roadmap.md の M7 の「性能: 1 体を 1ms 以内」の、遅くなったら気づける検査)。
- 判断 (2026-10-08): 利用者が案 A (just perf と週 1 回の定期の CI のジョブで時間の上限を確かめる。依存を足さない) を選んだ。
- やったこと:
  - `crates/ugoe/examples/puppet_timing.rs` に `--check` を足した。`update` と `follow` の中央値を `LIMITS` (生成のモデル 300µs、
    `avatar.ugoe` 100µs) と比べ、超えたものは 2 回まで測り直して最も速い中央値で判定し、超えたままなら `OVER` と出して終了コード 1。
  - `just perf` (release で `--check`)、`.github/workflows/perf.yml` (毎週月曜 05:11 UTC と手動。`lscpu` を出してから `just perf`。
    rust-cache は release 用に `shared-key: perf`)。AGENTS.md のコマンドの表と docs/testing.md (検査の表・CI のジョブ・
    「性能の計測」の「遅くなったら気づく検査」に方針・上限・決め方・揺れへの備え) を書いた。
  - 上限の決め方: 手元 (Ryzen 9 7900) の 55µs / 18µs に、ランナーの 1 コアの遅さ (2 倍ほど) と揺れ (2〜3 割) を見込んだ値
    (150µs 弱 / 50µs 弱) のさらに 2 倍ほど。生成のモデルの 300µs は、中程度の PC がランナーの 3 倍遅くても 1ms に収まる値でもある。
  - 失敗の確かめ (コミットしていない): `Puppet::update` の頭で `self.instance.update()` を 6 回余分に呼ぶ変更で `just perf` を流し、
    生成のモデルの `update` (373.0µs / 上限 300) と `avatar.ugoe` の `update` (126.1µs / 上限 100) が `OVER`、`follow` は ok、
    `2 measurement(s) over the limit` で終了コード 1 になった。戻した後は ok で終了コード 0 (手元はほかのビルドで load average 16 ほど、
    生成のモデルの `update` の中央値 81.9µs)。
  - `actionlint` (1.7.12、shellcheck は外して) で perf.yml にエラーが無いことを確かめた。
- push した後の CI で確かめること (push していないので未確認):
  - perf.yml を手動 (`workflow_dispatch`) で 1 度流し、成功すること。`lscpu` の CPU と、ランナーで測った値 (生成のモデル・`avatar.ugoe` の
    `update`・`follow` の中央値) を docs/testing.md の「遅くなったら気づく検査」に書き、上限が緩すぎ (値の 5 倍以上など) ・
    きつすぎ (値の 1.5 倍未満など) なら見直す。
  - 何度か流して、揺れで落ちないこと。release のビルドの時間と rust-cache が効くこと (timeout-minutes: 30 に収まること)。
- やらなかったこと: 前の値との比較 (案 B) と命令の数 (案 C)。wasm (ブラウザ) での時間の検査。
- 起票の候補:
  - perf.yml を何度か流した後の、ランナーの値の記録と上限の見直し。
  - PR ごとに細かい後退を止めたくなったら、命令の数を数える検査 (案 C、iai-callgrind と valgrind)。
