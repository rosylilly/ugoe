---
id: T0128
title: 公開仕様: .ugoe の例のファイルと、独立の書き出し・往復の試験
status: done
milestone: M5
crates: [spec, tools/spec-check, docs, ugoe-format, docs/testing.md]
depends_on: [T0120]
assignee: claude-worker-T0128
---

# T0128: 公開仕様: .ugoe の例のファイルと、独立の書き出し・往復の試験

## 目的

T0120 の後続の候補。バイナリの資料は `assets/compat/` にあるが仕様の一部ではなく、`spec/examples/` は JSON だけ。
また書き出し (spec/bundle.md#writing) は独立の実装がない。

## 受け入れ条件

- [x] 作業者は `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する
- [x] `spec/examples/` に小さな `.ugoe` (と `.ugoemod`) の例を置く (圧縮あり・なし、テクスチャ、機能の宣言)。大きさを抑える
- [x] `tools/spec-check` に書き出しを足し、独立の書き出し → 参照実装の読み込み (`ugoe validate` など黒箱で) → 独立の読み込み、の往復で内容が保たれることを確かめる
- [x] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md`・`spec/bundle.md`

## メモ

### 独立性を保った手順

- 作業者 (claude-worker-T0128) は `crates/` と `docs/spec/` を読まず、`spec/`・`tools/spec-check/`・ワークフローの文書だけを参照した (T0120 と同じ手順)。
- 参照実装は黒箱として、`target/debug/ugoe` の `validate`・`build`・`info`・`--help` の出力 (受理・拒否と誤りの文) と、
  `cargo test -p ugoe-format --test spec_examples` の成否の行だけを見た。ソースとビルドのログは読んでいない。

### やったこと

- `tools/spec-check` に書き出しを足した: MessagePack の符号化 (`msgpack.ts`)、型の決まったデータからのペイロードとコンテナの書き出し
  (`write.ts`。既定値と null を省く `minimal` とすべて書く `explicit`、zstd、FEAT、知らない値と基準の機能の宣言の拒否)、1 色の PNG の読み書き (`png.ts`)。
  書き出しのためにアバタープロファイルをすべて読むようにした (これまでは `level` だけ)。
- `spec/examples/bundles/` にファイルの例 10 個 (各 2 KB 以下) と目録 `bundles.json` (チャンクの表・結果・同じ中身の JSON の文書・画像の色)。
  圧縮あり・なし、ペイロードの順が表と違うもの、知らないチャンク (圧縮の印付きで中身が zstd でないもの・長さ 0)、META、物理・モーション・表情、
  テクスチャ 4 枚と法線マップ 3 枚 (同じ名前を 2 枚が名指す)、任意と必須の試験用の機能、モジュールのファイル。形式は `spec/examples/README.md#file-examples`。
  ファイルは `node tools/spec-check/src/make-bundles.ts` で目録と JSON の文書から作る。
- テスト (`just spec` に含まれる): `test/bundles.test.ts` (例を読んで目録と比べる・作り直したものと展開したペイロードが同じ)、
  `test/writer.test.ts` (符号化の境目、型、書き方、拒否、配置、全例と互換性の資料のこの実装での往復)。
- 参照実装との往復 `src/roundtrip.ts` (手で実行。CI には入れない): 独立の書き出し → `ugoe validate` → `ugoe build` で書き直し → 独立の読み込みで、
  spec/examples の全 18 例・新しい .ugoe の例 9 個・assets/compat/dev の .ugoe 9 個 × 2 通りの書き方で内容が一致した。
  例外は法線マップのあるもの 2 つ (下の参照実装の問題)。`required-feature.ugoe` は参照実装も `unsupported required features: test.example_required` で拒否した。
- この実装の誤りを直した: モジュールの `requires.avatar_level` を、プロファイルの無い本体が level 0 の要求で満たしていた (仕様は満たさない)。テストを足した。

### 見つけた仕様の抜け・曖昧さ

| # | 箇所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 1 | bundle.md Chunk types | 知らないチャンクの試験に使えるタグの予約がない (機能 ID は `test.` を予約している) | 先頭のバイトが `X` のタグを試験用に予約した (互換性の資料の `XFUT` とも合う) |
| 2 | bundle.md Textures and normal maps | 同じ法線マップを名指す複数のテクスチャの NRML の中身が同じでなければならないか、どれを使うかが書かれていない | 各テクスチャは自分のチャンクを使い、書き出しは同じ画像を入れるのがよい (SHOULD) と明記 |
| 3 | model.md Parameters | `Parameter.mirror` は「作るツールだけが使う」とあるが Authoring data の節に入っておらず、ツールが落としてよいか (MAY drop) が分からない | 未修正。この書き出しは値があれば書く |
| 4 | bundle.md Payload encoding | 長さ 0 のペイロードが、別のペイロードの先頭と同じ位置にあってよいか | 「何とも重ならない」から許されると読める (参照実装も受け付けた)。修正なし |

- 1・2 と examples/README.md の `docs/spec/` の同期は team-lead が行った (a450b52。docs/testing.md も)。

### 参照実装の側 (team-lead が足した)

- 参照実装の側のテストは team-lead が足した (80bbe55。作業者は読んでいない): `spec_examples` が bundles.json のあるディレクトリを受け入れ、
  bundles.json の各ファイルを参照実装で照合する。ファイルの例 10 件すべてが一致した (チャンクの表・圧縮・payload_order・FEAT、読んだ中身と警告、
  required-feature の拒否、layout.ugoe の Xzst を展開せずに読み飛ばすこと)。
- そのため crates: に ugoe-format (テストだけ) と docs/testing.md を足した (team-lead の承認)。
- bundles.json の `error` の値は、team-lead の指示で spec/bundle.md#reading の手順名にした (fad754c)。

### 参照実装の問題 (範囲外、後続の候補)

- `ugoe build <in>.ugoe -o <out>.ugoe` が、法線マップのあるバンドルで必ず `normal map `<名前>` has no image data` で失敗する
  (参照実装自身が書いた `assets/compat/dev/dress-up-base.ugoe` でも)。`ugoe validate` は受け付ける。
- CLI は `.ugoemod` を読めない (`validate` が MODU を拒否する)。モジュールのファイルは参照実装の側のテストでしか確かめられない。

### 確かめていないこと・残した課題

- KTX2 の画像の例は作っていない (正しい KTX2 を書くのが大きいため。読み込み器は署名しか見ない)。
- 参照実装との往復は手で実行する (実行ファイルが要るため CI に入れていない)。

### 後続の候補

- 上の参照実装の問題 2 つ (`ugoe build` の法線マップは team-lead が別のタスクに起票する。CLI のモジュールの読み込み)。
- `Parameter.mirror` の扱いの整理 (上の 3)。
- 物理・モーションの例の `.ugoe` を時間の例として走らせる (いまは中身が JSON と同じことだけを確かめる)。
