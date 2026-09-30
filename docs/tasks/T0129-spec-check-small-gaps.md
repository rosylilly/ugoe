---
id: T0129
title: 公開仕様: 独立の読み込み器の型検査を CI に入れる・差し替えたパーツを足す順を決める
status: todo
milestone: M5
crates: [spec, tools/spec-check, docs, justfile, package.json]
depends_on: [T0120]
assignee:
---

# T0129: 公開仕様: 独立の読み込み器の小さな残り

## 目的

T0120 の後続の候補 (小さなもの 2 つ)。

## 受け入れ条件

- [ ] `tools/spec-check` の TypeScript の型検査 (tsc) を `just ci` で走らせる (今は手で 1 回通しただけ)。依存を増やすなら理由をメモに書く
- [ ] 複数のスロットがパーツを差し替えるとき、隠した `#replaced/...` パーツを足す順を `spec/model.md` の Composition に書く (実行時の結果は変わらないが、合成の結果を比べるときに順が要る)。参照実装の順は team-lead に確かめてもらう。`docs/spec/` の同期は team-lead に頼む
- [ ] 最初のセグメントの規則を破るモーションのカーブを、spec/model.md は「無視してよい (MAY)」、spec/motion.md は「値を持たない」としていて強さが食い違う。参照実装の振る舞いを team-lead に確かめてもらい、揃える (T0126 の (7))
- [ ] 表情をすべて止めるホストの操作が仕様にない。参照実装の API を team-lead に確かめてもらい、spec/motion.md に書く (T0126 の (8))
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
