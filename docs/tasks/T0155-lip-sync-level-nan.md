---
id: T0155
title: リップシンクの音量の NaN を受け付けない
status: done
milestone: M9
crates: [ugoe-motion, ugoe, docs]
depends_on: [T0139]
assignee: claude-orchestrator
---

# T0155: リップシンクの音量の NaN を受け付けない

## 目的

`Puppet::set_lip_sync_level(Some(f32::NAN))` は `LipSyncPlayer::set_level` の `clamp` を NaN のまま通り抜け、リップシンクの対象の
パラメータが毎フレーム NaN になる (基準値には入らないので音量を戻せば消えるが、その間は形が NaN になる)。さらに指揮役の
`Puppet::lip_sync_applied()` が NaN の音量を返すので、受け手の `Puppet::follow` は `ParameterValuesError::LipSyncNaN` で
すべてのフレームを拒む (スタジオの受け手が止まる)。T0139 のレビューで見つけた。

音量の NaN をどう扱うか (`None` とみなす・0 にする・`set_level` で拒む) を決め、`LipSyncPlayer::set_level` (根) で扱う。

## 受け入れ条件

- [ ] NaN の音量の扱いを決め、`docs/spec/motion.md#リップシンク` に書いた (公開仕様 `spec/motion.md` に書くなら同じ PR で。docs/spec/public-spec.md)
- [ ] `LipSyncPlayer::set_level` が決めたとおりに扱い、`Puppet` の出力と `lip_sync_applied()` に NaN が出ないことをテストで確かめた
- [ ] wasm の `set_lip_sync_level` と C ABI の音量の口が同じ扱いになることを確かめた (違えば起票する)
- [ ] `just ci` が通る

## 参考

- docs/spec/motion.md#リップシンク・#受け手として進める
- crates/ugoe-motion/src/effects.rs (`LipSyncPlayer::set_level`)
- crates/ugoe/src/follow.rs (`Puppet::follow` の NaN の検査)

## メモ
- **閉じた (M9 のマージ):** T0317 (ugoe-motion のレビュー) で直した。LipSyncPlayer::set_level は NaN を「音量なし」として扱う。
