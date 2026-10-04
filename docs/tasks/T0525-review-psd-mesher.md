---
id: T0525
title: コードレビュー: ugoe-psd と ugoe-mesher
status: done
milestone: M9
crates: [ugoe-psd, ugoe-mesher, docs]
depends_on: []
assignee: claude-worker-T0525
---

# T0525: コードレビュー: ugoe-psd と ugoe-mesher

## 目的

ugoe-psd と ugoe-mesher のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-psd` と `crates/ugoe-mesher` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 壊れた・巨大な PSD (長さの偽装・圧縮の展開・レイヤーの深さ) で止まらないか、確保の上限
  - メッシュの生成の退化 (透明だけのレイヤー・細い形・1px)、決定性
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0531〜T0535 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `ugoe-psd/src/lib.rs` (`import`) | グループを深く入れ子にした PSD (レイヤー数の上限 32767 で約 16000 段) で、`LayerTree` を捨てる・`Debug` で表示する再帰がスタックをあふれさせ、プロセスごと落ちる | 直した (256 段を超えたら `Unsupported`。先に落ちるテストを書いた。README に追記) |
| 高 | `ugoe-mesher/src/lib.rs` (`generate`) | `margin` が大きい (1e6・`f32::MAX`) と、余白の `margin.ceil() as usize + 2` があふれる・格子の確保が爆発する。オプションは JSON (エディタの IPC・MCP) から来る | 直した (`margin` は 1024 まで。テスト。README に追記) |
| 高 | `ugoe-mesher/src/lib.rs`・`src/contour.rs` | `edge_spacing`・`interior_spacing` が極端に小さい (1e-30 など) と、頂点数を確かめる前に点を作り続け (確保の爆発・終わらない)、内部点の格子の `as i64 + 1` があふれる | 直した (輪郭の点の数を再サンプリングの前に見積もり、内部点の格子が上限の 256 倍を超えたら点を作らずにやり直す。テスト) |
| 中 | `ugoe-psd/src/parse.rs` (`inflate`) | ZIP のチャンネルはレイヤーの矩形の大きさ (1 辺 60000) までそのまま展開し、全レイヤーの分を持ってから木を作る。小さな ZIP 爆弾で数十 GB を確保させられる (無圧縮・PackBits は入力の大きさで抑えてある) | T0531 を起票 (上限の値は設計の判断) |
| 中 | `ugoe-mesher/src/lib.rs`・`src/contour.rs` | 作業用の格子が中身の範囲でなく画像全体で、1 セル 20 バイトほど確保する。輪郭の辺 ID が `u32` で、2^31 セルを超えるとあふれる | T0533 を起票 |
| 低 | `ugoe-editor-core/src/automesh.rs` (`AutoMeshOptions::check`) | mesher の検証を写した別の実装で、今回の `margin` の上限を知らない (`generate` が `InvalidOptions` を返すので壊れはしない) | T0532 を起票 (範囲の外) |
| 低 | `ugoe-mesher/src/points.rs` (`triangulate_points`) | 非正規化数 (1e-44 など) の座標を spade が拒み、点が黙って落ちる。「挿入は失敗しない」というコメントが誤り | 直した (0 に寄せる。テスト) |
| 低 | `ugoe-psd/src/lib.rs`・`ugoe-mesher/src/lib.rs`・`points.rs` | 公開のエラー型 (`PsdError`・`MeshError`・`TriangulateError`) と `MeshOptions` に doc コメントがない | 直した |
| 低 | `ugoe-psd/src/parse.rs` (`parse_masks`) | 無効にしたマスクでも、矩形が範囲の外なら取り込み全体が `Invalid` になる | 見送り (壊れた入力で、`Invalid` で害はない) |
| 低 | `ugoe-psd/src/parse.rs` (`parse`) | レイヤー情報が空で、後ろに 1〜3 バイトだけ残る PSD は、グローバルなマスク情報を読めず `Invalid` になる | 見送り (そう書く書き手を確認していない) |
| 低 | `ugoe-psd/src/parse.rs` (`parse_composite`) | レイヤーのない PSD の 4 番目のチャンネルを常に透明度とみなす。保存した選択範囲のアルファチャンネルだと見た目が変わる | 見送り (仕様が曖昧。実例を見てから) |
| 低 | `ugoe-psd/src/parse.rs` (`read_channel_images`) | 使わない追加のチャンネル (ID 3 以上) が未対応の圧縮方式でも、レイヤーごと取り込まない | 見送り (実例がない) |
| 低 | `ugoe-mesher/src/points.rs` (`even_odd`)・`triangulate.rs` (`EvenOdd`) | 同じ制約辺が 2 回あると偶奇で打ち消し合う | 見送り (doc の偶奇規則どおり。editor-core は重複を渡さない) |

確かめて問題のなかったこと:

- 壊れた・巨大な PSD: すべての読み取りは境界検査付きで、長さの偽装 (チャンネル・ブロック・`luni`・RLE の行の表) は確保の前に弾く。
  PackBits は確保を入力の 64 倍までに抑えている。途中で切れた入力・ランダムに壊した入力のテストがある。外から来る文字列をバイトの位置で切る箇所はない。
  画素数の掛け算は `usize`・`u64` で、`u32` のあふれはない。
- メッシュの退化: 透明だけ (`Empty`)・1px・細い線・鞍点・穴・小島のテストがある。三角形は spade の厳密な述語で作り、座標は先に f32 に丸めるので
  面積 0 の三角形は出ない。NaN の座標とオプションは入口で弾く。決定性: 乱数・ハッシュ順・並列処理を使っていない (テストあり)。
- `cargo clippy -W clippy::pedantic`: 型変換の警告が主で、見た範囲では値の範囲を確かめてある。

### 横断 (T0008 向け)

- **外から来る数値の上限:** JSON (IPC・MCP) から来るオプションを「有限で 0 以上」だけ確かめ、上限を見ない検証がある (mesher の `margin`・間隔)。
  `f32` から `usize`・`i64` への `as` は飽和するので、その後の `+ 1`・`* 2` があふれて panic する。ほかのクレートのオプション (ai の `RigParams` など) も同じ形を確かめたい。
- **再帰するデータの深さ:** 外から読む木 (PSD のグループ、.ugoe のパーツ・デフォーマの階層、MCP の JSON) は、作る処理が再帰でなくても、
  derive した `Drop`・`Clone`・`Debug` と下流の処理が再帰する。深さの上限があるかを確かめたい。
- **展開の爆発:** 圧縮を展開する箇所 (PSD の ZIP、.ugoe のバンドル、PNG のテクスチャ) で、展開後の大きさの上限があるか。
- **検証の写し:** 同じ検証を 2 か所に写している (mesher と editor-core の `AutoMeshOptions::check`)。片方だけ直すと食い違う。
