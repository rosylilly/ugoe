---
id: T0426
title: コードレビュー: bindings/unity (UPM パッケージ)
status: done
milestone: M9
crates: [bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0426
---

# T0426: コードレビュー: bindings/unity (UPM パッケージ)

## 目的

bindings/unity (UPM パッケージ) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `bindings/unity` のすべて (com.ugoe.unity の C#・テスト・build.mjs・release.mjs)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - C ABI の宣言 (DllImport) と `ugoe-ffi` のヘッダの一致、ハンドルの解放 (SafeHandle・Dispose・ファイナライザ)
  - マーシャリング (文字列の符号化・配列の長さ・固定)、メインスレッドの前提、毎フレームの GC の確保
  - プラットフォームごとのネイティブライブラリの置き場所と、エディタとビルドでの読み込み
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0443〜T0447 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

範囲は `bindings/unity` のすべて (C#・シェーダ・テスト・スタブ・build.mjs・release.mjs) を読んだ。`Native.cs` の宣言は `include/ugoe.h` と
1 つずつ突き合わせ (構造体の並びと型は一致)、`.github/workflows/unity.yml` の native のジョブも読んだ (直していない)。
パスは `bindings/unity/com.ugoe.unity/` から。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `Runtime/Unity/UgoePuppet.cs` の `AttachModule` | 同じスロットか名前のモジュールの差し替えを、覚えておく並びでは末尾に加えていた。C ABI は同じ位置に入れるので、無効・有効や再読み込みで `SetModules` に渡すと合成の順が変わる | 直した (規則を `ModuleOrder` (`Runtime/Core/Module.cs`) に出し、C ABI の順と比べるテスト `RememberedModuleOrderMatchesTheNativeOrder` を先に足した) |
| 中 | `Runtime/Unity/UgoePuppet.cs` の `Load` | 読めないモジュール (`UgoeModule.Module` が投げる) や、テクスチャ・`Build` の途中の失敗で、作りかけの `Puppet` が残り、LateUpdate が毎フレーム `NullReferenceException` を出す | 直した (モジュールごとに捕まえて飛ばし、後半の失敗では Unload してエラーを 1 回出す)。Unity で動かすテストは無い (T0443・T0446) |
| 中 | `Runtime/Unity/UgoePuppet.cs` の `OnValidate`/`LateUpdate` | インスペクタで `pixelsPerUnit` を変えても、頂点が動いたメッシュだけが新しい大きさになり、動かないメッシュは古い大きさのまま (setter では読み直すが、インスペクタからは読み直さない)。0 以下も入る | 直した (読み直しの判断に加え、OnValidate で正に保つ)。テストは T0446 |
| 中 | `Runtime/Core/Native.cs` | C ABI の 9 つの関数 (受け手の `follow`・`set_base_values`、トラッキングの優先の 3 つ、本体の物理の 2 つ、`part_evaluated_opacity`、`positions`) が宣言されていない (T0430) | 直した (`positions` 以外を宣言し、`Puppet.SetBaseValues`・`Follow`・`MotionOverTracking`・`SetTrackingKept`・`SetBasePhysicsJson`・`BasePhysicsJson`・`GetPartEvaluatedOpacity` と `UgoePuppet.Follow` を足した。`positions` は C# では `copy_positions` と同じになるので範囲外と README に書いた。テスト `tests/ReceiverTests.cs`) |
| 中 | `Runtime/Unity/` 全体 | Unity の上で一度も動かしていない (README の「制限」) | T0443 を起票 |
| 中 | `Runtime/Unity/UgoePuppet.cs` | 状態の扱い (読み直しの判断・送り直す範囲・マスクを描き直す条件) にテストが無い。上の 3 つの不具合はすべてここ | T0446 を起票 |
| 低 | `Runtime/Core/UgoeException.cs` の `Utf8.Encode` | 途中に NUL のある文字列を渡すと C の側で切れ、`FindPart("Root\0Hat")` が `Root` を返す | 直した (`ArgumentException`。テストを先に足した) |
| 低 | `Runtime/Unity/UgoePuppet.cs` の `LateUpdate` | `speed` が負だと負の `dt` を渡し、C ABI が断って毎フレーム例外 | 直した (負の速さは止めるのと同じ) |
| 低 | `Runtime/Unity/UgoePuppet.cs` の `Load` | シーンの値に NaN があると `SetParameter` が投げる | 直した (NaN は飛ばす) |
| 低 | `Runtime/Core/Offscreen.cs` | `Math.Clamp` は .NET Standard 2.1 からで、Unity の API 互換レベル「.NET Framework」では通らない (スタブの型検査は netstandard2.1 だけ) | 直した (自前の clamp)。検査の穴は T0444 を起票 |
| 低 | `Runtime/Unity/UgoeIntermediate.cs`・`UgoePuppet.RenderIntermediate` | Linear の色空間 (既定の Intermediate) で、毎フレーム `List` 2 つ・配列・描画の並びを確保 | 直した (四角形の uv と三角形は作るときに 1 度、頂点と並びは使い回す) |
| 低 | `Runtime/Unity/UgoeMaskRenderer.cs` | マスクのページがデバイスの作り直しで失われても、止まっている間は描き直さない (中間のテクスチャは `Lost` で描き直していた) | 直した (`Lost` を見て描き直す) |
| 低 | `Runtime/Unity/UgoeMaskRenderer.cs` の共通部分 | 作業用のテクスチャが既定の readWrite (Linear の色空間では sRGB) で、ページ (Linear) と量子化が違う | 直した (`RenderTextureReadWrite.Linear`。スタブに多重定義を足した) |
| 低 | `Runtime/Core/Puppet.cs`・`Module.cs` | 作った直後の情報の読み出しが失敗すると、ハンドルがファイナライザまで残る | 直した (失敗したらすぐ破棄) |
| 低 | `Runtime/Core/Puppet.cs` の `MotionEvents` | イベントの無いフレームでも空の配列を確保 (購読があると毎フレーム) | 直した (`Array.Empty`) |
| 低 | `Runtime/Core/Offscreen.cs`・`UgoeMaskRenderer.Render` | マスクを読む要素が動くフレームごとに、配置のための `List` と配列を確保 | T0445 を起票 (公開の `MaskAtlasLayout` の形を決めてから) |
| 低 | `Runtime/Unity/UgoePuppet.MaskResolution` | エディタで止めている間にカメラを寄せても、マスクと中間のテクスチャの解像度は次に描き直すまで変わらない | 見送り (画質だけ。止めている間に毎フレーム測り直す価値が小さい) |
| 低 | `Editor/UgoeImporters.cs` | 違うアーキテクチャのライブラリ (`BadImageFormatException`) は捕まえず、取り込みの例外になる | 見送り (取り込みは失敗として見えるので害は小さい。T0443 で確かめる) |
| 低 | `Samples~/Basic/UgoeSample.cs` | ライティングのモデルで毎フレーム `SetLights` (配列の確保と材質の更新) | 見送り (見本。マウスに付いて来る光源の分かりやすさを優先) |
| 低 | `Runtime/Core/Module.cs` の `EnsureCompatible` | ABI の版の不一致を `InvalidArgument` の `UgoeException` で知らせる (結果の種類が合っていない) | 見送り (公開の `UgoeResult` に値を足すのは API の変更で、文で区別できる) |

- T0430 (T0423 が起票した、Unity から使えない 9 つの C ABI の関数) は **このタスクで済んだ** (上の表の 4 行目)。T0430 は done にしてよい。
  T0423 で `follow` の強さ・パーツの不透明度が有限でなければ `UGOE_ERROR_INVALID_ARGUMENT` になったことも `Puppet.Follow` の doc に書いた。
- 受け手の値 (`values_before_physics` など指揮役の側) は C ABI にも wasm にも無いので、Unity は受け手にだけなれる (README の「受け手」に書いた。
  指揮役を C ABI に出すかは今回は決めていない)。
- MIT の範囲に GPL のコードは無い (`packages/` を参照していない)。NuGet の依存は README の表のとおりで、増やしていない。
- `just unity-check` (.meta の検査・dotnet test 44 件・スタブの型検査・release.mjs のテスト) は通る。

### 横断

- **ホストの部分のテストの穴:** バインディングの「Unity / Godot などのエンジンに依存する部分」は型検査だけで、状態の扱いの不具合が
  見つからない。エンジンに依存しない判断をコアの層に寄せてテストする形 (T0446) は、`bindings/godot` にもありそう。
- **C ABI とバインディングの宣言の突き合わせが手作業:** T0430 の 9 つの漏れは、ヘッダの関数の一覧と `Native.cs` の宣言を比べるだけで見つかる。
  `ugoe.h` の関数名と各バインディング (Unity・Godot の C ABI を使う部分) の宣言を比べる検査を CI に足すとよい (除外は理由付きで列挙)。
- **言語の版・API の互換の検査:** 型検査の対象のフレームワークが 1 つだと、利用者が選べるほかの設定 (Unity の .NET Framework) で通らない
  API に気づかない (T0444)。ほかのバインディング (npm の対象の ES の版など) でも、検査の対象と利用者の環境が一致しているかを見るとよい。
- **外から来た文字列の NUL:** C ABI に文字列を渡すバインディングは、途中の NUL で切れる。ほかの C ABI の利用者 (Godot・適合テスト) も同じ。

### `just ci`

- 1 回目と 2 回目は `web-check` の `@ugoe/editor` のテストで `src/lib/preview/tracking.test.ts` の `beforeAll` が 10 秒の hook の時間切れ
  (`Error: Hook timed out in 10000ms`)。並行のワーカーで load average が 35 前後のとき。このタスクは `bindings/unity` と `docs/tasks` だけを変えている。
  そのファイルだけなら 8.3 秒で 7 件通り、load average が 21 に下がってから `pnpm --filter '@ugoe/editor' test` を流し直すと 44 ファイル 349 件すべて通った。
  `web-check` の残り (viewer・studio・face-tracking・player・i18n・runtime・ui の check と test)・`i18n`・`app-build-check` も個別に流して通った。
  `web-check` より前の段 (fmt-check・lint・test・layers・groups・licenses・publish-check・spec) は 1 回目で通っている。
