---
id: T0116
title: 公開仕様: 骨格とモデル・バンドルの形式 (英語)
status: todo
milestone: M5
crates: [spec, docs, scripts]
depends_on: []
assignee:
---

# T0116: 公開仕様: 骨格とモデル・バンドルの形式 (英語)

## 目的

ADR 0021 に従い、リポジトリの `spec/` に英語の公開仕様 (CC BY 4.0) を作る。まず骨格と、モデルのデータ (`model.json`)・バンドル (`.ugoe`)・前方互換 (機能の宣言とフォールバック) を書く。

## 受け入れ条件

- [ ] `spec/README.md` (目的・対象の読者・バージョン・草案であること・用語)、`spec/LICENSE` (CC BY 4.0 の全文)。ライセンスの検査 (`just licenses` や README の表) に `spec/` を反映する
- [ ] `spec/model.md`: すべての型とフィールド、既定値、検証の規則。実装の型名やタスク ID に頼らず、それだけで読めること
- [ ] `spec/bundle.md`: `.ugoe` のバイト列の形式 (チャンク・MessagePack・zstd・テクスチャ)、`schema_version`、機能の宣言、未知の値のフォールバック
- [ ] `docs/spec/` (日本語) との対応表を置き、食い違いが見つかったら両方を直す
- [ ] `just ci` が通る

## 参考

- `docs/adr/0021-public-specification.md`
- `docs/spec/model.md`、`docs/spec/file-formats.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
