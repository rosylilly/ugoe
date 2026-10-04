# ugoe-motion

**層:** L3 機能 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

モーションの再生、表情、目パチ・呼吸・リップシンク・ポーズ。

## 守ること

- 乱数はシード付きの決定的なものを使う
- フレーム内の適用順は `ugoe` (Puppet) が管理する。このクレートは個々の処理を提供するだけ
- パラメータの添字は ID から解決して持ち、渡された `ParameterValues` の並びが違えば解決し直す (`ParameterValues::has_layout` で定数時間に見分ける。T0325)。毎フレームの処理で名前を探したりメモリを確保したりしない

## 仕様

- [docs/spec/motion.md](../../docs/spec/motion.md)

## 関連タスク

- T0302 (`docs/tasks/T0302-*.md`)
- T0303 (`docs/tasks/T0303-*.md`)
