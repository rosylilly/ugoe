---
id: T0807
title: スタジオ: 仮想カメラ (Windows と Linux)
status: in-progress
milestone: M8
crates: [apps/studio, ugoe-headless, ugoe-render, scripts, commitlint.config.mjs, docs]
depends_on: [T0804]
assignee: claude-worker-T0807
---

# T0807: スタジオ: 仮想カメラ (Windows と Linux)

## 目的

会議アプリや配信ソフトのカメラの入力として、スタジオの出力をそのまま選べるようにする。macOS は署名と System Extension が要るので T0809 に分ける。

## 受け入れ条件

- [ ] ADR 0028 で決めた描画の場所から、一定のフレームレートで RGBA の画を仮想カメラへ渡す (背景は不透明。透過は単色で埋める)
- [ ] バックエンドに ugoe-render を常駐させて受け手として描き、読み戻す (ADR 0028。ugoe-headless に常駐の描画の口を足す)
- [ ] Windows: Windows 11 の仮想カメラの API (Media Foundation) で出す。COM の unsafe は ADR 0028 で決めた専用の 1 クレートに閉じ込める。登録はインストール時に管理者権限。使えない版では理由を出す
- [ ] Linux: v4l2loopback の機器に書く。機器が無いときは入れ方を案内する
- [ ] 開始・停止・解像度・フレームレートの設定を操作ウィンドウに置く。止まった・遅れたことを知らせる
- [ ] unsafe を許す仮想カメラ専用のクレートを作る (ADR 0028 の 8.。場所は apps/studio の下)。scripts/check-layers.mjs・ci-groups.mjs・check-licenses.mjs と commitlint の scope に登録する
- [ ] 作業の環境 (WSL) で確かめられない部分は、手順を docs に書いて利用者の確認に残す
- [ ] 依存を足すならライセンスを確かめる (GPL-3.0 と両立)。`just ci` が通る

## 参考

- ADR 0028、docs/studio.md

## メモ
