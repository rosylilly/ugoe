---
id: T0951
title: "@ugoe/face-tracking: TrackingPanel から高さの上限と送りの指定を外し、置く側のアプリに任せる"
status: done
milestone: M10
crates: [packages/face-tracking, apps/viewer, apps/studio, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0951
---

# T0951: @ugoe/face-tracking: TrackingPanel から高さの上限と送りの指定を外し、置く側のアプリに任せる

## 目的

`@ugoe/face-tracking` の `TrackingPanel` は自身に `max-height: 45%`・`overflow-y: auto` を持つ。T0725 でビューアは右の列を 2 段にし、パネルの高さの上限を
外から外した (`.features > *`)。配置は置く側 (ビューア・スタジオ・エディタ) が決めるものなので、部品から外す (T0725 の候補)。

## 受け入れ条件

- [x] `TrackingPanel` が高さの上限と送りを持たない。ビューアの `.features > *` の打ち消しを外した
- [x] スタジオとエディタでトラッキングのパネルを置く場所が、必要なら自分で高さと送りを決め、`just ui-audit` (3 つのアプリ) の指摘が増えない。前と後の画像を比べてメモに書いた
- [x] packages/face-tracking/README.md (置き方) と docs/ui.md を直した
- [x] `just ci` が通る

## 参考

- T0725 のメモ (やらなかったこと・起票の候補)、docs/ui.md#ビューアの見直し-t0725
- packages/face-tracking の `TrackingPanel`

## メモ

- T0040 で起票した (元の候補: T0725 (`TrackingPanel`))。パネルの中の詰め (「体 (肩)」の折り返し・小さなボタンの 2 行) も、3 つのアプリで画像を見て直せるならこのタスクで直す。

### やったこと (T0951)

- `TrackingPanel.svelte` の `.tracking` の規則 (`flex-shrink: 0`・`max-height: 45%`・`overflow-y: auto`) を消した。パネルは中身の高さで伸びる。
- ビューア (`App.svelte`) の `.features > *` から `max-height: none`・`overflow: visible` の打ち消しを外した (`flex: none` は残す。
  パネルを縮めずに上の段がまとめて送るため)。コメントも直した。
- スタジオ (右の列 `.studio__panels` が列ごと送る) とエディタ (プレビューの列が列ごと送る) は、パネルを中身の高さの縦の並びに置いている。
  親の高さが決まっていないので前から `max-height: 45%` は効いておらず、置く側で高さや送りを足す必要は無かった (画像でも変わらない)。
- エディタの `PreviewPanel.svelte` に残っていた `details.tracking` 向けの規則 (パネルが `<details>` だったころのもの。今の `Section` には
  当たらない) を消した。
- packages/face-tracking/README.md の「使い方」に置き方 (高さの上限と送りは置く側が決める) を足し、docs/ui.md の部品の表と
  「ビューアの見直し (T0725)」を直した。

### 前と後 (`just ui-audit`、3 つのアプリ)

- 前 `target/ui-audit-before`・後 `target/ui-audit-after --compare target/ui-audit-before`: editor +0 -0 ~0、viewer +0 -0 ~0、studio +0 -0 ~0。
  件数は前も後も editor 0・viewer 0・studio のコントラスト 2 (3) (main の時点から在るもので、この変更とは無関係)。
- 画像を `compare -fuzz 2%` で比べた。違いはモデルの描画 (呼吸・物理の時刻の差) と、パラメータの値の欄、エディタの着せ替えのエラーの文の中の
  出力先のパス (`ui-audit-before`・`ui-audit-after`) だけで、トラッキングのパネルとその前後の部品の位置は変わらない (ビューアの広い幅・狭い幅、
  エディタのプレビュー、スタジオ)。
- `just e2e` (3 つのアプリ) が通る。

### やらなかったこと

- パネルの中の詰め (「体 (肩) も」の折り返し・小さなボタンの 2 行): ビューアの広い幅・360px、エディタのプレビューの画像で、「体 (肩) も」は
  折り返しておらず、小さなボタンの 2 行はキャリブレーション (キャリブレーション・解除) と記録 (記録・保存) の 2 つの組を分けた並びで
  意図どおりと判断し、直さなかった。
- スタジオの入力のパネルは `just ui-audit` の場面で閉じているか送った先の外にあり、画像では中を見られなかった (検査の件数は変わらない)。

### 起票の候補

- `just ui-audit` のスタジオに、「入力」のパネル (トラッキングのパネル) を開いて送った場面が無い。足すと置き方の変更を画像で見られる。
