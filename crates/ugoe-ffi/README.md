# ugoe-ffi

**層:** L5 バインディング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

C ABI。Unity・Unreal・Godot などから使う。

## 守ること

- `unsafe` はこのクレートでだけ許可する
- パニックを FFI の境界の外へ漏らさない
- ABI を壊したら `ABI_VERSION` を上げる

## 仕様

- [docs/portability.md](../../docs/portability.md)

## 関連タスク

- T0401 (`docs/tasks/T0401-*.md`)
