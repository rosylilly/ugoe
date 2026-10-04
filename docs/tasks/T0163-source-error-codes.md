---
id: T0163
title: 検証・合成・推定のエラーにもコードを付け、エディタの自由な文の引数を訳せるようにする
status: done
milestone: M9
crates: [ugoe-model, ugoe-rigging, ugoe-editor-core, apps/editor, docs]
depends_on: [T0015, T0678]
assignee: claude-worker-T0163
---

# T0163: 検証・合成・推定のエラーにもコードを付け、エディタの自由な文の引数を訳せるようにする

## 目的

エディタのエラーは ADR 0029 のコードと引数で訳すようになった (T0681 でメッセージの一覧も検査する) が、引数のうち自由な文
(`reason`・`detail`・`details`) は英語のまま出る (`docs/editor.md` の「訳すには、その元のエラーにもコードを付ける必要があります (後続)」)。
元のエラー (モデルの検証 `ValidationError`、モジュールの合成 `ComposeError`、キーフォームの推定 `ugoe-rigging` のエラーなど) が
機械の読めるコードと引数を持てば、エディタはそれを訳せる。

**判断が要る:** `ugoe-model` の公開 API の変更 (エラー型にコードを足す)。multi-agent.md に従い、「インターフェース (エラー型のコードと引数の口)」を
先に小さな PR で入れ、エディタの訳を後に入れる。コードの名前の付け方を ADR 0029 と `DressUpError::code`・`LoadWarning::code` に揃える。

## 受け入れ条件

- [x] 対象のエラー型と、コードの口の形を決めた (メモに表)。`ugoe-model` の分を先の PR に分けた
- [x] `ValidationError` などにコードと引数を足し、テストで固定した。`docs/spec/model.md` と公開仕様 (`spec/`) を必要なら更新した
- [x] `ugoe-editor-core` の `message_codes.rs` が元のエラーのコードを引数に載せ、エディタが訳す (翻訳と `docs/editor.md#エラーと警告のコード` を更新)
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0681 のメモ (起票の候補)、docs/editor.md、ADR 0029
- T0015 (検証エラーの列の見せ方)・T0678 (モジュールのエラーのコード)・T0157 (着せ替えのエラーの補足の値)

## メモ

- T0019 で起票した (元の候補: T0681)。エラーの列の見せ方 (T0015) とモジュールのエラー (T0678) が同じ型を触るので、その後に着手する。
- T0163 (claude-worker-T0163) のメモ:
  - **口の形:** 元のクレートのエラーに `code() -> &'static str` (バリアントの名前の `snake_case`) だけを足した。補足の値はバリアントの
    公開のフィールドそのもので、別の値の型 (SDK の `FieldValue` のようなもの) は作らない (公開 API を最小にするため)。
    editor-core は `Coded` をこれらの型に実装し、領域の名前を付けたコードと、フィールドからの引数を出す
    (`source_error_codes_follow_the_crate_codes` が元のコードと一致することを確かめる)。

    | エラー型 | クレート | 元のコード | editor-core のコード | 載せる場所 |
    | --- | --- | --- | --- | --- |
    | `ValidationError` | ugoe-model | `ValidationError::code` (27 種) | `validation.*` | 検証の一覧を持つコードの `Message::items` (先頭 10 件) |
    | `InferError` | ugoe-rigging | `InferError::code` | `infer.*` | `edit.inference_failed` の `cause`、`RederiveWarning::cause` |
    | `WorldError` | ugoe-rigging | `WorldError::code` | `world.*` | `infer.parent_transform` の `cause` |
    | `NormalMapError` | ugoe-rigging | `NormalMapError::code` | `normal_map.*` | `lighting.generate_failed` の `cause` |
    | `ComposeError` | ugoe-runtime (範囲外) | なし (editor-core が SDK の `DressUpError::code` と同じ名前で付ける) | `compose.*` | `module.compose_failed` の `cause` |

  - **ugoe-model の分は先のコミットに分けた:** `feat(model): add machine-readable codes to validation errors` は model と仕様書だけで、
    単独で取り込める (足すだけで既存の型は変えていない)。PR を分けるなら、このコミットを先に cherry-pick する。
  - `Message` に `items` (省略できる一覧の中身) を、`RederiveWarning` に `cause` (省略できる) を足した。`detail`・`details` の英語の文は
    互換のため残す (MCP・CLI・古い画面)。画面 (`errors.ts` の `messageText`) は、`detail` を持つコードなら原因の訳を `detail` に差し込み
    (後ろに「原因:」を付けない)、`items` があれば各項目の訳を `; ` でつないで `details` に差し込む (残りは `errorListMore`)。
  - 翻訳: `validation`・`infer`・`world`・`normal_map`・`compose` の文を `en.json`・`ja.json` に足した。これらは括弧や一覧の中に入るので、
    文頭を大文字にせず、句点を付けない。`errorArgs` に `expected` の値 (`finite and >= 0`・`between min and max`・`normal`) を足した。
  - `docs/spec/model.md#検証エラーのコード`・`docs/editor.md#エラーと警告のコード`・ugoe-rigging の README を更新した。公開仕様 (`spec/`) は
    Rust の API なので書かない (`public-spec.md` の備考に書いた)。
  - **やらなかったこと:** editor-core が自分で組む理由の文 (`edit.invalid_physics`・`edit.invalid_motion`・`edit.deformer_edit_failed`・
    `edit.cannot_regenerate_mesh` などの `reason`)、自動メッシュ (`MeshError`。`edit.automesh_failed`・`psd.automesh_failed`)、PSD の読み込み
    (`PsdError`)、`space.evaluate_failed` の `detail` はまだ英語。`validation.invalid_blend_shape` の `reason` と `validation.invalid_value` の
    `expected` の一部 (`in 0..=1` など、`.` を含みカタログのキーにできない値) も英語のまま。
  - 起票の候補:
    - `ugoe-runtime` の `ComposeError::code()` を足し、SDK の `DressUpError::info` と editor-core の `compose.*` をそれから作る (今は名前を 2 か所で揃えている)。
    - `ugoe-mesher` の `MeshError`・`ugoe-psd` の `PsdError` にもコードを付け、`edit.automesh_failed`・`psd.*` の `detail` を訳す。
    - editor-core が組む `reason` の文 (物理・モーション・表情・デフォーマの編集) を、コードと引数を持つ型にする。
    - SDK・wasm・C ABI の `DressUpError::fields` の `errors` (検証のエラーの数だけ) に、`ValidationError::code` の一覧を載せる。

