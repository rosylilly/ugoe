---
id: T0532
title: AutoMeshOptions::check を ugoe-mesher の検証と揃える
status: done
milestone: M9
crates: [ugoe-mesher, ugoe-editor-core, docs]
depends_on: [T0525]
assignee: claude-worker-T0532
---

# T0532: AutoMeshOptions::check を ugoe-mesher の検証と揃える

## 目的

T0525 で `ugoe_mesher::generate` の `margin` に上限 (1024 px) を設けた (大きな値で作業用の格子の大きさがあふれ、
確保が爆発していた)。`ugoe-editor-core` の `AutoMeshOptions::check` (`src/automesh.rs`) は mesher の検証を
写した別の実装で、上限を知らない。いまは `generate` が `MeshError::InvalidOptions` を返すので壊れはしないが、
PSD の取り込みではレイヤーごとのエラーになり、オプションの誤りとして最初に弾けない。

検証を 1 か所にする。例: `ugoe-mesher` に `MeshOptions::validate(&self) -> Result<(), MeshError>` を公開し、
`AutoMeshOptions::check` はそれを呼ぶ (公開 API の追加)。

## 受け入れ条件

- [x] `margin` の上限を含め、mesher と editor-core の検証が同じ規則になる (同じ関数を使う)
- [x] `margin` が 1024 を超えるオプションで PSD を取り込むと `PsdImportError::InvalidMeshOptions` になるテスト
- [x] `just ci` が通る

## 参考

- `crates/ugoe-mesher/README.md` の「オプション」
- `crates/ugoe-editor-core/src/automesh.rs`・`src/import.rs`

## メモ

- やったこと:
  - `ugoe-mesher` に `MeshOptions::validate(&self) -> Result<(), MeshError>` を公開した (中身は従来の非公開の `validate`。`generate` もこれを呼ぶ)。
  - `AutoMeshOptions::check` は `MeshOptions::from(*self).validate()` を呼び、`InvalidOptions` の理由を返す (戻り値の型 `Result<(), &'static str>` は変えていない)。
    これで `margin` が 1024 を超えると PSD の取り込みはレイヤーを処理する前に `PsdImportError::InvalidMeshOptions` になる。理由の文言は mesher のもの (例: `margin must be between 0 and 1024`) に変わった。
  - テスト: mesher の `validate_accepts_margin_up_to_the_limit`、editor-core の `check_follows_mesher_validation`、`rejects_invalid_mesh_options` に `margin = 1025` の場合 (修正前に失敗することを確かめた)。
  - 文書: `crates/ugoe-mesher/README.md` の「オプション」、`docs/editor.md` の自動メッシュの設定。
- やらなかったこと: `AutoMeshOptions` のフィールドの doc コメント (ts-rs と JSON Schema に出る) に上限を書くこと (生成物の再生成が要るため見送り)。
- 起票の候補: なし。

