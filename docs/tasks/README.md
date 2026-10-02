# タスクボード

1 ファイル 1 タスクです。手順は [../workflow/multi-agent.md](../workflow/multi-agent.md) にあります。

```sh
just tasks          # 状態別の一覧と、着手できる (ready) タスク
just tasks --ready  # ready のタスクだけ
```

## ファイル名

`T####-kebab-slug.md`。番号の上 2 桁は分野を表します。

| 番号帯 | 分野 |
| --- | --- |
| T00xx | 基盤・リポジトリ |
| T01xx | モデル・ランタイム・形式 |
| T02xx | レンダラ |
| T03xx | 物理・モーション |
| T04xx | バインディング (FFI・wasm) |
| T05xx | 取り込み・メッシュ・AI |
| T06xx | エディタ |
| T07xx | ビューア |

## フロントマター

```yaml
---
id: T0104
title: デフォーマ・パーツ階層・グルーの評価
status: todo            # todo | in-progress | review | done | blocked
milestone: M1            # M0〜M7、または番号の無い Release (公開。時期は未定。docs/roadmap.md)
crates: [ugoe-runtime]  # このタスクで触ってよい範囲
depends_on: [T0102]     # これらが done になるまで着手しない
assignee:               # 着手したエージェントやセッションの名前
---
```

## 本文の構成

- **目的**: 何のために何をするのか
- **受け入れ条件**: 検証できる形のチェックリスト。最後の項目は必ず「`just ci` が通る」
- **参考**: 関連する仕様書や ADR
- **メモ**: 作業中に分かったこと、残した課題、後続タスクの候補。完了時に必ず書く
