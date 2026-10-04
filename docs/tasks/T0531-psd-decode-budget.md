---
id: T0531
title: ugoe-psd の展開後の大きさに上限を設ける (ZIP の展開爆発)
status: done
milestone: M9
crates: [ugoe-psd, docs]
depends_on: [T0525]
assignee: claude-worker-T0531
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

- [x] 展開した大きさの合計 (またはレイヤーの画素数) の上限を決め、超えたら `PsdError::Unsupported` か
      `Invalid` にする。上限の値と理由を README に書く (実際の PSD を妨げない値。例: キャンバスの何倍か、固定の値か)
- [x] 小さな ZIP 爆弾の PSD (テストの中で組み立てる) で、確保する前にエラーになるテスト
- [x] 必要なら、展開したチャンネルを 1 レイヤーずつ RGBA にして捨てる (ピークのメモリを下げる)
- [x] `just ci` が通る

## 参考

- `crates/ugoe-psd/src/parse.rs` の `inflate`・`read_channel_images`
- docs/workflow/code-review.md (外から読むデータ: 巨大な入力・展開の爆発)

## メモ

### やったこと

- 公開 API に `ImportLimits` (`max_channel_decompressed` 既定 1 GiB・`max_total_decompressed` 既定 4 GiB、`#[non_exhaustive]`、
  `with_*` で既定値から変える) と `import_with(bytes, limits)` を足した。`import` は既定の上限で読む。T0150 の方針どおり、
  上限は形式でなく読む側の設定で、`ugoe-format` の `ReadLimits` と同じ形にした。
- チャンネル (色・アルファ・マスク) と合成画像の平面を展開する前に、矩形から分かる展開後の大きさを上限と比べ、
  超えたら `PsdError::Unsupported` (メッセージに "import limit") にする。圧縮方式によらず数える (合計をメモリの目安にするため)。
  専用のエラーの型は足さなかった (`PsdError` に variant を足すと公開 API の変更が大きくなるため。受け入れ条件どおり `Unsupported`)。
- チャンネルを 1 レイヤーずつ RGBA に並べ直してマスクを掛け、展開した平面をすぐ捨てるようにした
  (`RawLayer::assemble`。RGBA の組み立てを lib.rs から parse.rs に移した)。
- テスト: 矩形だけ大きい (40000 × 40000) ZIP のチャンネルが展開の前に止まること、本物の ZIP 爆弾 (1 MiB の 0 が約 1 KiB の
  レイヤーを 64 枚) が合計の上限で止まること、圧縮方式 0〜3・16 ビット・マスク・合成画像の数え方と境界値。
  テストのライター (`tests/common`) に `Layer::channel_data` を足した。
- README (「展開の上限」の節と理由)、`docs/ai-integration.md` の PSD 取り込みに 1 行。

### やらなかったこと

- エディタ・MCP サーバ・CLI から上限を変える設定は作っていない (どれも `import` の既定の上限で読む。範囲の外)。
- 既定の値は、実際の PSD (`assets/fixtures/psd-real` の 2 つ) が通ることと、大きなキャンバスの見積もりで決めた。
  もっと大きな実ファイルでは確かめていない。

### 起票の候補

- エディタ・MCP サーバで上限を超えたときのエラーを分かりやすく出す (いまは `Unsupported` のメッセージのまま)。
  必要なら、上限を変える設定を持たせる。
- PackBits は確保を入力の 64 倍まで許すので、合計の上限 (4 GiB) が実質の目安になる。より厳しくするなら別に検討。

