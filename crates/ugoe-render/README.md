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
- アダプタが見つからなければ失敗する。GPU がない環境では Mesa の lavapipe (Vulkan) か llvmpipe (GL) を使う。
  バックエンドは `WGPU_BACKEND=vulkan` / `WGPU_BACKEND=gl` などで選べる。

## 関連タスク

- T0201 (`docs/tasks/T0201-*.md`)
- T0202 (`docs/tasks/T0202-*.md`)
- T0203 (`docs/tasks/T0203-*.md`)
