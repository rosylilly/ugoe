---
id: T0800
title: スタジオ: ADR と設計書 (範囲・ウィンドウ構成・入力・外部 API の安全性・ライセンス)
status: done
milestone: M8
crates: [docs, AGENTS.md]
depends_on: []
assignee: claude-worker-T0800
---

# T0800: スタジオ: ADR と設計書 (範囲・ウィンドウ構成・入力・外部 API の安全性・ライセンス)

## 目的

配信で使う新しいデスクトップアプリ「スタジオ」(apps/studio) の範囲と構成を決める。実装の前に、ビューアとの違いと、外から操作する API の安全性を決めておく。

## 受け入れ条件

- [x] ADR 0028 (スタジオ): 目的、ビューアとの役割分担 (ビューアは確認用で軽いまま、配信の機能はスタジオに入れる)、ライセンス (GPL-3.0-only)、
      ウィンドウの構成 (操作ウィンドウと、モデルだけを描く出力ウィンドウ)、入力 (カメラ・マイク・ホットキー)、シーンとプロファイル、
      外部 API (ループバックだけ・トークン・利用者の許可・既定は切。他製品の API の名前は使わない)、仮想カメラと OBS のブラウザソースの出し方、
      M8 でやらないこと (複数人のコラボ・配信サイトのイベントとの連携・プラグインなど)
- [x] 描画の場所を決める: 出力ウィンドウ (WebView の wasm)・仮想カメラ・ブラウザソースに、どこで描いた画を渡すか。
      候補は (a) バックエンドで ugoe-render (wgpu) を使ってオフスクリーンに描く (ugoe-headless を使う)、(b) WebView の画を読み戻して渡す (1080p60 では IPC が重い)、
      (c) 評価後のパラメータの値だけを送り、受け手 (ブラウザソースの wasm など) が描く。物理と時間の扱い (受け手で物理を回すか) も決める
- [x] docs/studio.md: 画面と操作の設計 (操作ウィンドウのパネル構成、出力ウィンドウの設定、シーンの切り替え、ホットキー、API の一覧の骨子)
- [x] docs/architecture.md と AGENTS.md のリポジトリ構成に apps/studio を書く
- [x] `just ci` が通る

## 参考

- docs/vision.md、docs/adr/0019-viewer-license.md、apps/viewer/README.md

## メモ

- ADR 0028 (`docs/adr/0028-studio.md`、状態は Proposed) と `docs/studio.md` を書いた。`docs/architecture.md` (層の表と主要な判断)・
  `AGENTS.md` (リポジトリ構成と「最初に読むもの」)・`docs/README.md`・`docs/adr/README.md` に足した。
- 描画の場所: バックエンドの指揮役 (`ugoe::Puppet`、物理は切る) が刻みごとに `{seq, dt, values}` を配り、受け手 (出力ウィンドウの wasm・
  ブラウザソースの wasm・仮想カメラのネイティブ + `ugoe-render`) が全フレームを順に適用して物理を回す。根拠は動的物理 (ADR 0011) の出力が
  パラメータではないこと、`ugoe-headless` の今の口が 1 枚用であること、(b) が 1080p60 で約 500 MB/s になること。
- 利用者の決定 (2026-10-02) を反映して ADR 0028 を Accepted にした: Windows の仮想カメラは Media Foundation (管理者権限で登録、OBS 経由の手順も書く)、
  macOS は実装と手順を M8 で作り配布はアカウントの用意の後、Linux は v4l2loopback を案内するだけ、ホットキーは `tauri-plugin-global-shortcut`
  (Wayland は外部 API で代わり)、M8 でやらないことに 4 項目を足す。ADR に `unsafe` の例外の範囲 (8.) と一致の検査 (10.) を書いた。
- crates: に AGENTS.md を足した (受け入れ条件のリポジトリ構成に加え、team-lead の承認で `unsafe` の例外の行を「`ugoe-ffi` と ADR 0028 の仮想カメラの 1 クレート」に直した)。
- 後続のタスクとの食い違い (タスクファイルは直していない):
  - T0804: 「フレームレートの上限 (30 / 60 / 無制限)」は、この設計では指揮役の刻みの速さ (30 / 60 / 120) になる。受け手は刻みより速く描いても新しい形が出ない。
    また、入力を取る操作ウィンドウの WebView も抑制を切る必要がある。
  - T0801: 「状態の受け渡し (操作 → 出力)」は、操作 → バックエンドの指揮役 → 出力の形になる。指揮役とフレームの配信の形を T0801 で作る想定。
    `@ugoe/player` (T0904) に受け手のモード (段を切ってフレームを順に適用する) が要る。T0801 で AGENTS.md のライセンスの行にも apps/studio を足す。
  - T0807: 仮想カメラの常駐の描画の口を `ugoe-headless` に足すので、crates に `crates/ugoe-headless` が要る。Windows は `unsafe` を許すクレートを分ける (未決)。
  - 受け手が書いた値を確実に基準値にするための `Puppet` の口が要るかもしれない (ADR の「結果」)。要るならランタイムの公開 API の変更なので別のタスク。
  - T0808: 受け手のページが毎フレーム多数の `set_parameter` を呼ぶ。まとめて書く口 (`WebPuppet` に `Float32Array` で渡す) があると軽いが、`crates/ugoe-wasm` は範囲外。
