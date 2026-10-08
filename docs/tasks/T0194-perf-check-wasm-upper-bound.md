---
id: T0194
title: "性能: 時間の上限の検査 (just perf・perf.yml) に wasm (ブラウザ) の計測を入れる (判断が要る)"
status: in-progress
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

- [ ] 判断 (案と上限の値) をメモに書いた
- [ ] (A・B) `measure.mjs` が上限を受け取り、どのモデル・どの行が上限を超えたかを出して 1 で終わる。上限を下げて失敗することを手で確かめた
- [ ] (A・B) `just perf` から wasm の計測も流れる (Chrome・wasm のビルドが無いときに何が起きるかを決めて書いた。例: 分かる文で失敗する)
- [ ] (A) perf.yml に wasm の計測の段を足した (`timeout-minutes` に収まる見込みを書いた)
- [ ] docs/testing.md#遅くなったら気づく検査 と #性能の計測 の表を書き直した
- [ ] `just ci` が通る

## 参考

- T0191・T0192 のメモ、docs/testing.md#遅くなったら気づく検査・#1-フレームの評価
- bindings/web/examples/timing/measure.mjs、justfile の `perf`、.github/workflows/perf.yml

## メモ

- T0078 で起票した (元の候補: T0192 (時間の上限の検査 (`just perf`・perf.yml) に wasm を入れるか決める))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- CI のランナーで揺れて落ちないか・ビルドの時間が収まるかは、perf.yml を手で流して確かめる (作業者の環境では確かめられない。メモに残す)。
- 判断 (2026-10-08): 利用者が案 A (just perf と perf.yml の両方に wasm の上限の検査を入れる。上限は 300µs) を選んだ。
