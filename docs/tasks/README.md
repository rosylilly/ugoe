# タスクボード

1 ファイル 1 タスクです。手順は [../workflow/multi-agent.md](../workflow/multi-agent.md) にあります。

```sh
just tasks          # 状態別の一覧と、着手できる (ready) タスク
just tasks --ready  # ready のタスクだけ
```

## ファイル名

`T####-kebab-slug.md`。番号の上 2 桁 (2 つ目の範囲では T1 の次の 1 桁) が分野を表します。

| 番号帯 | 2 つ目の範囲 | 分野 |
| --- | --- | --- |
| T00xx | T10xx | 基盤・リポジトリ |
| T01xx | T11xx | モデル・ランタイム・形式 |
| T02xx | T12xx | レンダラ |
| T03xx | T13xx | 物理・モーション |
| T04xx | T14xx | バインディング (FFI・wasm) |
| T05xx | T15xx | 取り込み・メッシュ・AI |
| T06xx | T16xx | エディタ |
| T07xx | T17xx | ビューア |
| T08xx | T18xx | スタジオ |
| T09xx | T19xx | 共有 UI・i18n |

帯の 100 個を使い切ったら、2 つ目の範囲「T1 + 分野の 1 桁 + 2 桁」を使います。
エディタの T06xx は T0699 まで使い切ったので、新しいエディタのタスクは T1600 から付けます。ほかの帯は空いている番号から使います。

## フロントマター

```yaml
---
id: T0104
title: デフォーマ・パーツ階層・グルーの評価
status: todo            # todo | in-progress | review | done | blocked
milestone: M1            # M0〜M11、または番号の無い Release (公開。時期は未定。docs/roadmap.md)
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
