---
id: T0306
title: トラッキングの対応付け (ARKit → パラメータ)
status: done
milestone: M3
crates: [ugoe, ugoe-motion]
depends_on: [T0110, T0302]
assignee: claude-worker-T0306
---

# T0306: トラッキングの対応付け (ARKit → パラメータ)

## 目的

アバタープロファイルの対応表に従い、頭の向きと ARKit の 52 ブレンドシェイプからパラメータを計算する API を作る。モデルの上書き設定を反映する。

## 受け入れ条件

- [x] `docs/spec/avatar-profile.md` の表のすべての式にテストがある
- [x] モデルの `avatar.tracking` の上書きが効く
- [ ] 符号の約束 (`EyeBallX` など) を実際のトラッキングの値で確かめ、仕様の `> 未決:` を解消する (確かめられない場合はメモに残す)
- [x] `just ci` が通る

## 参考

- `docs/spec/avatar-profile.md#フェイストラッキングとの対応`

## メモ

- 実装は `crates/ugoe-motion/src/tracking.rs`。`TrackingInput` (頭・体の回転と 52 ブレンドシェイプの配列。名前で設定する `set_blend_shape(s)` は `_neutral` などの知らない名前を無視する)、`standard_value` (表の式、クランプ前)、`TrackingMap` (モデルごとに上書きを解決し、`apply` は文字列比較もメモリ確保もしない)。
- 仕様の上書き規則で曖昧だった点を決めて仕様に書いた: 無視される上書き (知らない `source` / `input`、書き込み先なし・モデルにない、`scale` / `offset` が非有限) は手順 2 の置き換えも起こさない。計算値が非有限ならそのパラメータに書き込まない。
- 平滑化・キャリブレーション・途切れたときのフェード・回転行列からオイラー角への変換・鏡像の入れ替えは Rust の対応付けに入れず、ホスト (T0703 のビューア) の責務とした (仕様の「実装の範囲」)。
- `Puppet::set_tracking(Option<TrackingInput>)` を追加した。T0303 の適用順の「3. トラッキング」(モーションの直後、基準値の保存の前) で書き込むので、表情・目パチ・呼吸はトラッキングの上に重なる。`None` に戻すと最後の値が基準値として残る。`docs/spec/motion.md` の適用順も更新した。
- トラッキング中はモデルの目パチが `EyeLOpen` に掛かるので、ビューアはトラッキング中に目パチを止める (`set_effects`) かを決める必要がある (T0703)。
- 符号の未決 (`EyeBallX` / `MouthX` / `JawX`) はカメラがなく確かめられなかった。仕様の `> 未決:` を T0703 に引き継いだ。
- MediaPipe の `Left` / `Right`: 公式文書には記載がないが、入力画像に写った顔自身の左右で、入力を左右反転すると入れ替わると報告されている (https://github.com/google-ai-edge/mediapipe/issues/6368)。Web のデモは反転しない入力、Android / iOS のサンプルは反転した入力を使う。ARKit も利用者自身の左右 (前面カメラの鏡像表示では画面の反対側)。反転しない映像を入れれば仕様の約束と一致する。
- wasm へのバインディング (`ugoe-wasm`) は範囲外なので入れていない。T0703 で `set_tracking` と名前での設定を公開する。
