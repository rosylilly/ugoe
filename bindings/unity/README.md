# ugoe for Unity (UPM パッケージ)

`.ugoe` のモデルを Unity (2022.3 以降) で表示し、動かすためのパッケージ。評価は C ABI (`ugoe-ffi`) で行い、
描画は Unity のメッシュとマテリアルで行う ([docs/portability.md](../../docs/portability.md) の段階 B)。

**ライセンス:** MIT。パッケージの依存は `ugoe-ffi` (MIT) だけ。Unity の参照アセンブリ (`UnityEngine.dll` など) は
リポジトリに入れない (Unity の部分の型検査は、API の形だけを手で書いたスタブ `tests/UnityCompile/UnityStubs.cs` で行う)。

検査 (`tests/`) だけが NuGet のパッケージに依存する。配布物 (`com.ugoe.unity/`) には入らない。

| NuGet のパッケージ | 版 | ライセンス |
| --- | --- | --- |
| NUnit | 4.2.2 | MIT |
| NUnit3TestAdapter | 4.6.0 | MIT |
| Microsoft.NET.Test.Sdk | 17.12.0 | MIT |
| (推移的) Microsoft.TestPlatform.ObjectModel・Microsoft.TestPlatform.TestHost・Microsoft.CodeCoverage | 17.12.0 | MIT |
| (推移的) Newtonsoft.Json | 13.0.1 | MIT |
| (推移的) System.Reflection.Metadata | 1.6.0 | MIT (dotnet/corefx の LICENSE) |

`scripts/check-licenses.mjs` (`just licenses`) は Cargo と npm の依存だけを見て、NuGet は見ない。NuGet の依存を足すときは、
ライセンスが MIT と両立する (寛容な) ものかを手で確かめ、この表に書く。

## 構成

| パス | 内容 |
| --- | --- |
| `com.ugoe.unity/` | UPM パッケージ (`package.json` のあるところ) |
| `com.ugoe.unity/Runtime/Core/` | **Unity に依存しない** C# の層: P/Invoke (`Native.cs`)、`Puppet`・`Module`・`LightingUniform`・`TrackingInput`、描画の準備の計算 (乗算済みアルファ・裏面カリング・マスクの内容の計画・変更フラグを見た取り出し・描画のまとめ・マスクのページと中間のテクスチャの配置・色空間の変換) |
| `com.ugoe.unity/Runtime/Unity/` | Unity の部分: `UgoePuppet` (コンポーネント)、`UgoeModel` / `UgoeModule` (アセット)、テクスチャのデコード、マスクの描画、Linear の色空間の中間のテクスチャ (`UgoeIntermediate`) |
| `com.ugoe.unity/Runtime/Resources/Ugoe/` | シェーダ (`UgoeDrawable.shader`・`UgoeMask.shader`・`UgoeComposite.shader`・`UgoeCommon.hlsl`) |
| `com.ugoe.unity/Runtime/Plugins/` | ネイティブライブラリ (**コミットしない**。`build.mjs` か CI の成果物で置く) |
| `com.ugoe.unity/Editor/` | `.ugoe` / `.ugoemod` の取り込み (ScriptedImporter) とインスペクタ |
| `com.ugoe.unity/Samples~/Basic/` | 見本のシーン (`Basic.unity`) とスクリプト。`Models/` は `build.mjs` が置く (コミットしない) |
| `tests/` | `Runtime/Core` の dotnet のテスト (ホストの `libugoe_ffi` と `assets/samples/` のモデルで実行) |
| `tests/UnityCompile/` | Unity に依存する C# を、Unity の API の形だけを書いたスタブに対して型検査する (実行はしない) |
| `build.mjs` | ビルド・配置・検査 (下) |
| `release.mjs` | 版の照合と UPM の tarball ([リリース](#リリース)) |

## ビルドと検査

リポジトリのルートで (Unity は要らない):

```sh
just unity-build                           # = node bindings/unity/build.mjs
just unity-check                           # = node bindings/unity/build.mjs --check
node bindings/unity/build.mjs              # ホスト向けの ugoe-ffi をビルドして Runtime/Plugins/<OS>/ に置き、見本のモデルを Samples~/Basic/Models/ に写す
node bindings/unity/build.mjs --release    # リリースビルド
node bindings/unity/build.mjs --check      # 上に加えて、.meta の検査・dotnet test (tests/)・Unity の部分の型検査 (tests/UnityCompile)・release.mjs のテスト
node bindings/unity/build.mjs --metas      # 足りない .meta を作る (ファイルを足したら実行してコミットする)
node bindings/unity/build.mjs --plugins <dir>   # CI の成果物のライブラリを Runtime/Plugins/ に並べ、見本のモデルを写す (ビルドしない)
```

- `--check` には .NET SDK (8 以降) が要る (無ければ入れ方を示して止まる)。これらは `just ci` に含まれない。
  CI では `.github/workflows/unity.yml` が、`check` (= `just unity-check`)、`native` (下の表の各プラットフォームの
  リリースビルド)、`package` (ライブラリを `Runtime/Plugins/` に並べた `com.ugoe.unity` と、その UPM の tarball を成果物に残す。
  公開はしない。[リリース](#リリース)) を実行する。
- `.meta` の GUID はパッケージの中のパスから決める (作り直しても変わらない)。Unity は `.meta` の無いファイルを
  不変のパッケージの中では無視するので、ファイルを足したら `--metas` を実行してコミットする。`--check` が足りない
  `.meta`・残った `.meta`・GUID の重複を検出する。
- ネイティブライブラリの置き場所とプラグインの設定 (`.meta` の `PluginImporter`) は `build.mjs` の `PLUGINS`:

| プラットフォーム | ファイル | ビルド |
| --- | --- | --- |
| Linux x86_64 (エディタと Standalone) | `Plugins/Linux/x86_64/libugoe_ffi.so` | `cargo build -p ugoe-ffi --release` |
| Windows x86_64 (エディタと Standalone) | `Plugins/Windows/x86_64/ugoe_ffi.dll` | 同上 (MSVC) |
| macOS (エディタと Standalone、universal) | `Plugins/macOS/libugoe_ffi.dylib` | `aarch64-apple-darwin` と `x86_64-apple-darwin` を `lipo -create` |
| Android arm64-v8a / armeabi-v7a / x86_64 | `Plugins/Android/<ABI>/libugoe_ffi.so` | `cargo ndk -t <ABI> build -p ugoe-ffi --release` |
| iOS (arm64) | `Plugins/iOS/libugoe_ffi.a` | `cargo build -p ugoe-ffi --release --target aarch64-apple-ios` (静的ライブラリ。`__Internal` で呼ぶ) |

WebGL は対象外 (Emscripten 向けの `ugoe-ffi` のビルドがまだ無い。Web では `bindings/web` を使う)。

## Unity のプロジェクトに入れる

1. ネイティブライブラリを置く: 手元の OS なら `node bindings/unity/build.mjs --release`。ほかのプラットフォームは CI の成果物を
   `--plugins` で並べる。
2. Unity の Package Manager で **Add package from disk...** を選び、`bindings/unity/com.ugoe.unity/package.json` を開く
   (または `Packages/manifest.json` に `"com.ugoe.unity": "file:<パス>/bindings/unity/com.ugoe.unity"`)。
   Git の URL (`...ugoe.git?path=bindings/unity/com.ugoe.unity`) でも入るが、ネイティブライブラリが入っていないので、
   別に置く必要がある。リリースの tarball (`com.ugoe.unity-<版>.tgz`。[リリース](#リリース)) なら、**Add package from tarball...**
   ですべてのプラットフォームのライブラリと見本のモデルごと入る。
   - 色空間は Gamma でも Linear でもよい。Linear では中間のテクスチャに合成してから写し、参照実装と同じ色にする
     ([色空間](#色空間))。
3. Package Manager の ugoe の **Samples** から **Basic** を取り込み、`Basic.unity` を開いて再生する。見本のモデルは
   `build.mjs` が `Samples~/Basic/Models/` に置いたもの (取り込んだ後に `.ugoe` を `Models/` に足してもよい)。
   - 見本は、モデルの切り替え・パラメータのスライダー・表情とモーションのボタン・着せ替えのチェックボックス (`dress-up`)、
     ライティングのモデル (`avatar-lit`) ではマウスの位置の点光源、クリックした位置の当たり判定のログを出す。
   - 見本は `UnityEngine.Input` (旧入力) を使う。新しい Input System だけのプロジェクトでは、Player Settings の
     Active Input Handling を Both にする。

## 使い方

```csharp
using Ugoe;
using Ugoe.Unity;

var puppet = gameObject.AddComponent<UgoePuppet>();
puppet.Model = model;                         // UgoeModel (.ugoe を取り込んだもの)
puppet.SetParameter("AngleX", 15);            // 次の LateUpdate で評価・描画する
puppet.StartMotion("idle", MotionPriority.Idle);
puppet.SetExpression("smile");
puppet.ShowPose("ArmB");                      // ポーズの表示の選択 (パーツの ID。フェードで切り替わる)
puppet.Puppet.SetEffectsJson(effectsJson);    // 目パチ・呼吸・リップシンクの設定の差し替え (モデルの effects と同じ JSON)
var arm = puppet.Puppet.GetPartOpacity(puppet.Puppet.FindPart("ArmB")); // 直前の評価のパーツの不透明度の上書き (Value, Weight)
var shown = puppet.Puppet.GetPartEvaluatedOpacity(puppet.Puppet.FindPart("ArmB")); // 祖先を掛けた評価後の不透明度
puppet.Puppet.SetLipSyncLevel(0.8f);          // Puppet (Ugoe.Puppet) は C ABI の薄い包み。直接使ってよい
puppet.MotionEventFired += e => Debug.Log($"{e.Value} at {e.Time}");
puppet.Puppet.ResetPhysics();                 // SetPhysicsJson / PhysicsJson で物理の設定の差し替えと読み出し

// 着せ替え (docs/spec/modules.md#puppet-での着せ替え)。付けられなければエラーを出して false
puppet.AttachModule(sailor);                  // UgoeModule (.ugoemod)
puppet.DetachModule("Sailor");

// ライティング (spec/rendering.md の Lighting。座標はキャンバス座標 + 奥行き)
if (puppet.Puppet.UsesLighting)
{
    puppet.AmbientLight = new Color(0.4f, 0.4f, 0.45f);
    var p = puppet.WorldToCanvas(mouseWorld);
    puppet.SetLights(new[]
    {
        new UgoeLight { kind = LightKind.Directional, vector = new Vector3(1, 1, -1), color = Color.white },
        new UgoeLight { kind = LightKind.Point, vector = new Vector3(p.x, p.y, 250), color = Color.white, range = 900 },
    });
}

// トラッキング (docs/spec/avatar-profile.md#フェイストラッキングとの対応)。平滑化はホストが行う
var input = new TrackingInput { HeadYaw = yaw, HeadPitch = pitch, HeadRoll = roll };
input.SetBlendShape("jawOpen", 0.4f);         // ARKit の名前 (MediaPipe の名前も可)
puppet.Puppet.SetTracking(input);
puppet.Puppet.SetStageEnabled(UpdateStage.EyeBlink, false);

// 当たり判定
foreach (var id in puppet.HitAreasAt(worldPoint)) Debug.Log(id);
```

| `UgoePuppet` のメンバー | 内容 |
| --- | --- |
| `Model` | 表示するモデル。代入すると読み直す |
| `playing` / `speed` / `playInEditor` / `Advance(dt)` | 時間を進めるか・速さ・エディタで再生していない間も進めるか・手で進める |
| `PixelsPerUnit` | 1 ユニットあたりのキャンバスのピクセル数 (既定 100)。キャンバスの原点がこのオブジェクトの原点に来る |
| `batching` / `DrawCalls` | 描画をまとめるか (既定 true。[描画のしかた](#描画のしかた)) / 直前のフレームの描画の呼び出しの数 (インスペクタにも出る) |
| `maskScale` / `maskCamera` | マスクと中間のテクスチャの解像度の倍率 (1 で画面上の大きさ) / 画面上の大きさを測るカメラ (空なら `Camera.main`) |
| `linearCompositing` / `UsesIntermediate` | Linear の色空間での描き方 (`Intermediate` (既定) か `Direct`。[色空間](#色空間)) / 中間のテクスチャを使っているか |
| `AmbientLight` / `SetLights` / `Lights` | 環境光 / 光源 (先頭の 8 個まで)。シーンに保存される |
| `SetParameter` / `GetParameter` / `SetSavedParameter` | パラメータ (`SetSavedParameter` はシーンに保存する値。インスペクタのスライダーが使う) |
| `StartMotion` / `SetExpression` / `ShowPose` | モーション・表情・ポーズの表示の選択 |
| `AttachModule` / `DetachModule` / `SetModules` | 着せ替え (付けたモジュールはシーンに保存される) |
| `WorldToCanvas` / `CanvasToWorld` / `HitAreasAt` | 座標の変換・当たり判定 |
| `Follow` | 受け手として、指揮役から受けた値で進めて描き直す (`playing` を false にして毎フレーム呼ぶ。[受け手](#受け手)) |
| `Puppet` | `Ugoe.Puppet` (パラメータの一覧、`SetParameters`、`SetTracking`・`MotionOverTracking`・`SetTrackingKept`、`HitTest`、パーツの一覧と不透明度 (`PartIds`・`FindPart`・`GetPartOpacity`・`GetPartEvaluatedOpacity`)、効果の差し替え (`SetEffectsJson`)、物理の設定 (`SetPhysicsJson`・`PhysicsJson`・`SetBasePhysicsJson`・`BasePhysicsJson`・`ResetPhysics`)、受け手 (`SetBaseValues`・`Follow`)、読み込みの警告など) |
| イベント `ModelLoaded` / `MotionEventFired` | 読み込んだとき / モーションのイベントが発火したとき (評価の直後) |

`Ugoe.Puppet` などの `Runtime/Core` の型は Unity に依存しないので、Unity の外 (dotnet のサーバなど) でも使える。
失敗は `UgoeException` (`Result`・着せ替えでは `Kind` と `Fields`。種類は crates/ugoe-ffi/README.md と同じ)。

### 受け手

指揮役 (スタジオなど、別の場所で評価するもの) の値で動かす受け手 ([docs/spec/motion.md#受け手として進める](../../docs/spec/motion.md#受け手として進める)) は、`playing` を false にして
毎フレーム `UgoePuppet.Follow(values, lipSyncLevel, lipSyncStrength, partOpacities, dt)` を呼ぶ。値は指揮役の
`ugoe::Puppet::values_before_physics`・`lip_sync_applied`・`part_opacities_applied` (C ABI・wasm には指揮役の側の取り出しが無いので、
指揮役から受け取る)。物理・リップシンク・パーツの不透明度・評価だけをこの Puppet で行う。

### C ABI との対応

`Native.cs` は `crates/ugoe-ffi/include/ugoe.h` のすべての関数を宣言する。ただし `ugoe_puppet_positions` (内部のバッファの借用) だけは
宣言しない。C# では借りても配列に写すことになり、`ugoe_puppet_copy_positions` (`Puppet.CopyPositions`) と同じだから。

## 描画のしかた

- **描画をまとめる** ([docs/spec/rendering.md#描画の最適化](../../docs/spec/rendering.md#描画の最適化) の条件どおり)。描画順で
  隣り合い、テクスチャ・合成モード・読むマスクのテクスチャ (ページ。下) が同じ要素を 1 つのメッシュにし (`DrawBatchPlan`、`Runtime/Core/DrawBatches.cs`)、
  まとめたものごとに子の GameObject (`MeshRenderer`、保存しない) を作る。マスクを読まない要素は、どの要素ともまとめる。
  描画順は決して入れ替えない (メッシュの中の三角形を描画順に並べる)。描画順は `sortingOrder` で決め、ルートの
  `SortingGroup` (自動で付く) の中で並ぶので、ほかの 2D の物とは 1 つのまとまりとして並ぶ。
  - 要素ごとの値 (乗算色と不透明度 = 頂点色、スクリーン色 = UV2、mask_strength・ライティングの種類・通常マスクの結合の係数 = UV3、
    読むマスクの内容のページの中の位置 = UV4) は頂点属性で渡す。テクスチャ・法線マップ・マスクのページはまとめたものごと (`MaterialPropertyBlock`)。
  - 描画順か合成モードが変わったらまとめ直す (子の GameObject は使い回す)。表示・不透明度・色・頂点が変わっただけではまとめ直さない
    (見えない要素は三角形を出さない)。1 つのメッシュは 65536 頂点まで (16 ビットの頂点番号)。
  - **効くのはテクスチャをアトラスにまとめたモデル** (`ugoe build --atlas`、[docs/spec/file-formats.md#テクスチャのアトラス化](../../docs/spec/file-formats.md#テクスチャのアトラス化))。
    見本の `avatar.ugoe` (24 要素、レイヤーごとのテクスチャ) はそのままでは 24 回のままだが、それをアトラスにした同梱の見本
    `avatar-atlas.ugoe` (`ugoe sample avatar --atlas`) は 1 回になる (`dress-up.ugoe` もアトラスにすると 14 回 → 1 回。`tests/BatchTests.cs`)。
    法線マップのあるテクスチャ (`avatar-lit.ugoe`) はアトラスにまとめられない (法線マップを同じ配置で詰める必要があり、ugoe-editor-core の
    アトラス化がまだ対応していない)。`MaterialPropertyBlock` を使うので、Unity の動的バッチングと SRP Batcher はもともと効かない。
  - `batching` を false にすると要素ごとに描く (比べるとき・調べるとき)。
- 毎フレーム LateUpdate で評価し (`ugoe_puppet_update`)、**変更フラグ** (`UGOE_CHANGED_*`・`render_order_changed`) の立った
  要素の分だけ、まとめたメッシュの頂点・頂点の色・三角形 (表示とカリング) を送り直す (`FrameSync`)。
  マスクも、関わる要素の頂点かマスクの効き具合が変わったときだけ描き直す。
- テクスチャは読み込み時に乗算済みアルファ (参照実装と同じ `(c × a + 127) div 255`) にし、バイリニア・端で止める・
  ミップマップなし・sRGB のデコードなし (`linear: true`) で作る。法線マップはアルファを掛けない。KTX2 は白で代用して警告する。
- シェーダ (`UgoeDrawable.shader`) は spec/rendering.md の式 (乗算色・スクリーン色・ライティング・不透明度・マスク) を
  参照シェーダ (`crates/ugoe-render/src/shaders/drawable.wgsl`) と同じ形で書いた。合成モードは材質の合成の設定で表す
  (normal = `One, OneMinusSrcAlpha`、additive = 色 `One, One`・アルファ `Zero, One`、multiply = 色 `DstColor, OneMinusSrcAlpha`・
  アルファ `Zero, One`)。法線は画面での微分から求めたヤコビ行列で回す。
- 裏面カリングは CPU で表を向く三角形を選ぶ (向きはキャンバス座標だけで決まり、Unity の Y 上向きへの反転に左右されないため)。
  マスクとしては裏向きの三角形も使う (マスク用メッシュは、ほかの要素のマスクになる要素だけ別に持つ)。
- マスクは、同じマスクの組を読む要素でまとめた「内容」ごとに、それを読む要素のキャンバス座標の外接矩形を写して描く
  (`UgoeMaskRenderer`)。内容は入れ子の深さごとに 1 枚の `RenderTexture` (ページ) に並べ (`MaskAtlasLayout`、`Runtime/Core/Offscreen.cs`)、
  要素は頂点属性で受けた写像で自分のキャンバス座標からページの位置を求めて読む。違う内容を読む要素も、同じページなら 1 回の描画にまとまる。
  R に通常マスク (和集合なら `Π(1 − c)`、共通部分なら作業用のテクスチャを使って `Π c`)、G に反転マスクの `Π(1 − c)` を乗算合成で描き、
  `k = 1 − s × (1 − m)` にする。入れ子は浅いページを読んで深いページに描くので、書いているテクスチャを読むことはなく、同じフレームで
  描くので遅れない。マスクの数と入れ子の深さに上限はない。ページの 1 辺は 4096 画素まで (収まらなければそのページの解像度を下げる)。

## 制限と確かめていないこと

- **Unity の上では実行していない** (この環境に Unity が無い)。確かめたのは `Runtime/Core` の dotnet のテストと、
  Unity の部分のスタブでの型検査だけ。Unity で確かめること:
  1. `node bindings/unity/build.mjs` の後、Unity 2022.3 (と Unity 6) の新しいプロジェクトにパッケージを入れ、コンパイルが通るか。
  2. `.ugoe` / `.ugoemod` がアセットとして取り込まれるか (インスペクタで `UgoePuppet.Model` に選べるか)。
  3. 見本の `Basic.unity` が開けて (手で書いたシーンのファイル)、再生でモデルが表示され、`AngleX` が揺れるか。
  4. `avatar.ugoe` の見た目が `ugoe render` の画像と合うか (マスク・合成モード・乗算色とスクリーン色)。とくに
     マスクの内容のテクスチャの上下の向き (`CommandBuffer.SetViewProjectionMatrices` のプラットフォームの差を Unity が
     吸収する前提) を Direct3D・Metal・Vulkan・OpenGL で。
  5. `avatar-lit.ugoe` の陰影、`dress-up.ugoe` の着せ替え、Built-in・URP の両方のパイプラインでの表示。
  6. Android・iOS のビルドでネイティブライブラリが読まれるか (iOS は `__Internal` の静的ライブラリ)。
- Linear の色空間の中間のテクスチャ ([色空間](#色空間)) は、C# の層の計算 (配置と色の変換。`tests/OffscreenTests.cs`) だけを
  確かめた。Unity の上で、Gamma のプロジェクトと Linear のプロジェクトの画素が (輪郭を除いて) 一致すること、写す四角形の上下の向き
  (マスクと同じ前提)、Built-in・URP の両方で表示されることを確かめる。
- 描画をまとめた見た目 (まとめない場合と画素が一致すること) は Unity で比べていない (`batching` の切り替えで比べられる)。
- 1 つの `UgoePuppet` は 1 つの解像度でマスクを描く (複数のカメラで大きさが大きく違うと、小さい方に合わない)。
- 見本は旧入力 (`UnityEngine.Input`) を使う。

## 色空間

ugoe の色は sRGB の値のまま計算して合成する約束 (spec/rendering.md) なので、**Gamma の色空間ではそのまま画面に描けば参照実装と一致する**。

**Linear の色空間** (`UgoePuppet.linearCompositing` が既定の `Intermediate`) では、中間のテクスチャを使って一致させる (`UgoeIntermediate`):

1. まとめたメッシュを、sRGB の変換をしない `RenderTexture` (`RenderTextureReadWrite.Linear`) に、sRGB の値のまま描く
   (シェーダのキーワード `UGOE_GAMMA_TARGET`)。合成も sRGB の値で行われるので、参照実装で透明な背景に描いた結果と同じになる。
   テクスチャはモデルの外接矩形を写し、原点を画素の格子にそろえる (解像度の倍率が 1 なら `ugoe render` のキャンバスの画素と重なる)。
   解像度はマスクと同じく画面上の大きさに合わせる (`maskScale`)。
2. それを 1 枚の四角形 (子の GameObject `ugoe intermediate`) で画面に写す (`Hidden/Ugoe/Composite`)。ストレートの色を
   IEC 61966-2-1 の式で線形に直してアルファを掛け直すので、不透明な画素は書き込み先の変換 (線形 → sRGB) で中間のテクスチャと同じ値に戻る
   (UnityCG の `GammaToLinearSpace` は近似なので使わない)。

モデルの中の合成 (重なった半透明の要素・加算と乗算・マスク・アンチエイリアス) は参照実装と一致する。シーンとの合成 (モデルの輪郭の
半透明の画素) だけは、ほかのスプライトと同じく線形で行われる。描画パイプライン (Built-in・URP) には依存しない (`Graphics.ExecuteCommandBuffer`
で LateUpdate に描く)。四角形 1 枚の描画とテクスチャ 1 枚が増え、中間のテクスチャを写すときにバイリニアで標本化し直す。

`linearCompositing` を `Direct` にすると画面に直接描く (シェーダが出力を線形に直して書く)。不透明な部分は同じ色になるが、半透明の部分の
合成が線形で行われ、参照と少しずれる (読み込んだときに 1 度だけ警告を出し、インスペクタにも出す)。

## リリース

版の正はルートの `Cargo.toml` の `[workspace.package] version` で、`com.ugoe.unity/package.json` の版をそれに合わせる
([docs/release.md](../../docs/release.md))。公開する (配る場所に置く) かどうかは利用者が決めて行う。

```sh
node bindings/unity/release.mjs sync               # package.json の版をクレートの版にする
node bindings/unity/release.mjs check [v<版>]      # 揃っているか (タグを渡すとタグとも比べる)
node bindings/unity/build.mjs --plugins <dir>      # 各プラットフォームのライブラリ (CI の成果物) と見本のモデルを並べる
node bindings/unity/release.mjs pack dist          # dist/com.ugoe.unity-<版>.tgz と .sha256 を作る
```

- `pack` は `npm pack` で、中身を `package/` の下に置いた tarball を作る (UPM の tarball の形。Package Manager の
  **Add package from tarball...** か、`manifest.json` の `"com.ugoe.unity": "file:<パス>/com.ugoe.unity-<版>.tgz"` で入る)。
  `build.mjs` の `PLUGINS` のすべてのライブラリと `.meta`、見本のモデルが入っていなければ失敗する
  (`--allow-missing` で、手元の OS の分だけの tarball も作れる)。
- **CI:** `v<版>` のタグを push すると `.github/workflows/unity.yml` が動き、`package` のジョブが版を確かめてから tarball を作り、
  成果物 `com.ugoe.unity-tarball` (tarball と SHA-256) に残す。ブランチでも同じ tarball を作る (版の照合はタグと比べない)。
- **公開は手で行う:** 成果物をダウンロードし、GitHub の Release に添付する (利用者は tarball をダウンロードして入れる)。
  スコープ付きのレジストリ (OpenUPM や自前の npm レジストリ) に置くなら、tarball をそのまま `npm publish <tgz> --registry <URL>`
  できる。OpenUPM は Git のタグからパッケージを作るので、ネイティブライブラリをコミットしないこのリポジトリのままでは使えない。

## 関連タスク

- T0405 (`docs/tasks/T0405-unity-package.md`)、T0419 (C# の API の追加・描画のまとめ・リリース・色空間)、T0420 (Linear の中間のテクスチャ・マスクのページ・アトラスにした見本)、C ABI は T0401・T0409・T0412・T0414
