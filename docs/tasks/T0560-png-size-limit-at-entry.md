---
id: T0560
title: AI の法線マップとテクスチャの追加・差し替えで、PNG の一辺の上限を入口で確かめる (判断が要る)
status: done
milestone: M9
crates: [ugoe-ai, ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0560
---

# T0560: AI の法線マップとテクスチャの追加・差し替えで、PNG の一辺の上限を入口で確かめる (判断が要る)

## 目的

T0162 の残り (T0162 の候補)。

- `ugoe-ai::normal` は `decode_rgba_png` を使い、上限を超えた PNG を「読めない」と同じに扱う。`ugoe_format::image::decode_png` に替え、上限の超過を区別して伝える。
- editor-core のテクスチャの追加・差し替え (`add_texture`・`set_texture_image`、`crates/ugoe-editor-core/src/document/texture_edit.rs`) は、一辺が上限を超える
  PNG を受け入れ、画素が要る処理で初めてエラーになる。

**判断が要る:** テクスチャの追加・差し替えで上限を超える PNG を入口で拒むか (拒むと、今まで開けたプロジェクトに足せなくなる画像がある)。

## 受け入れ条件

- [x] `ugoe-ai::normal` が `decode_png` を使い、上限の超過が別のエラーになるテストがある
- [x] テクスチャの入口で拒むかを決め (利用者の判断を待つなら blocked にしてメモに案を書く)、拒むならコマンドのエラーとコードと訳があり、テストがある
- [x] `just ci` が通る

## 参考

- T0162 のメモ (起票の候補)、crates/ugoe-format/src/image.rs

## メモ

- T0026 で起票した (元の候補: T0162 の 2 つ)。
- T0560 (claude-worker-T0560):
  - **判断: 入口で拒むことにした。** 案は (A) 入口で拒む、(B) 今のまま受け入れ画素が要る処理で拒む、(C) 受け入れて警告する、の 3 つ。
    上限 (既定 16384) を超える画像は、エディタのビューポート (`ugoe-wasm` の既定の `max_image_side` で、エディタは変えていない)・
    ランタイム (headless・wasm)・メッシュの作り直し・アトラス化・法線マップのどれも読めないので、置けても使えない。拒んでも
    利用者ができなくなることはなく、エラーが早く分かりやすくなるだけなので (A) にした (T0150 の「超えたらエラーで止める」とも合う)。
    16384 を超える一辺を扱える GPU はまれで、RGBA8 で 1 GiB を超える。上限ちょうどは受け入れる (テストあり)。
  - `AddTexture`・`SetTextureImage`・`SetNormalMap` の `png` が、IHDR の寸法だけを見て `EditError::TextureTooLarge` (`edit.texture_too_large`。既存のコードと訳を使う) を返す
    (`src/document/texture_edit.rs` の `png_dimensions`)。プロジェクトを開く (`project::read_images`) は確かめないので、以前に置いた大きな画像を持つプロジェクトは開ける。
    Undo はスナップショットなので影響しない。メッシュの作り直しのテストは、そうしたプロジェクトを開いた状態 (`Document::with_textures`) で作るようにした。
  - `ugoe-ai::normal` は `decode_png` を使い、上限の超過を新しい `AiError::TextureTooLarge { texture, width, height, max }` で返す (読めない画像は従来どおり `InvalidRequest`)。
    範囲外の小さな変更: エディタ (`apps/editor/src-tauri/src/error.rs`) のコード `ai.texture_too_large` と引数・カタログの見本、日英の訳、`just gen` の `messageCodes.ts`。
  - 仕様: docs/editor.md (テクスチャの節とコードの表)・docs/ai-integration.md (法線マップの生成)・Command の doc コメント (`command.schema.json` を再生成)。
  - 起票の候補: PSD の取り込みで上限を超えるレイヤーがあると、取り込み全体が `edit.texture_too_large` になる (以前は取り込めてもビューポートで読めなかった)。
    そのレイヤーだけを飛ばして警告する (`psd.*` の警告) か、縮小して取り込むかを検討する。
