# ugoe-render

**層:** L3 機能 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

wgpu による参照レンダラと参照シェーダ (WGSL)。

## 守ること

- Device / Queue / Surface を所有しない
- WebGL2 で使えない機能を使わない
- 描画の式は `docs/spec/rendering.md` に従う。式を変えたら仕様書も変える

## 仕様

- [docs/spec/rendering.md](../../docs/spec/rendering.md)

## テスト

- `just test-gpu` でオフスクリーン描画の画素を検証する (`tests/gpu.rs`、`gpu-tests` feature)。
- `UGOE_GPU_TEST_DUMP=<ディレクトリ>` を付けると、テストが描いた画像を `<テスト名>-<番号>.png` に書き出す
  (目で確かめる・変更の前後で画素を比べるため)。
- アダプタが見つからなければ失敗する。GPU がない環境では Mesa の lavapipe (Vulkan) か llvmpipe (GL) を使う。
  バックエンドは `WGPU_BACKEND=vulkan` / `WGPU_BACKEND=gl` などで選べる。

## 関連タスク

- T0201 (`docs/tasks/T0201-*.md`)
- T0202 (`docs/tasks/T0202-*.md`)
- T0203 (`docs/tasks/T0203-*.md`)
- T0208 (`docs/tasks/T0208-*.md`): 描画の統合
- T0209 (`docs/tasks/T0209-*.md`): ライティング (法線マップと光源)
