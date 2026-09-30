---
id: T0410
title: npm: @ugoe/runtime の公開の準備・wasm の縮小・<ugoe-avatar> のフェイストラッキング
status: done
milestone: M7
crates: [bindings/web, ugoe-wasm, docs, .github/workflows]
depends_on: [T0404]
assignee: claude-worker-T0410
---

# T0410: npm: @ugoe/runtime の公開の準備と改善

## 目的

T0404 の後続の候補をまとめる。

## 受け入れ条件

- [x] npm への公開の手順とリリースの自動化 (版の付け方・CI からの公開・`private: true` の外し方)。公開そのものは利用者が行う
- [x] wasm の縮小 (いま約 8MB、gzip で約 2MB)。wasm-opt・機能の切り分け・不要な依存の見直し。数字をメモに書く
- [x] `<ugoe-avatar>` のフェイストラッキング。GPL の `packages/face-tracking` は使えないので、MIT で書き直すか、ホストから入力を渡す口だけにするかを決める (ADR 0025 との関係)
- [x] モーション入りの見本で `motion` 属性を確かめる。WebGPU が作れないときの WebGL2 への切り替えを試す方法を用意する
- [x] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0404-npm-web-components.md` のメモ、`docs/adr/0025-web-runtime-package.md`

## メモ

- **リリース (`bindings/web/README.md#リリース`)**:
  - 版の正はルートの `Cargo.toml` の `[workspace.package] version` (すべての配布物で揃える。docs/portability.md)。
    `bindings/web/scripts/release.mjs` の `sync` (package.json の版をクレートの版に)・`check [v<版>]` (タグとも比べる)・
    `publish [--dry-run]` (`private: true` の間は公開せず `npm pack --dry-run` を表示して成功。プレリリースは dist-tag `next`。
    GitHub Actions では `--provenance`)。単体テスト (`scripts/release.test.mjs`) が package.json とクレートの版の一致を検査する。
  - CI: `.github/workflows/web-runtime-release.yml` (新規。範囲外なので team-lead の承認を得て crates: に `.github/workflows` を足した。
    npm への公開を CI から行うにはワークフローが要り、ほかの置き場所が無いため)。`v*` のタグ (editor-build.yml と同じ) と
    手動 (既定は dry-run) で起動し、版の検査 → wasm/TS のビルド・型検査・テスト・ライセンス検査 → `release.mjs publish`。
    認証は npm の trusted publishing (OIDC、`id-token: write`) か `NPM_TOKEN`。`private: true` を外すかは利用者の判断と README に書いた。
  - 確かめたこと: `check` の成功・タグの食い違いでの失敗、`sync`、private のままの `publish --dry-run` (内容の表示)、
    private を外してプレリリースの版にした写しでの `publish --dry-run` (`tag next and public access (dry-run)`)。
    `actionlint -shellcheck=` (1.7.12。shellcheck 0.9.0 との連携が JSON を読めず失敗するので外した) で検査して問題なし、
    js-yaml でも読めることを確かめた。**ワークフロー自体は GitHub で動かしていない。**
- **wasm の縮小**: `bindings/web/scripts/build.mjs` だけで行った (ルートの Cargo.toml・scripts/wasm.mjs・`just wasm` は変えない。
  エディタとビューアの wasm は今のまま)。cargo の `--config` でプロファイル `web-dist` (release を継承、opt-level "z"・lto "fat"・
  codegen-units 1、毎フレーム動くクレート `glam`・`ugoe`・`ugoe-core`・`ugoe-motion`・`ugoe-physics`・`ugoe-render`・`ugoe-runtime` は
  opt-level 3) を定義し、wasm-bindgen に `--remove-name-section --remove-producers-section`。ランタイムの機能は切り分けていない。
  数字 (バイト、Node の zlib。gzip は -9、brotli は既定の品質 11):

  | 設定 | 大きさ | gzip | brotli | 評価の速さ |
  | --- | --- | --- | --- | --- |
  | 前 (release・thin LTO・名前の節あり) | 7,920,320 | 1,974,207 | 1,310,795 | 約 19µs/フレーム |
  | 前 + 名前の節を外す | 6,090,050 | 1,750,831 | — | — |
  | O3 fat + 名前なし | 5,516,790 | 1,685,250 | — | — |
  | Os fat + 名前なし | 4,999,821 | 1,376,175 | — | 約 43µs |
  | Oz fat + 名前なし (すべて) | 4,354,077 | 1,250,656 | 933,127 | 約 48µs |
  | **採用: Oz fat + 毎フレームのクレートは O3 + 名前なし** | **4,452,501** | **1,300,532** | **964,480** | **約 18〜19µs** |
  | Oz fat + wasm-opt -Oz (binaryen 133) | 3,884,919 | 1,378,282 | — | — |

  評価の速さは、ヘッドレス Chrome (WebGL2) で見本のアバターの `set_parameter`×2 + `update(1/60)` + `deformer_point` を 2000 回回した平均。
  wasm-opt は生の大きさは減るが gzip 後が増えるので使わない。残りの大部分は `rmp_serde` (約 0.8MB)・naga (約 0.55MB)・wgpu。
  最終のビルド (main に rebase した後) の wasm を tarball に入れた見本を、ヘッドレス Chrome の WebGL2 と WebGPU の両方で描画できることを
  確かめた (`<ugoe-avatar>` と `createPuppet` の両方。エラーなし)。`scripts/wasm.mjs`・`justfile`・ルートの `Cargo.toml` は変えていないので、
  エディタとビューアの wasm (`just wasm`) は変わらない (`git diff main -- scripts justfile Cargo.toml apps` は空)。
  名前の節を外したので、パッケージの wasm のパニックのスタックトレースには関数名が出ない (エディタ・ビューアの wasm には出る)。
- **フェイストラッキング**: 「ホストから入力を渡す口だけ」にした (ADR 0025 に追記)。`<ugoe-avatar>` の `setTracking(input)` /
  `clearTracking()` / `tracking`、依存のない変換 `trackingFromFaceLandmarker` (MediaPipe の結果の形 → 入力。`mirrored` で左右反転の
  映像を直す)・`mirrorTracking`・`headRotationFromMatrix`・`trackingArguments` (`src/tracking.ts`、Vitest)。変換は仕様の向きの約束から
  自分で導いた (Ry·Rx·Rz の分解。GPL の `packages/face-tracking` は写していない)。トラッキングの間は目パチを止め、`paused` の間も
  `update(0)` で反映し、読み直し・作り直しの後に渡し直す。`clearTracking` は書き込んでいたパラメータを既定値 (`parameters` 属性の値) に戻す。
  README に MediaPipe (ホストが入れる、Apache-2.0) からつなぐ例を書いた。
- **`motion` 属性**: モーション入りの見本を作るスクリプト `examples/basic/make-motion-sample.mjs` (`npm run sample:motion`。
  `ugoe sample avatar --project` に手書きの `motions/` (idle・nod・shake) と `expressions/` (smile) を足して `ugoe build`。出力は gitignore) を足した。
  ヘッドレス Chrome で WebGL2・WebGPU (SwiftShader) の両方で `motion="nod"` / `"shake"` (force)・`"idle"` と `expression="smile"` が動くことを確かめた。
  そのとき、再生中のモーションより優先度の低い `motion` 属性 (idle) が黙って無視される不具合が分かったので、始められるまで毎フレーム試すようにした。
- **WebGL2 への切り替えの試し方**: 見本に `?simulate=no-adapter` (アダプタが無い → 同じ canvas で WebGL2) と
  `?simulate=device-error` (`requestDevice` が失敗 → `<ugoe-avatar>` は新しい canvas の WebGL2 で作り直す。`createPuppet` の例も作り直すようにした) を足した。
  WebGPU のある Chrome で両方とも webgl2 で描画し、エラーが無いことを確かめた (README の「WebGPU と WebGL2」)。
- スクリーンショット: scratchpad の `T0410/shots/` (`webgpu-simulate_device_error-page.png`・`*-nod.png`・`*-smile.png`・`*-tracking-paused.png` など)。
- **確かめていないこと**: CI のワークフローの実行と実際の npm への公開 (trusted publishing・トークン)、本物のカメラと MediaPipe でのトラッキング
  (合成の入力と、合成した行列の単体テストだけ)、本物の GPU、Firefox・Safari、GPU を失ったときの作り直し。
- **範囲外の変更**: `.github/workflows/web-runtime-release.yml` (新規。team-lead の承認済み)。
- **後続の候補**: `rmp_serde` の読み込みのコードの大きさの見直し (ugoe-format/ugoe-model。untagged・flatten の展開など)、
  エディタ・ビューアの wasm にも同じプロファイル (名前の節は開発中は残す) を使うか、トラッキングの平滑化・キャリブレーションを MIT で
  別パッケージにするか、適合テスト (T0408) への組み込み、見本のモデル (`assets/samples/`) にモーションを入れる。
