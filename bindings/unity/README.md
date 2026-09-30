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
| `com.ugoe.unity/Runtime/Core/` | **Unity に依存しない** C# の層: P/Invoke (`Native.cs`)、`Puppet`・`Module`・`LightingUniform`・`TrackingInput`、描画の準備の計算 (乗算済みアルファ・裏面カリング・マスクの内容の計画・変更フラグを見た取り出し) |
| `com.ugoe.unity/Runtime/Unity/` | Unity の部分: `UgoePuppet` (コンポーネント)、`UgoeModel` / `UgoeModule` (アセット)、テクスチャのデコード、マスクの描画 |
| `com.ugoe.unity/Runtime/Resources/Ugoe/` | シェーダ (`UgoeDrawable.shader`・`UgoeMask.shader`・`UgoeCommon.hlsl`) |
| `com.ugoe.unity/Runtime/Plugins/` | ネイティブライブラリ (**コミットしない**。`build.mjs` か CI の成果物で置く) |
| `com.ugoe.unity/Editor/` | `.ugoe` / `.ugoemod` の取り込み (ScriptedImporter) とインスペクタ |
| `com.ugoe.unity/Samples~/Basic/` | 見本のシーン (`Basic.unity`) とスクリプト。`Models/` は `build.mjs` が置く (コミットしない) |
| `tests/` | `Runtime/Core` の dotnet のテスト (ホストの `libugoe_ffi` と `assets/samples/` のモデルで実行) |
| `tests/UnityCompile/` | Unity に依存する C# を、Unity の API の形だけを書いたスタブに対して型検査する (実行はしない) |
| `build.mjs` | ビルド・配置・検査 (下) |

## ビルドと検査

リポジトリのルートで (Unity は要らない):

```sh
just unity-build                           # = node bindings/unity/build.mjs
just unity-check                           # = node bindings/unity/build.mjs --check
node bindings/unity/build.mjs              # ホスト向けの ugoe-ffi をビルドして Runtime/Plugins/<OS>/ に置き、見本のモデルを Samples~/Basic/Models/ に写す
node bindings/unity/build.mjs --release    # リリースビルド
node bindings/unity/build.mjs --check      # 上に加えて、.meta の検査・dotnet test (tests/)・Unity の部分の型検査 (tests/UnityCompile)
node bindings/unity/build.mjs --metas      # 足りない .meta を作る (ファイルを足したら実行してコミットする)
node bindings/unity/build.mjs --plugins <dir>   # CI の成果物のライブラリを Runtime/Plugins/ に並べる (ビルドしない)
```

- `--check` には .NET SDK (8 以降) が要る (無ければ入れ方を示して止まる)。これらは `just ci` に含まれない。
  CI では `.github/workflows/unity.yml` が、`check` (= `just unity-check`)、`native` (下の表の各プラットフォームの
  リリースビルド)、`package` (ライブラリを `Runtime/Plugins/` に並べた `com.ugoe.unity` を成果物に残す。公開はしない) を実行する。
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
   別に置く必要がある。
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
| `maskScale` / `maskCamera` | マスクの解像度の倍率 (1 で画面上の大きさ) / 画面上の大きさを測るカメラ (空なら `Camera.main`) |
| `AmbientLight` / `SetLights` / `Lights` | 環境光 / 光源 (先頭の 8 個まで)。シーンに保存される |
| `SetParameter` / `GetParameter` / `SetSavedParameter` | パラメータ (`SetSavedParameter` はシーンに保存する値。インスペクタのスライダーが使う) |
| `StartMotion` / `SetExpression` | モーション・表情 |
| `AttachModule` / `DetachModule` / `SetModules` | 着せ替え (付けたモジュールはシーンに保存される) |
| `WorldToCanvas` / `CanvasToWorld` / `HitAreasAt` | 座標の変換・当たり判定 |
| `Puppet` | `Ugoe.Puppet` (パラメータの一覧、`SetParameters`、`SetTracking`、`HitTest`、読み込みの警告など) |
| イベント `ModelLoaded` / `MotionEventFired` | 読み込んだとき / モーションのイベントが発火したとき (評価の直後) |

`Ugoe.Puppet` などの `Runtime/Core` の型は Unity に依存しないので、Unity の外 (dotnet のサーバなど) でも使える。
失敗は `UgoeException` (`Result`・着せ替えでは `Kind` と `Fields`。種類は crates/ugoe-ffi/README.md と同じ)。

## 描画のしかた

- 描画可能要素ごとに子の GameObject (`MeshRenderer`、保存しない) を作る。描画順は `sortingOrder` で決め、ルートの
  `SortingGroup` (自動で付く) の中で並ぶので、ほかの 2D の物とは 1 つのまとまりとして並ぶ。
- 毎フレーム LateUpdate で評価し (`ugoe_puppet_update`)、**変更フラグ** (`UGOE_CHANGED_*`・`render_order_changed`) の立った
  要素だけ、頂点 (`Mesh.vertices`)・色 (`MaterialPropertyBlock`)・表示・描画順を送り直す (`FrameSync`)。
  マスクも、関わる要素の頂点かマスクの効き具合が変わったときだけ描き直す。
- テクスチャは読み込み時に乗算済みアルファ (参照実装と同じ `(c × a + 127) div 255`) にし、バイリニア・端で止める・
  ミップマップなし・sRGB のデコードなし (`linear: true`) で作る。法線マップはアルファを掛けない。KTX2 は白で代用して警告する。
- シェーダ (`UgoeDrawable.shader`) は spec/rendering.md の式 (乗算色・スクリーン色・ライティング・不透明度・マスク) を
  参照シェーダ (`crates/ugoe-render/src/shaders/drawable.wgsl`) と同じ形で書いた。合成モードは材質の合成の設定で表す
  (normal = `One, OneMinusSrcAlpha`、additive = 色 `One, One`・アルファ `Zero, One`、multiply = 色 `DstColor, OneMinusSrcAlpha`・
  アルファ `Zero, One`)。法線は画面での微分から求めたヤコビ行列で回す。
- 裏面カリングは CPU で表を向く三角形を選ぶ (向きはキャンバス座標だけで決まり、Unity の Y 上向きへの反転に左右されないため)。
  マスクとしては裏向きの三角形も使う (サブメッシュ 1)。
- マスクは、同じマスクの組を読む要素でまとめた「内容」ごとに `RenderTexture` を作り、それを読む要素のキャンバス座標の外接矩形を
  写して描く (`UgoeMaskRenderer`)。R に通常マスク (和集合なら `Π(1 − c)`、共通部分なら作業用のテクスチャを使って `Π c`)、
  G に反転マスクの `Π(1 − c)` を乗算合成で描き、要素のシェーダが自分のキャンバス座標で読んで `k = 1 − s × (1 − m)` にする。
  入れ子は深さの順に同じフレームで描くので遅れない。マスクの数と入れ子の深さに上限はない。1 辺は 4096 画素まで
  (超えると解像度を下げる)。

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
- 色空間: ugoe の色は sRGB の値のまま計算する約束なので、**プロジェクトの色空間を Gamma にすると** 参照と一致する。
  Linear では、出力をストレートの色に戻してから線形に直して書くが、合成が線形で行われるので半透明の部分が少しずれる。
- 描画可能要素ごとに `MeshRenderer` を持つ (バッチングはしない)。要素が多いモデルでは描画の呼び出しが多い。
- 1 つの `UgoePuppet` は 1 つの解像度でマスクを描く (複数のカメラで大きさが大きく違うと、小さい方に合わない)。
- パーツの不透明度・ポーズの表示の選択・効果の差し替えは C ABI に無い (T0414 で足す予定) ので出していない。
- 見本は旧入力 (`UnityEngine.Input`) を使う。

## 関連タスク

- T0405 (`docs/tasks/T0405-unity-package.md`)、C ABI は T0401・T0409
