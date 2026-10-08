---
id: T0869
title: "スタジオ: 外部 API に、今のプロファイルが書き換えないプロファイル (見本・読み取り専用) かを出すか決める (判断が要る)"
status: todo
milestone: M8
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0869: スタジオ: 外部 API に、今のプロファイルが書き換えないプロファイルかを出すか決める (判断が要る)

## 目的

T0866 で、見本の印 (`template`) のあるプロファイルや書けない場所のプロファイルは、自動の保存で書き換えない「書き換えないプロファイル」になった
(`Stage::read_only`・`ReadOnly::Template`・`NotWritable`)。操作ウィンドウは保存の印 (「見本」・「読み取り専用」) と「写しを保存…」の知らせを出すが、
外部 API (docs/studio-api.md) には出していない。外から操作するクライアント (配信のデッキなど) が、変えた内容が保存されないことを知る口を足すか決める。

## 判断が要る

今の外部 API (version 1) にはプロファイルそのものの様子が無い (`studio.state` は放送中のシーン・モデル・着せ替え・表情・モーション・トラッキングだけ)。
API から保存はできず、API で変えられるもの (シーンの切り替え・表情・パラメータなど) の多くはプロファイルに残らない。

- **案 A: 出す。** `studio.state` の結果に `profile` (`{readOnly}`。`readOnly` は `"template"`・`"notWritable"`・`null`) を足し、変わったら
  イベント `profile.changed` を送る。`apiVersion` は 1 のまま (結果の欄とイベントを足すだけ。docs/studio-api.md#versioning)。
  プロファイルの名前やパスは出さない (パスを外に出さない今の方針に合わせる。名前を出すかは別に決める)。
- **案 B (推奨): 出さない。** API はプロファイルを保存できず、書き換えないプロファイルで困るのは操作ウィンドウで編集する人だけなので、
  外のクライアントが知る利点が小さい。docs/studio-api.md に「プロファイルの保存は扱わない」と 1 行書き、ロードマップの「起票していない後続の候補」に残す。
- 案 C: プロファイルの様子をまとめて出す (名前・未保存の変更があるか・書き換えないか)。A より広く、要望が出てから決めたほうがよい。

推奨は B。配信のデッキから「保存されていません」を出したい要望が出たら A にする。

## 受け入れ条件

- [ ] 判断をメモに書いた
- [ ] (A) `studio.state` に `profile.readOnly` を足し、変わったら `profile.changed` を送る。docs/studio-api.md・docs/studio-api.schema.json を直し、
      バックエンドのテスト (見本の印のあるプロファイルを開くと `"template"`、写しを保存すると `null`、書けない場所で保存に失敗すると `"notWritable"` とイベント) を足した
- [ ] (B) docs/studio-api.md に、API がプロファイルの保存と書き換えないプロファイルを扱わないことを書き、docs/roadmap.md の M8 の「起票していない後続の候補」に移した
- [ ] `just ci` が通る

## 参考

- T0866 のメモ (起票の候補)、docs/studio.md#書き換えないプロファイル、docs/studio-api.md (`studio.state`・Events・Versioning)
- apps/studio/src-tauri の `stage.rs` (`read_only`・`save_profile`)、外部 API の実装 (`studio.state` を作る所)

## メモ

- T0080 で起票した (元の候補: T0866 (書き換えないプロファイルの訳を外部 API のプロファイルの様子にも載せるか))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
