# テスト方針

## 種類

| 種類 | 場所 | 実行 |
| --- | --- | --- |
| ユニットテスト | 各クレートの `#[cfg(test)] mod tests` | `just test` |
| 結合テスト | 各クレートの `tests/` | `just test` |
| スナップショットテスト | `insta` (`tests/snapshots/`) | `just test`。更新は `cargo insta review` |
| 描画テスト (GPU) | `gpu-tests` feature を持つクレート (`ugoe-render` など) | `just test-gpu` |
| フロントエンドの検査 | Biome + svelte-check + Vitest (`apps/editor/src/**/*.test.ts`) | `just web-check` |
| 依存方向の検査 | `scripts/check-layers.mjs` | `just layers` |

`just ci` はこれらを (GPU テスト以外) すべて実行します。

## フィクスチャ

- ランタイムのテストの入力には `ugoe_model::fixtures` のモデルを使います。バイナリファイルを置くより、コードで組み立てたモデルのほうが差分を読みやすく、スキーマの変更にもコンパイラが追従させてくれるためです。
- 新しい機能には新しいフィクスチャ関数を追加します。**既存のフィクスチャの挙動は変えない** でください。他のエージェントのテストが壊れます。
- 画像や PSD のように、どうしてもファイルが必要な場合は `assets/fixtures/` に置きます。1 ファイル 1MB 以下にし、出所とライセンスを `assets/fixtures/README.md` に書きます。

## ランタイムの数値テスト

- 変形の結果は、許容誤差を付けて比較します (目安は `1e-4` px)。
- 仕様書の式に対応するテストには、テスト名かコメントで仕様書の節を示します。
- 評価の出力全体を `insta` の JSON スナップショットにしてよいですが、スナップショットを更新するときは、PR 本文に「なぜ値が変わったのか」を書きます。

## 描画テスト

- `ugoe-render` の描画テストは、オフスクリーンのテクスチャに描画して画素を読み戻し、期待する画像と比較します (許容誤差付き)。
- GPU を使うテストは各クレートの `gpu-tests` feature の裏に置きます。`just test-gpu` は `scripts/gpu-test-args.mjs` で `gpu-tests` feature を持つワークスペースのクレートを探し、まとめて `cargo test -p <crate> --features <crate>/gpu-tests` します。新しいクレートに描画テストを足すときは feature を定義するだけでよく、justfile や CI を直す必要はありません。
- アダプタが見つからないときは、飛ばさずに失敗させます (feature を明示的に有効にしたときだけ動くため)。
- 期待画像は `crates/ugoe-render/tests/golden/` に置きます。更新するときは PR に新旧の画像を載せます。
- `just test-gpu -- --nocapture` とすると、テストが選んだアダプタ (名前・ドライバ・バックエンド) が表示されます。

### CI での実行

GPU のない GitHub Actions の Linux ランナーでは、Mesa のソフトウェア実装で実行します (`.github/workflows/ci.yml` の `gpu` ジョブ)。バックエンドごとに matrix で 2 本走ります。

| matrix | ドライバ | 意味 |
| --- | --- | --- |
| `vulkan` | lavapipe (Vulkan) | ネイティブの主経路 |
| `gl` | llvmpipe (OpenGL ES / EGL) | WebGL2 相当のバックエンドでも動くことの確認 |

使う環境変数:

| 変数 | 値 | 目的 |
| --- | --- | --- |
| `WGPU_BACKEND` | `vulkan` / `gl` | wgpu が使うバックエンドを 1 つに絞る (テストは `InstanceDescriptor::new_without_display_handle_from_env` で読む) |
| `VK_DRIVER_FILES` (と旧名の `VK_ICD_FILENAMES`) | `/usr/share/vulkan/icd.d/lvp_icd*.json` | Vulkan ローダーが lavapipe 以外のドライバを読まないようにする |
| `LIBGL_ALWAYS_SOFTWARE` | `1` | GL を llvmpipe に固定する |

ジョブは Tauri のシステム依存を入れず、`just test-gpu` が指定するクレートのテストだけをビルドします。ログの `vulkaninfo --summary` (vulkan のみ) と `ugoe-render gpu tests: using adapter ...` の行で、どのドライバで走ったかを確認できます。

### 手元で CI と同じ条件で動かす

Mesa (`mesa-vulkan-drivers` `libegl-mesa0` `libgl1-mesa-dri`) が入っていれば、手元の GPU があってもソフトウェア実装に固定して再現できます。

```sh
icd="$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -n 1)"
# lavapipe (Vulkan)
VK_DRIVER_FILES=$icd VK_ICD_FILENAMES=$icd WGPU_BACKEND=vulkan just test-gpu -- --nocapture
# llvmpipe (GL)
WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 just test-gpu -- --nocapture
```

WSL2 では環境変数を付けないと dzn (Vulkan on D3D12) が選ばれることがあります。dzn はインスタンスの破棄時に SIGSEGV を起こすので、テストはデバイスを static に置いて破棄しないようにしています (`crates/ugoe-render/tests/gpu.rs`)。


## モーションの途中の絵を確かめる

モーション・物理・表情の結果は、自動テスト (`crates/ugoe/tests/m3_puppet.rs`・`crates/ugoe-editor-core/tests/m3_workflow.rs`) が
パラメータと頂点の値で確かめます。見た目は次の手順で PNG にして目で確かめます (GPU が無ければ lavapipe / llvmpipe で動きます)。

```sh
# エディタで作ったモデル (物理・モーション・表情を含む .ugoe) の、モーション `Shake` の途中のコマ
just cli render model.ugoe -o shake-0.25.png --scale 2 --motion Shake --time 0.25
just cli render model.ugoe -o shake-0.50.png --scale 2 --motion Shake --time 0.5
# 同じ時刻を物理なしで描き、髪の遅れ (物理の効果) を見比べる
just cli render model.ugoe -o shake-0.25-still.png --scale 2 --motion Shake --time 0.25 --no-physics
# 表情だけ・物理だけ (頭を 30 にした直後から 0.2 秒揺らす)
just cli render model.ugoe -o smile.png --expression Smile --no-effects
just cli render model.ugoe -o swing.png --param AngleX=30 --time 0.2
```

- `--time` は `Puppet::update(1 / fps)` を繰り返して進めるので (既定 60 fps)、物理の揺れもその時刻のものになります。
  `--param` の値は基準値になり、モーションの対象のパラメータはモーションの値で上書きされます。
- 目パチは乱数のシードで決まるので同じ時刻なら同じ絵ですが、見比べるときは `--no-effects` で止めると分かりやすくなります。
- 連番のコマは、`--time` を変えて繰り返し呼びます (例: `for t in 0 0.1 0.2 0.3; do just cli render m.ugoe -o f-$t.png --motion Shake --time $t; done`)。
- 自動の描画テスト `crates/ugoe-cli/tests/m3_render_gpu.rs` (`just test-gpu`) も同じ手順で `shake-*.png` などを
  一時ディレクトリの `ugoe-m3-render/` に残します。

## AI 関連

- 実際の API は CI で呼びません。`FakeProvider` に録画した応答を読ませてテストします。
- 実際の API を使うテストは `#[ignore]` を付け、環境変数 `UGOE_AI_LIVE=1` のときだけ手動で実行します。
