# 0025: Web のランタイムのパッケージ (@ugoe/runtime) は MIT で bindings/web に置く

- 状態: Accepted (2026-10-01、ライセンスは利用者の判断)
- 日付: 2026-10-01
- 関連: T0404、ADR 0017 (単一ランタイムと 1 行導入)、ADR 0018 (ライセンス)、ADR 0019 (ビューアは GPL-3.0-only)

## 背景

ADR 0017 で、Web 向けの配布物を npm の `@ugoe/runtime` (wasm + TypeScript の型) と Web Components の `<ugoe-avatar>` と決めた。
リポジトリの npm パッケージはこれまで、エディタ (`apps/editor`)・ビューア (`apps/viewer`) と、その 2 つが共有する部品の
`packages/` (`@ugoe/face-tracking` など) で、すべて GPL-3.0-only である (ADR 0018・0019)。
一方、`@ugoe/runtime` はゲームやアプリ・Web サイトに組み込まれる**ランタイム**で、ADR 0018 がランタイムを MIT にした理由
(商用・クローズドソースを含めて制約なく組み込める) がそのまま当てはまる。置き場所とライセンスの境界を決める必要がある。

## 決定

1. **`@ugoe/runtime` (`<ugoe-avatar>` を含む) は MIT にする。** ADR 0018 の「ランタイム・ライブラリは MIT」を npm の配布物に広げたもの。
   中身の wasm は MIT の `ugoe-wasm` クレート (`crates/`) から作るので、クレートとパッケージのライセンスは揃う。
2. **置き場所は `bindings/web/` にし、`packages/` とは分ける。** `packages/` は「アプリ (GPL) が共有する部品」の場所として GPL-3.0-only のままにする。
   `bindings/` は `crates/` のランタイムを別の言語・環境へ出すためのパッケージの場所で、MIT とする
   (今後の Unity・Godot などのバインディング (T0405〜T0407) も、別の決定が無ければここに置く)。
3. **境界の向き:**
   - GPL のアプリ (`apps/*`)・部品 (`packages/*`) は MIT の `@ugoe/runtime` を使ってよい (MIT は GPL-3.0 と両立する)。
   - **MIT の `bindings/` は GPL のコード (`apps/`・`packages/`) を import せず、コードを写さない。** 読んで参考にするのはよいが、
     同じことが要るなら自分で書く (ADR 0019 のとおり、ビューアのコードを写した見本は GPL になる)。
   - `@ugoe/runtime` の実行時の依存 (`dependencies`) は持たない。持つ場合は寛容なライセンス (`scripts/check-licenses.mjs` の `PERMISSIVE`) に限る。
     これを `just licenses` で検査する (パッケージの `license` が MIT であること、`dependencies` が寛容なライセンスで使えること)。
4. **見本 (`bindings/web/examples/`) も MIT** にする。ADR 0019 で「MIT の見本が要るときは別に用意する」とした、Web 向けの MIT の見本に当たる。
   見本のモデルは MIT の `assets/samples/` を使う。

## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| `packages/runtime` に置き、そこだけ MIT にする | pnpm のワークスペースの設定が要らない | 「`packages/` は GPL」という規則に例外ができ、GPL の部品を誤って import しやすい。ライセンスの境界がディレクトリで見えない |
| GPL-3.0-only にする (ほかのフロントエンドと同じ) | ライセンスが 1 つで済む | Web サイトやアプリに組み込むとその全体が GPL になり、ランタイムを MIT にした ADR 0018 の目的に反する |
| `crates/ugoe-wasm` の中に npm パッケージを置く | wasm と近い | Rust のクレートの中に Node のツールチェインが入る。`crates/` の検査 (層・ライセンス) と混ざる |

## 結果

- Web のサイトやアプリは、`npm i @ugoe/runtime` (または `<script type="module">`) でソースの開示なしにランタイムを組み込める。
- ライセンスの境界がディレクトリで決まる: `crates/`・`bindings/`・`docs/` は MIT、`apps/`・`packages/` は GPL-3.0-only。
- `packages/` の部品 (フェイストラッキングなど) を `@ugoe/runtime` から使いたくなったら、MIT で書き直して `bindings/` に置くか、
  作者の了解を得て部品のライセンスを変える (別の ADR)。
- npm への公開 (リリースの自動化) は別のタスクで行う。それまでパッケージは `private: true` で、`pnpm pack` の tarball で確かめる。
  (T0410 で手順と CI のワークフロー `web-runtime-release.yml` を用意した。`private: true` を外して公開するかは利用者が決める。
  `bindings/web/README.md#リリース`)

## 追記: フェイストラッキング (T0410)

`<ugoe-avatar>` のフェイストラッキングは、**ホストから入力を渡す口だけ**を持ち、トラッカー (カメラ・MediaPipe) は持たないことにした。

- `UgoeAvatarElement.setTracking(input)` / `clearTracking()` と、依存のない変換 `trackingFromFaceLandmarker`
  (MediaPipe Face Landmarker の結果の形 → 入力)・`mirrorTracking`・`headRotationFromMatrix` を `@ugoe/runtime` に置く。
  MediaPipe の型は import せず、使う形だけを書く。変換は仕様 (`docs/spec/avatar-profile.md` の向きと左右の約束) から自分で書いた
  (GPL の `packages/face-tracking` は読んだが写していない)。
- 平滑化・キャリブレーション・顔を見失ったときの扱いは、これまでどおりホストが行う (`docs/spec/avatar-profile.md#実装の範囲`)。

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| **入力を渡す口だけ (採用)** | 実行時の依存が増えない (ADR の決定 3)。トラッカーを選べる (MediaPipe 以外・サーバー側の推定・録画の再生)。パッケージの大きさが変わらない | カメラから動かすまでの数十行はホストが書く (README に例を置いた) |
| MIT で書き直したトラッカーを同梱する (`packages/face-tracking` 相当) | 属性 1 つでカメラから動かせる | `@mediapipe/tasks-vision` (Apache-2.0。寛容なので可) が実行時の依存になり、wasm (約 10MB) とモデル (約 4MB) の置き場所・CDN・カメラの許可の UI まで決める必要がある。平滑化・キャリブレーションを含めると GPL の実装を写さずに書き直す量が大きい |

トラッカーの同梱は、要望があれば別のパッケージ (`bindings/web` の中の `@ugoe/tracking` など、MIT) として検討する。
