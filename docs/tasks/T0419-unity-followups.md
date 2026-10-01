---
id: T0419
title: Unity: T0414 の API を C# に出す・描画のバッチング・UPM のリリースの自動化・Linear の色空間
status: done
milestone: M7
crates: [bindings/unity, .github/workflows, docs]
depends_on: [T0405, T0414]
assignee: claude-worker-T0419
---

# T0419: Unity の後続

## 目的

T0405 の後続の候補をまとめる。

## 受け入れ条件

- [x] T0412・T0414 で足した C ABI (show_pose・set_effects_json・part_*・motion_event・physics_json) を C# に出し、dotnet のテストを足す
- [x] 描画のバッチング (要素ごとの MeshRenderer と SortingGroup をまとめる) を検討し、効くなら入れる
- [x] UPM パッケージのリリースの自動化 (タグで zip を作る。公開は利用者が行う)
- [x] Linear の色空間でも半透明の色が参照と合うようにするか、Gamma を要求して README に書く
- [x] 実際の Unity での確認 (シェーダのコンパイル・シーンと .meta・マスクの上下の向き・Built-in と URP・Android と iOS) は、できなければ利用者の確認に残す
- [x] `just ci`・`just unity-check` が通る

## 参考

- `docs/tasks/T0405-unity-package.md` のメモ、`bindings/unity/README.md`

## メモ

### やったこと

- **C# の API (T0412・T0414 の C ABI):** `Ugoe.Puppet` に `PartIds`・`FindPart`・`GetPartOpacity` (`PartOpacity { Value, Weight }`。
  直前の評価に渡した上書き)・`ShowPose(part) -> bool`・`SetEffectsJson(json)`、`UgoePuppet.ShowPose`。モーションのイベントと
  物理の JSON は T0405 で出してあったのでそのまま。パーツの ID は着せ替えの後に読み直す。
  テスト (`tests/HostApiTests.cs`) は公開仕様の例の期待値と比べる: `pose-select` (フレーム 0〜5)、`effects-replace` (全フレーム)、
  `bundles/motion-crossfade.ugoe` のイベント (フレーム 0〜3)。
- **描画のまとめ (効くので入れた。既定で有効、`UgoePuppet.batching` で切れる):** `Runtime/Core/DrawBatches.cs` の `DrawBatchPlan` が、
  docs/spec/rendering.md#描画の最適化 の条件 (描画順で隣り合い、テクスチャ・合成モード・読むマスクの内容が同じ。マスクを読まない要素は
  どの内容ともまとめる。描画順は入れ替えない) でまとめ、まとめたものごとに 1 つのメッシュと MeshRenderer を作る。要素ごとの値
  (乗算色と不透明度・スクリーン色・mask_strength・ライティングの種類) はプロパティブロックから頂点属性 (COLOR・UV2・UV3) に移した
  (シェーダの `_Color`・`_ScreenColor`・`_LitMode` は無くなった)。裏面カリングは CPU なので状態の違いにしない。描画順か合成モードが
  変わったときだけまとめ直し、表示・色・頂点の変化は変わった要素の範囲だけ書き直す。1 メッシュ 65536 頂点まで。マスク用メッシュは
  マスクになる要素だけ別に持つ。インスペクタに描画の呼び出しの数 (`DrawCalls`) を出す。
  - 効果: 見本はレイヤーごとのテクスチャなので、そのままでは減らない (24 → 24)。`ugoe build --atlas` で 1 枚のアトラスにした
    `avatar.ugoe` は 24 → 2、`dress-up.ugoe` は 14 → 2 (実際に `ugoe build --atlas` した avatar で確かめ、テストではテクスチャを 0 に
    揃えて真似る)。法線マップのあるテクスチャ (`avatar-lit.ugoe`) はアトラスにまとめられないので減らない。
- **リリースの自動化:** `bindings/unity/release.mjs` (`check [<tag>]`・`sync`・`pack <dir> [--allow-missing]`。web の release.mjs と同じ形)。
  `pack` は `npm pack` で UPM の tarball (`com.ugoe.unity-<版>.tgz`、中は `package/`) と SHA-256 を作り、全プラットフォームの
  ライブラリと `.meta`・見本のモデルが入っていなければ失敗する。`unity.yml` に `v*` のタグを足し、`package` のジョブが
  版を確かめて tarball を成果物 `com.ugoe.unity-tarball` に残す (公開はしない)。`build.mjs --plugins` は見本のモデルも写すようにし、
  `PLUGINS` と `pkg` を export した (`import.meta.main` で直接実行のときだけ動く)。`build.mjs --check` が `release.test.mjs` (node --test) も回す。
  tarball は zip ではなく UPM が直接入れられる tgz にした。
- **色空間:** Gamma を要求する。README の「色空間」に理由と Linear でずれる部分を書き、Linear のプロジェクトでは読み込み時に 1 度だけ
  警告し、インスペクタにも出す。Linear で一致させるには中間のレンダーテクスチャに Gamma のまま合成してから写す必要があり、
  パイプラインごとの組み込みが要るので入れていない。
- 文書: `bindings/unity/README.md` (使い方・描画のしかた・色空間・リリース)、パッケージの README、`docs/release.md` (表と版を揃える手順)、
  `docs/portability.md` (配布の Unity の行)。

### 確かめたこと

- `just ci`、`just unity-check` (.meta 45 件・dotnet test 32 件・スタブでの型検査・node --test 3 件)、`actionlint -shellcheck=` (1.7.12。
  shellcheck は asdf の shim が壊れていて使えなかった)、`release.mjs pack` を手元で (Linux の分だけ `--allow-missing`) 実行して中身を見た。

### 確かめていないこと (利用者の確認に残す)

- **Unity の上では動かしていない。** 書き換えたシェーダ (頂点属性で受ける) のコンパイル、まとめた描画がまとめない場合 (`batching` を
  切る) と画素で一致すること、`Mesh.colors` と UV3 の頂点属性が各グラフィックス API で渡ること、ほか T0405 のメモの一覧
  (シーンと .meta・マスクの上下の向き・Built-in と URP・Android と iOS)。確かめ方は bindings/unity/README.md#制限と確かめていないこと。
- `unity.yml` の tarball のジョブは GitHub Actions の上で実行していない (タグも打っていない)。tarball を Unity に入れてもいない。

### 範囲外の変更

- なし (`justfile` も変えていない。リリースは `node bindings/unity/release.mjs` を直接呼ぶ)。

### 後続の候補

- Linear の色空間で参照と一致させる (中間のレンダーテクスチャ。Built-in と URP の組み込み)。
- 見本にアトラスにしたモデルを入れる (`build.mjs` が `ugoe build --atlas` を呼ぶと ugoe-cli のビルドが要るので入れていない)。
- 法線マップのあるテクスチャのアトラス化 (ugoe-editor-core。いまは avatar-lit でまとめが効かない)。
- マスクの内容のアトラス化 (内容ごとの RenderTexture を 1 枚にまとめると、違う内容を読む要素もまとめられる)。
- GitHub の Release に tarball を添付するところまでの自動化 (公開の判断は利用者)。
