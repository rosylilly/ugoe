# ugoe-model

**層:** L1 データ (依存してよいクレートは `scripts/check-layers.mjs` を参照)

モデル・物理・モーション・表情のデータ定義 (スキーマ)。検証 (`validate`) とテスト用のフィクスチャ (`fixtures`) を含む。

## 守ること

- アルゴリズム (補間・変形・再生) を置かない
- 型を変えたら同じ PR で `docs/spec/model.md` を更新する。ファイル形式に影響する場合は `SCHEMA_VERSION` も更新する
- `fixtures` の既存の関数の挙動を変えない (追加だけにする)
- 型を追加したら `TS` / `JsonSchema` の derive (feature `ts` / `schema`) も付け、`just gen` を実行する ([型の生成](../../docs/editor.md#型の生成))

## 仕様

- [docs/spec/model.md](../../docs/spec/model.md)

## 関連タスク

- T0102 (`docs/tasks/T0102-*.md`)
- T0103 (`docs/tasks/T0103-*.md`)
