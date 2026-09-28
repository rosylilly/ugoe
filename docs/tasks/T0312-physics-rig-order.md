---
id: T0312
title: 物理リグの順序付き評価と中継パラメータ
status: done
milestone: M3.5
crates: [ugoe-model, ugoe-physics, ugoe-motion, ugoe-editor-core, apps/viewer, docs]
depends_on: [T0305]
assignee: claude-worker-T0312
---

# T0312: 物理リグの順序付き評価と中継パラメータ

## 目的

Live2D の実践では、物理のグループを「上から順に処理される」前提で連鎖させ、あるグループの出力 (中継用パラメータ) を次のグループの入力にして、
「体が遅れて付いてくる」「弾んでから角度に戻る」「位置 Z に応じて足が曲がる」といった動きを作っている (参考の記事 4)。
今の ugoe は、すべてのリグがフレームの始めのパラメータを読み、出力はすべてのリグを進めた後に書くので、連鎖すると 1 フレーム遅れ、リグの順序に意味がない。

## 受け入れ条件

- [x] リグを `PhysicsSettings::rigs` の順に進め、前のリグの出力 (パラメータ) を後のリグが **同じステップで** 入力として読む。規則 (ステップの中の順序、出力の補間との関係) を `docs/spec/physics.md` に書く
- [x] 連鎖 (体の追従 → 弾み → 角度) のテスト。順序を入れ替えると結果が変わること、決定性、動的物理との組み合わせ
- [x] 中継用のパラメータを示すフラグ (例: `Parameter::internal`) を足す。トラッキングの対応付けは書き込まず、ビューアのスライダーとアプリ向けの一覧では既定で隠す。`validate` と `docs/spec/model.md`
- [x] エディタの物理のパネルでリグの順序を並べ替えられる (コマンドと Undo)
- [x] `just ci`・`just gen` (差分なし)・`just wasm-check` が通る

## 参考

- 乾物ひもの「物理演算を用いた動き表現」: https://note.com/himono_vtuber/n/n9015f0030d53
- `docs/spec/physics.md`

## メモ

- **順序付きの評価** (`ugoe_physics::PhysicsSystem::advance`): ステップを外側、リグを内側のループにした。
  リグ k の入力のうち、k より前のリグがパラメータに出力するパラメータを読むもの (中継の入力) は、
  そのステップで前のリグが出した補間しない出力を、パラメータの写し (`Relay`) から読む。写しの出力先は毎ステップの始めに
  フレームの始めの値に戻す。外からの入力は従来どおりフレームの間で補間し、中継の入力の分は補間せずに足す。
  静止姿勢 (最初の `update`) も並びの順に求める。パラメータに書く値は従来どおり直前 2 ステップの補間。
  規則は `docs/spec/physics.md#リグの順序と中継`。
- **既存の結果は変わらない:** 中継の入力がない設定は写しを作らず、処理も従来と同じ。作業中に旧実装を一時的に残して
  `physics_hair`・`dynamic_hair` で 400 フレームの出力と粒子の位置がビット単位で一致することを確かめた (比較のコードは消した)。
- **中継パラメータ:** `Parameter::internal` (serde 既定 false、false なら書き出さない。TS は `internal?: boolean`)。
  `ugoe_motion::TrackingMap::for_model` が中継用のパラメータを除き、`Puppet` と `ugoe-wasm` の `tracking_targets` はこれを使う。
  構造の検証 (`validate`) では制限しない (どこから参照してもよい) と決め、代わりにアバタープロファイルの検査
  (`check_profile`) で、トラッキング・視線の対象が中継用なら警告 (`InternalTrackingTarget`) にした。
- **テスト:** フィクスチャ `physics_relay` / `physics_relay_settings` (体の追従 → 弾み → 角度) を追加。
  `ugoe-physics/src/relay_tests.rs` で同じステップでの連鎖・逆順での遅れ・順序で結果が変わること・静止姿勢・決定性・
  動的物理との組み合わせを検査。
- **エディタ:** コマンド `reorder_physics_rig { id, before }` と `set_parameter_internal { id, internal }` を末尾に追加 (Undo のテストと見本)。
  物理のパネルでリグを ▲▼ で並べ替え、前のリグの出力を読むリグに「relay」の印。パラメータ一覧は中継用のパラメータを
  斜体と「relay」の印で出し、編集欄の「Relay (internal)」で切り替える。ビューアは既定で隠し、チェックで出す。
- **範囲外の小さな変更:** `Parameter` に項目を足したので、構造体リテラルに `internal: false` を足した
  (`ugoe-cli/src/sample.rs`、`ugoe-rigging/tests/*`、`ugoe-editor-core` のテスト)。`ugoe/src/lib.rs` と
  `ugoe-wasm/src/tracking.rs` は `TrackingMap::for_model` に切り替えた (1 行ずつ)。`apps/editor` は受け入れ条件のため変更。
- **後続の候補:** Cubism の物理設定の取り込みで、グループの順序と中継パラメータをそのまま写す。エディタのリグの一覧で
  ドラッグでの並べ替え (今はボタンだけ)。後ろのリグの出力を前のリグが読む (1 フレーム遅れる) 設定をエディタで知らせる。
