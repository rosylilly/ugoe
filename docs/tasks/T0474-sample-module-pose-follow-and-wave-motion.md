---
id: T0474
title: "見本: 着せ替えのスロットのポーズの公開と追従、ポーズを要求するモーション"
status: todo
milestone: M9
crates: [ugoe-cli, assets/samples, docs]
depends_on: []
assignee:
---

# T0474: 見本: 着せ替えのスロットのポーズの公開と追従、ポーズを要求するモーション

## 目的

T0468 で見本のアバターと着せ替えの本体にパラメータのグループとポーズを持たせた。次が残っている (T0468 の候補)。

1. 着せ替えの見本のスロット `Outfit` の `pose_parts` に `Wave` を公開し、モジュール `Sailor` の袖を `links` で追従させる
   (モジュールのポーズの追従の見本。今は見本に例が無い)。
2. 見本にポーズを切り替えるモーション (例: `wave`。パーツの不透明度のカーブ) を足す。今は `ugoe render` で試すのにモーションの JSON を手で書く。

## 受け入れ条件

- [ ] `dress-up` の見本の `Outfit` が `Wave` を公開し、`Sailor` を付けて `Wave` を表示すると袖が追従する。`crates/ugoe-cli/src/sample/dress_up.rs` のテストで確かめた
- [ ] 見本に `wave` のモーションがあり、再生するとポーズが切り替わる。テストがある
- [ ] `assets/samples` を作り直し (`committed_sample_is_up_to_date` が通る)、assets/samples/README.md と docs/manual の該当箇所に試し方を書いた
- [ ] 既定の見た目 (`ugoe render` の PNG) が変わらないか、変わるなら `*.jpg` を作り直した
- [ ] `just conformance --only queries` と `just ci` が通る

## 参考

- T0468 のメモ (判断と理由・起票の候補)
- docs/spec/modules.md (スロットのポーズ)、assets/samples/README.md

## メモ

- T0036 で起票した (元の候補: T0468 (`Outfit` の `pose_parts`・ポーズを要求するモーション))。
