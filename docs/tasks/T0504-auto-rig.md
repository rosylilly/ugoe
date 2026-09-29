---
id: T0504
title: 自動リギングの提案
status: done
milestone: M6
crates: [ugoe-ai, docs]
depends_on: [T0503, T0601, T0509]
assignee: claude-worker-T0504
---

# T0504: 自動リギングの提案

## 目的

意味ラベルとテンプレートから `Proposal` を作る。

## 受け入れ条件

- [x] 顔の角度・目の開閉・口の開閉の提案が作れる
- [x] 提案が `validate` を通る
- [x] 検証エラーを AI にフィードバックして再試行する
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md#自動リギング`
- `crates/ugoe-cli/src/sample.rs` (T0702・T0704 の見本のジェネレータ。レイヤーの名前から部位を見つけ、コマンドの列でリグを組む。テンプレートの出発点になる)
- `crates/ugoe-editor-core/src/physics_template.rs` (T0634 の物理のテンプレート)

## メモ

- T0505 の `ugoe_ai::turn::mesh_labels` は、分類の結果 (`LayerLabel`) をメッシュの名前と `layer_path` の末尾で突き合わせる。同じ名前のメッシュがあるとラベルが付かない。PSD の取り込みがレイヤーのパスからメッシュ ID への対応を返すようにすると確実になる (T0505 のメモ)。

- やったこと
  - `ugoe_ai::rig`: ラベルのテンプレート `propose_rig` と、テンプレートで決められない値 (`RigParams`: 振り向きの係数・目の閉じる線・閉じた目の弧・1 枚の口の開き) を
    AI に決めさせる `propose_rig_with_ai`。組み方・値の範囲・再試行の手順は `docs/ai-integration.md#自動リギング` に書いた。
    レベル 1 のパラメータ (`AngleX/Y/Z`・`EyeL/ROpen`・`MouthOpenY`) を作る。頭の回転デフォーマ (`AngleZ`)、頭のワープ (深度からの回転)、目ごとのワープ (つぶす + 閉じた目の絵との入れ替え)、
    口 (開き・閉じの絵の入れ替え、または 1 枚の口をワープで縦に開く)。ミラー (`AngleX`・`AngleZ` は自分自身、`EyeLOpen`↔`EyeROpen`) も設定する。
  - 顔の角度は T0505 の `plan_turn` を使う。深度マップがあれば (`TurnDepth::Map`) それを、なければ顔の外接矩形に内接する楕円体の深度マップを作って同じ経路に通す (ラベルの補正はどちらにも効く)。
    `turn.rs` の `Builder` と振り向きのコマンドを作る部分 (`push_turn`) を `pub(crate)` にして共有した (`propose_depth_turn` の動作は同じ)。
  - 再試行: 範囲の外の値は丸めずエラー、範囲の中ならリグを組んで複製に適用し `validate`。通らない答え (スキーマに合わない答えを含む) は、前の答えとエラーをプロンプトに添えて問い直す。
    上限 (既定 3) で `InvalidProposal`。送信の失敗・`face` のないモデルなどは問い直さない (`face` がなければ AI を呼ぶ前に `InvalidRequest`)。
  - `rig::mesh_parts` (ラベルと左右をメッシュに対応付ける)。`turn::mesh_labels` はこれを使うようにした (同じ名前のメッシュはレイヤーも同じ名前なので、結果は変わらない)。
  - `classify::overview_png` (原画の PNG。AI に送る)。
  - 例 `examples/auto_rig.rs` (PSD → 名前の規則で分類 → 提案 → 適用 → `.ugoe`)。
- `sample.rs` との関係: 共有せず、補助 (`keyform_at`・`set_cell`・`shape`・`fade`) と目・口の切り替えの値を ugoe-ai に写した。
  見本はレイヤーの名前で部位を探すが、こちらは意味ラベルで探すので組み方が違い、見本の出力を変えないため (team-lead の了承済み)。見本の `sample.rs` は変えていない。
- 範囲外の変更 (team-lead の承認済み。`crates:` に docs を足した): `docs/ai-integration.md#自動リギング` にテンプレートの表・AI が決める値・再試行の手順を書いた。仕様とコードを同じ PR で合わせるため。
- 確認
  - `tests/auto_rig.rs`: 実際の PSD 2 つを取り込み、`classify_by_name` → `mesh_parts` → 提案 → `Batch` で適用 → `validate` が空。ugoe-runtime で評価して、
    既定値で見た目が変わらないこと、`EyeLOpen` = 0 で左目だけ閉じた目の絵に入れ替わること、0〜1 のどの値でも開いた目か閉じた目が不透明なこと、`MouthOpenY` で口が入れ替わること、
    `AngleX` = 30 で鼻が画面の右へ・後ろ髪は逆へ動くこと、`AngleZ` で首と体が動かないこと、全身の PSD で目が線につぶれ 1 枚の口が縦に伸びること、深度マップで深度が変わることを確かめる。
  - AI の経路: 録画 `tests/cassettes/auto_rig_retry.json` (Messages API の応答の形に合わせて手で作った。1 回目は `turn_angle_scale` 2.5、2 回目は 0.6) を `ClaudeProvider` で再生し、
    1 回目を捨てて 2 回目の値を使うこと、2 回目のリクエストに前の答えとエラーが入ることを確かめる。`FakeProvider` で上限での打ち切り・スキーマに合わない答えの再試行・`face` のないモデルで AI を呼ばないことも確かめる。
  - 目視 (`ugoe render`、llvmpipe): 胸から上の PSD で `AngleX` ±30 (顔が向いた側へ寄り、後ろ髪が反対側に見える)、`AngleY` ±30、`AngleZ` 30 (頭だけ傾く)、
    `EyeLOpen` 0.35 (細くつぶれる)・0 (閉じた目の絵)、`EyeROpen` 0、`MouthOpenY` 0.12 (細い隙間)・0.5・1 (大きく開く)、組み合わせ (両目を閉じて口を開け、斜め上を向く) を見た。自然に見える。
    全身の PSD では目が線につぶれて肌が見え、閉じて見える。1 枚の口は縦に少し太くなるだけで、開いて見えるほどではない (口の中の絵がないため)。
- 分かったこと・後続の候補
  - 全身の PSD では顔が小さく、頭のワープ (6×6) の制御点のうち顔の上にあるものがなく、前髪・後ろ髪のラベルの補正だけで振り向く (それでも向きは変わって見える)。顔の大きさから分割数を決めるとよい。
  - 1 枚の口: 口の中の絵を生成して置くか (T0702 の見本のように)、口の開きを表情のテクスチャで行う方法が要る。
  - 見本の `sample.rs` を `ugoe_ai::rig` を使う形に移す (見本の出力のテストが変わるので別タスク)。
  - レベル 2 のパラメータ (視線・眉・口の形・体の向き・呼吸)、物理のテンプレート (T0634) を組み合わせた提案。
  - 実際の API での録り直しと、プロンプトの調整 (キーがない)。エディタからの呼び出し (送る前の確認に `rig_request` と `disclosure` を使う)。
