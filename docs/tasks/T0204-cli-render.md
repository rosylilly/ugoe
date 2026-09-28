---
id: T0204
title: CLI でのスナップショット描画
status: done
milestone: M1
crates: [ugoe-cli]
depends_on: [T0201]
assignee: claude-worker-T0204
---

# T0204: CLI でのスナップショット描画

## 目的

`ugoe render <model> --param AngleX=10 -o out.png` で、ヘッドレスの wgpu で描画して PNG を書き出す。AI エージェントが見た目を確認する手段にもなる。

## 受け入れ条件

- [x] フィクスチャのモデルを PNG に書き出せる
- [x] パラメータを複数指定できる
- [x] 出力の解像度と背景色を指定できる
- [x] `just ci` が通る

## メモ

- `ugoe render <INPUT> -o out.png` を追加した。入力はモデル JSON・`.ugoe` バンドル (先頭のマジックで判別)・`fixture:<name>`。
  オプションは `--param ID=VALUE` (複数可、値域外はクランプして警告)、`--size WxH` / `--width` / `--height` / `--scale`、
  `--fit contain|stretch`、`--background` (既定は透明)、`--texture INDEX=PATH`、`--backend`。使い方は `crates/ugoe-cli/README.md`。
- 引数・パラメータ・テクスチャの誤りは GPU を使う前に報告する。
- 描画先は `Rgba8Unorm`・乗算済み。読み戻しは行を `COPY_BYTES_PER_ROW_ALIGNMENT` に揃え、PNG に書くときストレートアルファへ戻す。
- WSL2 の dzn ドライバは wgpu のインスタンス・デバイスの破棄で SIGSEGV を起こすので、CLI は破棄せずに終了する (`Gpu::leak`)。
- `ugoe fixture` は `ugoe_model::fixtures` の全フィクスチャを受け付け (kebab-case も可)、`ugoe fixture list` で一覧を出す。
  `validate` / `info` も `fixture:<name>` とバンドルを受け付け、`info --json` を追加した。
- 依存: `png` 0.18 (tauri 経由で既に Cargo.lock にあったもの)。
- GPU テストは `cargo test -p ugoe-cli --features gpu-tests` (バイナリを別プロセスで実行して PNG の画素を確かめる)。
  `just test-gpu` は ugoe-render だけなので、こちらも含めるよう justfile を更新する必要がある (T0203 の範囲)。
- 残した課題:
  - KTX2 テクスチャのデコードは未対応 (エラーにして `--texture` で PNG を渡すよう案内する)。
  - 物理・モーション・ポーズのフェードは適用しない (指定値で 1 回 `update` した状態を描く)。モーションの時刻指定などは後続タスクの候補。
  - JSON モデルのプレースホルダは白一色なので、マスクのフィクスチャは `--texture` で色付きの画像を渡さないと形が見分けにくい。
  - 手元で `fixture:nested_masks` を描くと `Pupil` が `Iris` の範囲 (y = 64 から) まで見え、入れ子の継承が反映されていない。
    ugoe-render のマスク処理の問題で、T0205 で直る見込み。
