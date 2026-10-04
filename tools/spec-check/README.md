# spec-check — 公開仕様だけで書いた独立の読み込み器と書き出し

公開仕様 ([spec/](../../spec/README.md)) が実装から独立して読めることを確かめるための、TypeScript の小さな実装です
(T0120、[ADR 0021](../../docs/adr/0021-public-specification.md))。参照実装 (`crates/`) のコードは使わず、読みもせずに書きました。

できること:

- `.ugoe` と `.ugoemod` のコンテナの読み込み ([spec/bundle.md](../../spec/bundle.md#reading) の手順 1〜8): ヘッダとチャンクの表の検査、機能の宣言、
  `schema_version` の先読み、zstd の展開、MessagePack の復号、画像の数と署名、知らないチャンク・フィールド・値のフォールバックと警告
- JSON の文書 (`model.json`) の読み込み
- モデル・物理・モーション・表情・モジュールの検証 ([spec/model.md](../../spec/model.md#validation))
- 評価 ([spec/evaluation.md](../../spec/evaluation.md)): 線形と滑らかな補間、ブレンドシェイプ、ワープ・回転デフォーマ (ワープの下の回転、外挿を含む)、パーツとデフォーマの不透明度、グルー、描画順
- 着せ替えのモジュールの取り付け ([spec/model.md](../../spec/model.md#composition))
- 物理 ([spec/physics.md](../../spec/physics.md)): 振り子の鎖、固定の刻みと出力の補間、リグの順と中継、動的な物理 (根のアンカー・重力の座標系・
  デフォーマの角度と原点・頂点と制御点のオフセット)、当たり判定 (円とカプセル、掃引と二分法)
- モーションと効果 ([spec/motion.md](../../spec/motion.md)): 曲線 (Bézier を含む)、フェード・ループ・イベント・優先度とクロスフェード、表情、
  目パチ (SplitMix64)・呼吸・リップシンク、効果のグループ、ポーズとパーツの不透明度の上書き、1 フレームの順 (T0126)。ホストがポーズのエントリを選ぶ操作と効果の設定の差し替え (T0130)
- 書き出し ([spec/bundle.md](../../spec/bundle.md#writing)): MessagePack の符号化、既定値と null の 2 通りの書き方、zstd の圧縮、
  機能の宣言、`.ugoe` と `.ugoemod` のコンテナ。ファイルの例 (`spec/examples/bundles/`) の作成と、参照実装との往復の試験 (T0128)
- 描画 ([spec/rendering.md](../../spec/rendering.md)): CPU の小さなラスタライザ (`render.ts`)。画素の中心での内外の判定と左上の規則、
  乗算済みの双線形・clamp-to-edge、乗算色とスクリーン色、3 つの合成モード、マスク (入れ子・反転・union と intersection・強さ)、
  裏面の除去、法線マップとライティング (環境光・平行光・点光源)、任意のアフィンのビュー変換。描画の例 (`spec/examples/render-*/`) の照合と、
  参照実装の描画との比較 (T0127・T0130)
- 追従 (トラッキング、[spec/avatar-profile.md](../../spec/avatar-profile.md#face-tracking)): 標準の対応と `avatar.tracking` の上書き、
  1 フレームの順の手順 3 (`tracking.ts`、T0130)

しないこと: 当たり判定のヒットテスト、視線 (`look_at`)。描画は例の検証のためのもので、速さを求めない。

## 独立性を保つ手順

この実装を直すエージェント (と人間) は、次を守ります。守れない変更 (参照実装の挙動に合わせるなど) をするなら、先に仕様を直します。

1. **読んでよいもの:** `spec/` (本文・JSON Schema・例のファイル)、ワークフローの文書 (`AGENTS.md`・`docs/tasks/`・`docs/adr/`・`docs/workflow/`・`docs/testing.md`)、
   `justfile`・`package.json`・`scripts/` など実装のアルゴリズムを含まないもの。
2. **読んではならないもの:** `crates/` 以下と `docs/spec/` (日本語の開発用の仕様。実装の型名や経緯を含む)。grep や、ビルドの出力・ログで中身を覗くことも含みます。
3. 期待値は `spec/examples/` のもの (手で導いた値) だけを使います。参照実装の出力を期待値にしません。
4. 仕様が曖昧・不足で実装を決められないときは、推測で参照実装に合わせず、仕様の抜けとして記録して `spec/` を直します
   (`spec/` を直したら `docs/spec/` の同期が要ります。同期は `docs/spec/` を読める別の作業者が行います)。
5. 例外として、参照実装の **外から見える挙動** (`ugoe validate` などのコマンドにファイルを与えて受理・拒否を見ること、`assets/compat/` のバイナリを読むこと) は、
   仕様の曖昧さを見つけるために使ってよいとします。ソースやログは読みません。

## 使い方

Node.js 24 以上 (型の除去で `.ts` を直接実行する。zstd は `node:zlib` のもの) だけで動き、実行時に依存するパッケージはありません。
型検査のためだけに、開発用の依存として `typescript` と `@types/node` (エディタ・ビューアと同じ版) を持ちます (pnpm のワークスペースの一員)。

```sh
node --test tools/spec-check/test/                                  # テスト
node tools/spec-check/src/cli.ts spec/examples/single-quad/model.json X=0.5   # 評価の結果を JSON で表示
node tools/spec-check/src/cli.ts assets/compat/dev/smooth-arc.ugoe
node tools/spec-check/src/cli.ts spec/examples/physics-pendulum          # 時間の例を走らせ、食い違いを表示
node tools/spec-check/src/make-bundles.ts                                # ファイルの例を bundles.json から作り直す
node tools/spec-check/src/roundtrip.ts target/debug/ugoe                 # 参照実装との往復の試験 (下)
node tools/spec-check/src/render-reference.ts target/debug/ugoe          # 参照実装の描画と描画の例の比較 (下)
```

### 参照実装との往復の試験

`src/roundtrip.ts` は、参照実装を黒箱として (コマンドだけを使い、ソースもログも読まずに) 次を確かめる。`ugoe` の実行ファイルが要るので CI には入れず、
書き出しやファイルの例を変えたときに手で実行する (`cargo build --bin ugoe` の後)。

1. 元のデータ (`spec/examples/` の JSON の文書と `.ugoe` の例、`assets/compat/dev/` の `.ugoe`) をこの実装で読む。
2. この実装で `.ugoe` に書く。書き方は 2 通り (既定値と値の無いフィールドを省き圧縮しない / すべて書き MessagePack と META を zstd で圧縮する)。
3. 参照実装の `ugoe validate` が受け付ける。
4. 参照実装の `ugoe build <in> -o <out>` で読んで書き直させ、それをこの実装で読むと元のデータと同じ (f32 に丸めた値で完全一致、画像はバイト単位)。

モジュールのファイルは `ugoe` のコマンドが読めないので、参照実装の側のテストで確かめる。
T0128 の時点では、法線マップのあるバンドル (`lit-quads.ugoe`・`dress-up-base.ugoe`) で `ugoe build` が
`normal map … has no image data` で失敗する (参照実装の `build` の問題。`validate` は受け付ける)。ほかはすべて一致した。
T0127 の時点でも同じで、法線マップのある描画の例 `render-lighting` も同じ理由で失敗する (ほかの描画の例は一致する)。

### 参照実装の描画との比較

`src/render-reference.ts` は、描画の例のモデルとテクスチャをこの実装で `.ugoe` に書き、参照実装の `ugoe render <bundle> --size WxH
(--fit stretch | --view=A,B,C,D,E,F) [--ambient] [--light]` で PNG に描かせて、調べる画素を期待値と許容誤差で比べる (黒箱。ソースもログも読まない)。手で実行する。

- PNG は非乗算なので、読んだ色に alpha を掛けて乗算済みに戻す。alpha が 0 の画素の色と、色が alpha を超える画素の色 (加算の合成・1 を超える光)
  は PNG で表せないので比べない (数だけ表示する)。それらは参照実装の側の GPU のテストで確かめる。
- `view` のあるケースは、例の 6 数をそのまま同じ順で `--view` に渡す (`ugoe render --help` の説明が
  [spec/examples/README.md](../../spec/examples/README.md#rendering-examples) の `view` と同じ式。T0422)。負の数で始まる値をオプションと
  取り違えられないよう `--view=…` と書く。`view` の無いケースは `--fit stretch` (例はキャンバスの縦横比を保つので、例の既定のビュー変換と同じ)。
- CLI は長さ 0 の平行光を引数の誤りとして拒否するので、そのような光源は渡さずに比べる (無効な光源は無視されるので結果は同じはず)。
- T0127 の時点では、比べられる画素はすべて一致した。T0422 の時点では、`view` のあるケース (`render-culling` の 3 つ・`render-lighting` の 3 つ) を含めて
  すべての例を比べ、比べられる画素はすべて一致した (`render-lighting` も描けるようになっていた)。`view` を 3 画素ずらす・上下を反転させると
  これらのケースで食い違いが出ることも確かめた (渡した `view` が効いている)。

型の検査 (`tsc`。`just spec` に含まれる):

```sh
pnpm --filter ugoe-spec-check check
```

## テスト

| ファイル | 内容 |
| --- | --- |
| `test/examples.test.ts` | `spec/examples/` の評価の例 (`expected.json`) をすべて評価し、[許容誤差](../../spec/examples/README.md#tolerances) で期待値と比べる。 |
| `test/timeline.test.ts` | `spec/examples/` の時間の例 (`timeline.json`) をはじめからフレームごとに走らせ、[許容誤差](../../spec/examples/README.md#timeline-tolerances) で期待値と比べる。ファイルの例のうち時間の例と同じ中身のもの (`physics-pendulum.ugoe`・`motion-crossfade.ugoe`・`expressions.ugoe`) は、バンドルから読んだデータでも走らせる (T0129)。 |
| `test/compat.test.ts` | `assets/compat/` の資料を読む。`dev/` は警告なしに読めて評価できる、`future/` は仕様どおりに劣化して読める (評価の結果が元のファイルと同じ) か拒否される ([docs/testing.md](../../docs/testing.md#互換性テスト))。 |
| `test/bundles.test.ts` | ファイルの例 (`spec/examples/bundles/`) を読み、目録 `bundles.json` (チャンクの表・結果・同じ中身の JSON の文書・画像の色) と比べる。目録から書き出しで作り直したものと、展開したペイロードが同じ。 |
| `test/writer.test.ts` | MessagePack の符号化 (整数と長さの境目、float 32 と 64)、整数と `f32` のフィールドの型、2 通りの書き方、知らない値と基準の機能の宣言の拒否、コンテナの配置、すべての例と互換性の資料のこの実装での往復。 |
| `test/render.test.ts` | `spec/examples/` の描画の例 (`render.json`) をこの実装のラスタライザで描き、[許容誤差](../../spec/examples/README.md#rendering-tolerances) で調べる画素と比べる。調べる画素が三角形の辺から 1 画素以上離れていること (例の作り方の規則)、隣り合う三角形が共有する辺の画素を 1 度だけ塗ること、乗算済みでの補間。 |
| `test/reader.test.ts` | コンテナの誤り (ヘッダ・表・重なり・許されないチャンク・画像の数)、機能の宣言、MessagePack、JSON の整数と `null`、知らない値、検証。 |
| `test/physics.test.ts` | 物理の時間の進め方のうち時間の例で確かめにくいもの: 1 回の更新の刻みの上限を超える巨大な `dt` の後、残りの時間が割り算の余りになり、物理が進み続ける。 |

### CI

型検査とテストは `just spec` (`just ci` に含まれる) で実行する。1 秒ほどで終わるので、公開仕様の例や `assets/compat/` の資料が変わったときに、
仕様だけで読めることが保たれているかを毎回確かめる。参照実装と食い違ったら、どちらが仕様に合っていないかを調べる
(仕様が曖昧なら仕様を直す)。

## 設計の選択

- 計算は倍精度で行う。仕様は単精度以上を求め、許容誤差の内ならよいとしている ([spec/evaluation.md](../../spec/evaluation.md#numbers-and-determinism))。
- 整数と浮動小数点数を区別するため、MessagePack の整数と、JSON の小数部も指数部もない数を `bigint` で読む
  (`JSON.parse` の reviver の `context.source` を使う)。`1.0` は整数のフィールドでは拒否する。
- 知らない値は読むときにフォールバックに置き換えて警告を残す。検証に影響する `Parameter.kind` とデフォーマの種類だけは「知らない値」のまま持つ。
  そのため書き出しは、型の上に残る「知らない値」を拒否し (`WriteError`)、警告のあったデータは書かない (呼び出し側が確かめる)。
- 書き出しは型の決まったデータ (`types.ts`) から書く。整数の型のフィールドは MessagePack の整数、`f32` は値が整数でも float 32。
  作るときのデータは既定値なら常に省く (仕様の SHOULD)。アバタープロファイルは書き出しのためにすべてのフィールドを読む。
- ファイルの例の zstd の圧縮結果は Node.js (zstd) の版で変わりうるので、テストは展開したペイロードで比べる。
- MessagePack の値の後ろに余りのバイトがあるペイロードは拒否する (仕様はどちらも許す。参照実装は無視する)。
- 目パチの `seed` (`u64`) は `bigint` で正確に読む (2^53 を超えうる)。間隔は単精度に丸める (仕様は単精度の `u` を求める)。
- 時間の例の期待値は、この実装の出力を写したものではない。仕様から別に書いた転記 (T0126 の作業者が Python で書いた使い捨てのもの) で出し、
  静止の姿勢・閉じた式で書ける値は手で確かめた。この実装はそれと独立に書き、一致を確かめた。
- 描画の例の期待値は、この実装の出力を写したものではない。調べる画素だけで仕様の式を直接計算する別の転記 (T0127 の作業者が Python で書いた
  使い捨てのもの) で出し、合成とマスクの係数は手で確かめた。この実装のラスタライザはそれと独立に書き、一致を確かめた。
- 描画は倍精度の実数で行い、描くたびに [0, 1] に切り詰める (8 ビットに量子化しない)。テクスチャの乗算済みへの変換も実数で行う。
- Bézier の `s` は二分法で解く (仕様は `|b(s) − x| ≤ 1e-6` を求め、解き方を問わない)。

## ライセンス

MIT (リポジトリのツールと同じ)。テストは `spec/` の例 (CC BY 4.0) を実行時に読むだけで、このディレクトリに写していません。
