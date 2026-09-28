# ugoe-psd

**層:** T1 素材処理 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

PSD から、PSD の形式に依存しない `LayerTree` を作る。

## 守ること

- PSD パーサの実装の詳細をこのクレートの外に漏らさない (`parse` モジュールは非公開)
- 壊れた入力でパニックしない (途中で切れた入力・ランダムに壊した入力のテストがある)

## 仕様

- [docs/ai-integration.md#psd-取り込み](../../docs/ai-integration.md#psd-取り込み)

## 取り込めるもの

| 項目 | 出力 |
| --- | --- |
| グループ (セクション区切り `lsct`) | `LayerNode::Group` (名前・表示・不透明度・ブレンドモード・ID・クリッピング) |
| ラスタレイヤー | `Layer`。外接矩形で切り出した非乗算 RGBA8 と、キャンバス上の位置 (`offset`) |
| 表示 / 非表示 | `visible` |
| 不透明度 / 塗りの不透明度 (`iOpa`) | `opacity` / `fill_opacity` (0..=1) |
| ブレンドモード | `blend_mode` (PSD のキーから末尾の空白を除いたもの。例: `norm` `mul` `pass`) |
| クリッピング | `clipping` (下のレイヤーでクリッピングされている) |
| レイヤー ID (`lyid`) | `id` |
| Unicode のレイヤー名 (`luni`) | `name` (なければパスカル文字列の名前) |

並び順は **下から上** (描画順) で、PSD のファイル内の順と同じ。

対応する画像は 8 ビットの RGB だけ。PSB・16/32 ビット・CMYK・グレースケールなどは
`PsdError::Unsupported`、壊れた入力は `PsdError::Invalid` になる。
チャンネルの圧縮は無圧縮と PackBits (RLE) に対応する (Photoshop が 8 ビットで書くのはこの 2 つ)。

## 警告 (`LayerTree::warnings`)

取り込みは続けるが、見た目が PSD と変わりうるものを `ImportWarning { path, kind }` で返す。

| `WarningKind` | 扱い |
| --- | --- |
| `AdjustmentLayer` | 調整レイヤー・塗りつぶしレイヤー。取り込まない |
| `SmartObjectWithoutPixels` | 合成済みの画素のないスマートオブジェクト。取り込まない (画素があればラスタとして取り込み、警告しない) |
| `LayerEffectsIgnored` | レイヤー効果。効果を除いた画素を取り込む |
| `LayerMaskIgnored` | レイヤーマスク。マスクを掛けていない画素を取り込む |
| `UnsupportedCompression` | ZIP 圧縮などのチャンネル。取り込まない |
| `MissingPixels` | 色チャンネルがない。取り込まない |
| `FlattenedImage` | レイヤー情報がない PSD。合成画像を 1 枚のレイヤーにする |

## パーサの選択 (T0501)

外部クレートを使わず、必要な部分だけを読む自前のパーサ (`src/parse.rs`、約 500 行) にした。

- 候補だった [`psd`](https://crates.io/crates/psd) クレート (0.3.5、MIT / Apache-2.0、最終リリース 2024-01) は
  グループ・表示・不透明度・クリッピングを読めるが、次の理由で採らなかった。
  - 読み取りカーソルが境界検査をせずにスライスするため、途中で切れた入力でパニックする
    (「壊れた入力でパニックしない」を守れない)。
  - レイヤーの画素はキャンバス全体の大きさで返り、外接矩形で切り出した画像が得られない。幅と高さも `u16`。
  - 塗りの不透明度 (`iOpa`)・PSD のレイヤー ID (`lyid`)・未知の追加情報のキー (調整レイヤー・
    スマートオブジェクト・効果の検出に使う) を取り出す API がない。
- 必要なのはヘッダ・レイヤーレコード・チャンネル画像 (無圧縮 / PackBits) だけで、自前でも小さい。
  依存を増やさず、すべての読み取りを境界検査付きにでき、ライセンスの心配もない。
- ZIP 圧縮のチャンネルは 8 ビットの Photoshop の出力には現れないため、対応していない (警告して飛ばす)。
  必要になったら `miniz_oxide` (MIT) などで足す。

## テスト

- `tests/common/mod.rs` はテスト用の PSD ライター。Adobe の仕様どおりに書き、偽の中身は書かない
  (ZIP のチャンネルも、圧縮しないブロックだけの正しい zlib ストリームで書く)。合成画像は `tests/common/compose.rs`
  でレイヤーから計算する。
- `assets/fixtures/layers.psd` (構造を試す小さなもの) と `character.psd` (目で確かめる絵) は
  `tests/common/fixtures.rs` の定義から作り、コミットしたファイルと一致することをテストする
  (作り直すときは `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-psd --test fixtures`)。
- `tests/fixtures.rs` は、取り込みとは独立の ImageMagick (`identify` / `convert`) でも両方の PSD が読め、
  レイヤーの位置・大きさと合成画像の画素が期待どおりであることを確かめる (ImageMagick が無ければ飛ばす)。
- 壊れた入力 (途中で切れたもの・ランダムに壊したもの・対応の取れないグループ) と未対応の中身
  (ZIP・調整レイヤー・レイヤー効果・画素のないスマートオブジェクト) は、ファイルにせず `tests/import.rs` の中で作る。
- `tests/real_world.rs` は `assets/fixtures/psd-real/*.psd` (実際のペイントツールの PSD) をすべて取り込む。
  置き方は `assets/fixtures/psd-real/README.md`。

## 関連タスク

- T0501 (`docs/tasks/T0501-*.md`)
- T0605 (エディタでの取り込み。`clipping` はマスクに対応させる)
- T0620 (仕様どおりのテスト用 PSD、実ファイルのテストの置き場所、未対応の機能の洗い出し)
