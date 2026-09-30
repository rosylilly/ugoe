# ugoe-model

**層:** L1 データ (依存してよいクレートは `scripts/check-layers.mjs` を参照)

モデル・物理・モーション・表情のデータ定義 (スキーマ)。検証 (`validate`) とテスト用のフィクスチャ (`fixtures`) を含む。

## 守ること

- アルゴリズム (補間・変形・再生) を置かない
- 型を変えたら同じ PR で `docs/spec/model.md` と公開仕様の `spec/model.md` を更新する。ファイル形式に影響する場合は `SCHEMA_VERSION` も更新する。
  `just gen` で公開仕様の JSON Schema (`spec/schema/`) を作り直し、`just spec` で `spec/model.md` のフィールドの表と合っていることを確かめる
- `fixtures` の既存の関数の挙動を変えない (追加だけにする)
- 型を追加したら `TS` / `JsonSchema` の derive (feature `ts` / `schema`) も付け、`just gen` を実行する ([型の生成](../../docs/editor.md#型の生成))
- `fixtures` の関数のうち `spec_files::example_models` に挙げたものは、公開仕様の例 (`spec/examples/`) にもなっている。
  足すだけなら挙動は変わらないが、`just gen` で `model.json` が変わる

## 仕様

- [docs/spec/model.md](../../docs/spec/model.md)
- 公開仕様: [spec/model.md](../../spec/model.md)、[spec/schema/](../../spec/schema/README.md)、[spec/examples/](../../spec/examples/README.md)

## 関連タスク

- T0102 (`docs/tasks/T0102-*.md`)
- T0103 (`docs/tasks/T0103-*.md`)
