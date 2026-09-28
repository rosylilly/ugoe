---
id: T0620
title: 実用の PSD への対応 (仕様どおりのテスト用 PSD と実ファイルの検証)
status: in-progress
milestone: M2
crates: [ugoe-psd, assets/fixtures, ugoe-editor-core (テストのみ)]
depends_on: []
assignee: claude-worker-T0620
---

# T0620: 実用の PSD への対応 (仕様どおりのテスト用 PSD と実ファイルの検証)

## 目的

T0619 の実機確認で、テスト用の `assets/fixtures/layers.psd` が Photoshop で開けないこと、表示が虹色の模様で確認用に適さないことが分かった。
ugoe の PSD の読み込みは、自作の書き出し処理と ImageMagick の出力でしか検証されておらず、実際のペイントツールが書き出した PSD で動くかが分かっていない。

## 受け入れ条件

- [ ] 今の `layers.psd` が Photoshop で開けない原因を、PSD の公開仕様 (Adobe の File Formats Specification) と照らして特定し、メモに書く
- [ ] テスト用 PSD の書き出し処理が、仕様どおりの (Photoshop などで開ける) PSD を書く。偽の中身は書かない
- [ ] 取り込みのエラー処理を試すための壊れた入力・未対応の中身は、ファイルにせずテストの中で作る (仕様どおりの PSD と分ける)
- [ ] 確認用の PSD を用意する: 意味の分かる簡単なキャラクターの絵 (単色の図形でよい: 顔・両目・口・髪・体など) を、グループ・クリッピング・非表示のレイヤー・不透明度を含めて描いたもの。`just editor` で取り込んで見た目で確かめられること。合成画像も正しく作る
- [ ] ImageMagick で読み、合成画像とレイヤーが期待どおりであることを確かめる
- [ ] 実際のペイントツールの PSD を置けば自動でテストされる場所 (例: `assets/fixtures/psd-real/`) と、その追加手順・ライセンスの記録方法を用意する (ファイルが無ければテストは何もしない)
- [ ] PSD の仕様のうち、実際のツールがよく使うのに未対応のもの (16 ビット、ZIP 圧縮、レイヤーマスク、グレースケールなど) を洗い出し、優先度を付けて後続タスクの候補にする
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0619-editor-manual-check.md` のメモ
- `docs/tasks/T0501-psd-import.md`
- `crates/ugoe-psd/README.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
