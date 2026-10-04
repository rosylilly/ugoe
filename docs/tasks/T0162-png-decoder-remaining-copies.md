---
id: T0162
title: PNG のデコードの残りの写しを ugoe_format::image に寄せる (editor-core とテスト)
status: done
milestone: M9
crates: [ugoe-editor-core, ugoe-format, docs]
depends_on: []
assignee: claude-worker-T0162
---

# T0162: PNG のデコードの残りの写しを ugoe_format::image に寄せる (editor-core とテスト)

## 目的

T0009 で PNG のデコードと大きさの上限を `ugoe_format::image` にまとめた (`ugoe-headless`・`ugoe-wasm`) が、写しが 2 つ残っている。

- **`ugoe-editor-core/src/texture.rs` (優先):** 大きさの上限の無い写し。エディタは外から来た PNG (取り込み・差し替え) を読むので、巨大な画像でメモリを使い切りうる。
- `ugoe-format/tests/spec_bundles.rs` のテスト用のデコーダ (dev-dependency で自分の `png` feature を有効にすれば置き換えられる)。

## 受け入れ条件

- [x] `ugoe-editor-core` の PNG のデコードが `ugoe_format::image` を使い、上限を超える画像をエラー (コードつき。ADR 0029) で拒む。テストがある
- [x] `spec_bundles.rs` のテスト用のデコーダを `ugoe_format::image` に置き換えた
- [x] `crates/ugoe-editor-core/README.md` (変わるなら) を更新した
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0009 のメモ (起票の候補)
- crates/ugoe-format/src/image.rs

## メモ

- T0019 で起票した (元の候補: T0009 の 2 つ)。

### やったこと

- `ugoe-editor-core::texture`: `ugoe_format::image` の `ImageLimits`・`ImageError`・`Rgba8`・`DEFAULT_MAX_IMAGE_SIDE` を再公開し、
  `decode_png(bytes, ImageLimits) -> Result<Rgba8, ImageError>` を足した (中身は `ugoe_format::image::decode_png`)。
  これまでの `decode_rgba_png(bytes) -> Option<(w, h, rgba)>` は形を変えずに残し、既定の上限で共有の関数を呼ぶだけにした
  (ugoe-ai・ugoe-cli・エディタの Tauri 側の呼び出しを変えずに済む)。`ugoe-format` の `png` feature を有効にした。
- エディタの中で画素が要る処理は、上限を超える画像をコードつきのエラーで拒む (画素は確保しない):
  - メッシュの作り直し (`RegenerateMesh`): `EditError::TextureTooLarge` → `edit.texture_too_large`
  - 法線マップの生成 / 割り当て: `LightingError::TextureTooLarge` → `lighting.texture_too_large`、`LightingError::NormalMapTooLarge` → `lighting.normal_map_too_large`
  - アトラス化: `AtlasError::TextureTooLarge` → `atlas.texture_too_large`
  - 引数は `texture` (`name`)・`width`・`height`・`max`。上限は既定値 (16384) で読む。
- テスト: `texture` (上限の差し替えと、IHDR だけの 65535x65535 の PNG)・`import_tests` (作り直し)・`lighting_tests` (生成と割り当て)・`atlas_tests`。
- `ugoe-format/tests/spec_bundles.rs` の写しのデコーダを消し、`ugoe_format::image::decode_png` を使う
  (dev-dependency に自分自身を `features = ["png"]` で足した。`just publish-check` は通る)。
- 文書: `docs/editor.md` のコードの一覧と「テクスチャ」の節、`crates/ugoe-editor-core/README.md`。
- 範囲の外の小さな変更: 新しいコードの翻訳 (`apps/editor/locales/{ja,en}.json`) と `just gen` の生成物 (`apps/editor/src/lib/generated/messageCodes.ts`)。
  コードを足すとカタログの検査 (messageCodes.test・i18n) が要求するため。

### やらなかったこと

- 上限を利用者が変える口 (エディタの設定など) は作っていない。editor-core の中の呼び出しは既定値を使う (`decode_png` は上限を引数で受ける)。
- `add_texture` / `set_texture_image` (取り込み・差し替えの入口) では寸法を IHDR で読むだけで、上限での拒否はしていない
  (受け入れ条件はデコードの拒否。入口で拒むとコマンドの意味が変わるので判断を残した)。
- `module_compare/atlas.rs` と ugoe-ai・ugoe-cli の呼び出しは `decode_rgba_png` (Option) のまま。上限を超えると「読めない」と同じ扱いになる。

### 起票の候補

- テクスチャの追加・差し替え (`add_texture` / `set_texture_image`) で、一辺が上限を超える PNG を入口で拒む (いまは画素が要る処理で初めてエラーになる)。
- `ugoe-ai::normal` の `decode_rgba_png` の呼び出しを `decode_png` に替え、上限を超えたことを区別して伝える。

### 検査

- rebase の後に `just ci` が通った。`just wasm-check`・`just publish-check` も通った。
