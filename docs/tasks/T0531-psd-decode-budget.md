---
id: T0531
title: ugoe-psd の展開後の大きさに上限を設ける (ZIP の展開爆発)
status: todo
milestone: M9
crates: [ugoe-psd, docs]
depends_on: [T0525]
assignee:
---

# T0531: ugoe-psd の展開後の大きさに上限を設ける (ZIP の展開爆発)

## 目的

T0525 のレビューで見つけた所見。ZIP (圧縮方式 2・3) のチャンネルは、レイヤーの矩形の大きさ (1 辺 60000 まで、
16 ビットなら 1 チャンネル 7.2 GB) までそのまま展開する。deflate は 1000 倍ほどに縮むので、数十 MB の PSD で
数十〜数百 GB を確保させられる。しかも `parse` はすべてのレイヤーのチャンネルを展開してから木を作るので、
レイヤーの数だけ積み上がる。エディタ・MCP サーバが利用者の選んだ (あるいは外から渡された) PSD を開くと、
メモリを使い切って止まる。

無圧縮と PackBits は入力の大きさ (PackBits は 64 倍) で抑えられているので、ZIP だけの問題。

## 受け入れ条件

- [ ] 展開した大きさの合計 (またはレイヤーの画素数) の上限を決め、超えたら `PsdError::Unsupported` か
      `Invalid` にする。上限の値と理由を README に書く (実際の PSD を妨げない値。例: キャンバスの何倍か、固定の値か)
- [ ] 小さな ZIP 爆弾の PSD (テストの中で組み立てる) で、確保する前にエラーになるテスト
- [ ] 必要なら、展開したチャンネルを 1 レイヤーずつ RGBA にして捨てる (ピークのメモリを下げる)
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-psd/src/parse.rs` の `inflate`・`read_channel_images`
- docs/workflow/code-review.md (外から読むデータ: 巨大な入力・展開の爆発)

## メモ
