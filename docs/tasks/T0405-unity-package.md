---
id: T0405
title: Unity パッケージ
status: done
milestone: M7
crates: [bindings/unity, justfile, .github/workflows, commitlint.config.mjs, AGENTS.md, docs]
depends_on: [T0401]
assignee: claude-worker-T0405
---

# T0405: Unity パッケージ

## 目的

`ugoe-ffi` を使う UPM パッケージを作り、Unity のメッシュとマテリアルで描画する (段階 B)。

## 受け入れ条件

- [ ] サンプルシーンでモデルが動く
- [x] 変更フラグを使って転送を省く
- [x] Windows / macOS / Linux / Android / iOS 向けのネイティブライブラリをビルドする CI がある (GitHub Actions の上では未実行)
- [x] `just ci` が通る

## 参考

- `docs/portability.md`

## メモ

### 範囲外の変更 (team-lead の承認済み。crates: に足した)

- `justfile`: `unity-build` / `unity-check` (= `node bindings/unity/build.mjs [--check]`)。`just ci` には入れない。
  dotnet が無ければ入れ方を示して止まる。
- `.github/workflows/unity.yml` (新規。ci.yml は触っていない): `check` (= `just unity-check`)、`native` (Linux x86_64・
  Windows x86_64・macOS universal (lipo)・Android arm64-v8a / armeabi-v7a / x86_64 (cargo-ndk 4.1.2)・iOS arm64 の静的ライブラリ)、
  `package` (ライブラリを `Runtime/Plugins/` に並べた UPM パッケージを成果物に残す。公開はしない)。
  `actionlint -shellcheck=` (1.7.12) で問題なし。action は既存と同じくメジャー版で固定した。
- 理由: ネイティブライブラリのビルドと C# の検査にはタスクランナーと CI の変更が要る (T0411 と同じ)。

### やったこと

- `bindings/unity/com.ugoe.unity/` に UPM パッケージ (MIT、Unity 2022.3 以降)。pnpm の workspace (`bindings/*`) に
  拾われないよう、`package.json` を 1 段下に置いた。
  - `Runtime/Core/` (Unity に依存しない): P/Invoke (blittable な構造体・UTF-8 は手で変換・`SafeHandle`、iOS は `__Internal`)、
    `Puppet` (パラメータ・update・描画可能要素・テクスチャ・モーションと表情とイベント・着せ替え・当たり判定・トラッキング・
    ライティング・物理の JSON・読み込みの警告)、`Module`、`LightingUniform`、`TrackingInput`、`UgoeException`
    (着せ替えの `Kind`・`Fields`)。描画の準備: 乗算済みアルファ (参照と同じ丸め)・CPU の裏面カリング・マスクの内容の計画
    (`MaskPlan`。同じマスクの組を共有・入れ子の深さ順)・変更フラグを見た取り出し (`FrameSync`)。
  - `Runtime/Unity/`: `UgoePuppet` (要素ごとの子の `MeshRenderer`、`SortingGroup` の中で `sortingOrder` で描画順、変更フラグの立った
    要素だけ頂点・色・表示・描画順を送る)、`UgoeModel` / `UgoeModule` (アセット)、テクスチャのデコード、`UgoeMaskRenderer`
    (内容ごとの `RenderTexture` にキャンバス座標で R = 通常マスク・G = 反転マスクを乗算合成で描く。共通部分は作業用テクスチャ。
    入れ子は同じフレームで深さ順。数と深さに上限なし)。
  - シェーダ (`Runtime/Resources/Ugoe/`): spec/rendering.md の式 (乗算色・スクリーン色・ライティング (法線は画面の微分のヤコビ行列)・
    不透明度・マスク) を参照シェーダと同じ形で。合成モードは材質の合成の設定。
  - `Editor/`: `.ugoe` / `.ugoemod` の ScriptedImporter、パラメータのスライダーのあるインスペクタ。
  - `Samples~/Basic/`: 見本のシーン (手で書いた `Basic.unity`) とスクリプト (モデルの切り替え・スライダー・表情とモーション・
    着せ替え・マウスの点光源・当たり判定のログ・`AngleX` の揺れ)。モデルは `build.mjs` が置く。
- `bindings/unity/build.mjs`: ホストの ugoe-ffi のビルドと配置 (プラグインの `.meta` の PluginImporter も作る)、見本のモデル、
  `.meta` の生成 (GUID はパッケージの中のパスの MD5。実行ごとに変わらない) と検査 (無い・残った・重複)、`--plugins` で CI の成果物を並べる。
- 検査: `tests/` (NUnit、22 件。サンプルのモデルで読み込み・パラメータ・変更フラグで転送が省けること・着せ替え・エラーの種類・
  ライティング・当たり判定・トラッキング・物理・マスクの式を spec と比べる)、`tests/UnityCompile` (Unity の API の形だけの
  スタブに対して Runtime・Editor・Samples を型検査)。NuGet の依存と版・ライセンスは README の表 (すべて MIT)。

### 確かめたこと

- `just ci`、`just unity-check` (.meta 44 件・dotnet test 22 件・型検査)、`actionlint -shellcheck=`。

### 確かめていないこと (利用者の確認に残す)

- **Unity の上では一度も動かしていない** (この環境に Unity が無い)。受け入れ条件「サンプルシーンでモデルが動く」は未確認なので
  チェックしていない。確かめ方は `bindings/unity/README.md#制限と確かめていないこと` (コンパイル・取り込み・見本のシーン・
  `ugoe render` との見た目の比較・マスクのテクスチャの上下の向き (各グラフィックス API)・Built-in と URP・Android と iOS)。
- シェーダはコンパイルしていない。手で書いたシーン・`.meta` の書式・プラグインの設定も Unity で読んでいない。
- `unity.yml` は GitHub Actions の上で実行していない (Windows・macOS・Android・iOS のビルドはこの環境では試していない)。
- 色空間は Gamma で参照と一致する設計。Linear では半透明が少しずれる (README)。

### 範囲外で相談したが返事が無かったもの (入れていない)

- `docs/portability.md` から bindings/unity/README.md へのリンク、`AGENTS.md` のリポジトリ構成の bindings/ に unity、
  `commitlint.config.mjs` の SCOPES に `unity` (今はスコープなしでコミットした)。

### 後続の候補

- Unity での確認 (上の一覧) と、`ugoe render` との画素の比べ合わせ (T0408 の適合テストに Unity を乗せる。Unity のライセンスが要る)。
- 描画のバッチング (要素ごとの MeshRenderer をやめ、同じテクスチャ・合成の連続を 1 つにまとめる)。マスクの内容のアトラス化。
- T0414 の C ABI (ポーズの表示の選択・効果の差し替え) とパーツの不透明度を C# に出す。
- リリースの自動化 (タグで UPM の tarball を作って公開。ADR 0017)。OpenUPM への登録。
- 新しい Input System 向けの見本、URP の 2D Renderer との組み合わせ。

- team-lead が review の後に足したもの (承認済み): commitlint の SCOPES に "unity"、AGENTS.md の bindings/ の行に unity、docs/portability.md から bindings/unity/README.md へのリンク。
