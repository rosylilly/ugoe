# テスト方針

## 種類

| 種類 | 場所 | 実行 |
| --- | --- | --- |
| ユニットテスト | 各クレートの `#[cfg(test)] mod tests` | `just test` |
| 結合テスト | 各クレートの `tests/` | `just test` |
| スナップショットテスト | `insta` (`tests/snapshots/`) | `just test`。更新は `cargo insta review` |
| 描画テスト (GPU) | `gpu-tests` feature を持つクレート (`ugoe-render` など) | `just test-gpu` |
| フロントエンドの検査 | Biome + svelte-check | `just web-check` |
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


## AI 関連

- 実際の API は CI で呼びません。`FakeProvider` に録画した応答を読ませてテストします。
- 実際の API を使うテストは `#[ignore]` を付け、環境変数 `UGOE_AI_LIVE=1` のときだけ手動で実行します。
