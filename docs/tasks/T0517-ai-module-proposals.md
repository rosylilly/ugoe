---
id: T0517
title: AI: 着せ替えのモジュール向けの提案
status: done
milestone: M6
crates: [ugoe-ai, ugoe-rigging, apps/editor, docs]
depends_on: [T0648]
assignee: claude-worker-T0517
---

# T0517: AI: 着せ替えのモジュール向けの提案

## 目的

T0648 の後続の候補。モジュールのプロジェクトでは本体のパラメータしか使えないので、今の自動リギングの提案 (パラメータを足す) は採用時に拒否される。
衣装の PSD から、本体のパラメータとスロットで公開されたパラメータだけを使うリグ (頭に付く衣装は頭の変形に従う・揺れる衣装は物理など) を提案する。

## 受け入れ条件

- [x] モジュールのプロジェクトで使える提案: 取り付け先に合わせた変形 (本体のデフォーマに従う)・揺れ (物理のテンプレート)・見た目の差分の推定。パラメータを足さない
- [x] 衣装の意味ラベル (topwear・headwear など、T0503) を使う
- [x] 録画した応答でテスト。エディタの AI タブがモジュールのプロジェクトでこの提案を使う
- [x] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0648-editor-module-project.md`・`docs/tasks/T0504-auto-rig.md` のメモ

## メモ

- **やったこと**
  - `ugoe_ai::module_rig` (新規): モジュールのプロジェクトの作業用のドキュメントから、パラメータを足さず本体を変えない提案を作る。
    エフェクト・ミラーも触らない。軸と物理の入力は本体にある標準パラメータとスロットが公開するものだけ。
    - **追従 (取り付け先に合わせた変形):** 取り付け先の下にある本体のワープ (自分の軸を持つもの) のうち、衣装の外接矩形の 10% 以上を覆い、
      衣装と重なる本体のメッシュの描画順が衣装に最も近いものに従う (帽子は顔のワープで、後ろ髪のワープではない)。衣装を
      `Follow<ID>` のワープ (8×8、親は取り付け先) で包み、本体のワープと祖先 (取り付け先の手前まで) の軸のキーの組ごとに、
      本体のワープで写した位置を置く (`ParentWorld` で局所座標に写して戻す)。これを受け入れ条件の「見た目の差分の推定」と解釈した
      (キーフォーム = 本体の変形からの差分)。曲線補間も写す。キーの組が 81 を超えるワープには従わない。
    - **揺れ:** `hair_*`・`tail`・`earwear`・`neckwear`・`bottomwear` のラベルの衣装を 3×3 の揺れ用のワープで包み、追従のワープ
      (従うものがなければ取り付け先の直下の `SwayRoot`) を `frame` にした振り子 (rig.rs の髪揺れと同じ方法。`pendulum` を共有) で揺らす。
      入力は頭の衣装が `AngleX`、体の衣装が `BodyAngleX` (無ければ `AngleX`)。ラベルは T0503 の語彙 (`module_mesh_parts` がモジュールのメッシュだけに対応付け)。
    - **AI が決める値:** `ModuleRigParams { sway_scale 0〜2, sway_length 0.5〜1.5 }`。問い直し・注文・`Not applied:`・作り直し
      (`PreviousModuleRig`・`ModuleRigGuidance`) は rig.rs と同じ形。システムプロンプトに「モジュールはパラメータを足せない」と書く。
    - 組むものがなければ `InvalidRequest` (nothing to rig)。提案は複製 (モジュールの情報つき) に適用し、`validate` と `split_module` を確かめる。
  - rig.rs: 補助 (`Rig`・`Rect`・`create_warp_as`・`mesh_bounds`・`mesh_parts_where`・`hair::Sway`) を `pub(crate)` にし、`pendulum` を
    `frame`・根元の座標系・入力のパラメータを受け取る形にした (モデルの提案の出力は変わらない)。`turn::Builder::from_document`。
  - エディタ: `proposals.rs` の `rig` が `Document` を受け取り、モジュールのプロジェクトなら `module_rig` を使う (IPC は同じ。`RigAsk`・
    `PreviousAnswer`。種類の違う前の答えでの作り直しは断る)。`make_rig` は `Document::detached` で複製を取る。AI タブの見出し・説明・注文の
    欄の説明がモジュール向けになる (`ProposalsPanel` の `module`)。T0650 の「できないこと」の AI の項目を「できること」に移した (`MODULE_CAN`/`MODULE_CANNOT`)。
  - 右の列のタブを折り返す (`.tabs` に `flex-wrap`)。6 つのタブが列の幅を超え、AI タブを押すと列ごと横に送られて本文の左が切れていた (T0649/T0650 以降の既存の不具合)。
  - 文書: `docs/ai-integration.md#モジュールの自動リギング`、`docs/editor.md#モジュールのプロジェクトでの-ai-の提案` (と「使えるもの」の節)、
    `docs/manual/dress-up.md#ai-に衣装のリグを提案させる`・`docs/manual/ai.md`、ugoe-ai の README。
- **範囲外の変更 (ugoe-rigging の `world.rs` だけ。team-lead の承認済み。`crates:` に足した):** `ugoe_rigging::world::ancestors_only` が
  `model.slots` を消していなかったため、スロットの取り付け先が親の祖先以外のデフォーマだと `ParentWorld` (と editor-core の `canvas_to_local`) が
  検証エラーで失敗していた。モジュールのプロジェクトの作業用のモデルで必ず起きる (着せ替えの見本の本体のモジュールで、座標変換を使う編集が失敗するはず)。
  `out.slots.clear()` と `out.avatar = None` (アバタープロファイルは消したスロットと当たり判定を名前で参照する) を足し、`dress_up_base`
  (取り付け先の違う 2 つのスロットとアバタープロファイル) で単体テストを足した。ほかに消えた要素を参照しうるものを確かめた: メッシュ・
  グルー・当たり判定・ポーズ・エフェクトはもとから消している。残すパーツ (親のパーツ)・パラメータのグループ (パラメータ)・テクスチャ・
  デフォーマの `part` はどれも消さない要素だけを参照し、デフォーマのミラーは消している。ライティングは真偽値だけ。
- **テスト:** `crates/ugoe-ai/tests/module_rig.rs` (6 件: 実際の PSD にテンプレートのリグとスロットを組んだ本体 + テストで作った帽子と耳飾りで、
  帽子の頂点がキーの上で本体の顔のワープの中の点と 2 割以内で一致・パラメータが増えない・モジュールに分けて合成できる・`Puppet` で耳飾りが
  揺れて帽子は揺れない・組むものがないときの断り・モデルのプロジェクトの断り・見本のアバターで後ろ髪ではなく顔のワープに従う・録画
  `tests/cassettes/module_rig_retry.json` (手で作った。1 回目は範囲の外、2 回目は正しい値と Not applied 1 件) の再生)、module_rig.rs の単体 4 件、
  ugoe-rigging 1 件、エディタ `module_projects_get_module_rigs` (規則の経路の提案と採用・FakeProvider での作り直し・種類の違う前の答えの断り)。
- **確かめたこと:** `just ci` (main (T0711) に rebase した後、`just wasm` 2 つの後)。`cargo check -p ugoe-editor` は ci の中。
  ヘッドレスの Chrome と使い捨ての HTTP ブリッジ (scratchpad/T0517/bridge。T0650 のものに AI の提案の IPC を足し、本物の `Document`・
  `Proposals`・`ugoe_ai::module_rig` を使う。AI は録画を再生): 見本のアバター + スロット `Headwear` (取り付け先 `HeadRotation`) の本体の
  モジュールを開く → 帽子と耳飾りの PSD を「Import PSD…」→ AI タブ (見出し「Auto rig the costume from PSD」と説明) → Classify → Propose rig
  (規則。`FollowFaceWarp` と耳飾りの揺れ) → Use Claude + 注文で Propose rig (送信 2 回・Not applied 1 件) → Preview で `AngleX` = 30
  (帽子と耳飾りが顔と一緒に振り向く。ドキュメントは変わらない) → Adopt (パラメータ 27 のまま)。横のはみ出し 0。ツールチップの「できること」。
  スクリーンショット: scratchpad/T0517/shots/t0517-0{1,2,3,4,5}-*.png。
- **確かめていないこと:** 実際の Tauri のウィンドウ、実際の AI の API (録画は手で作った)、ブリッジでの送る前の確認のダイアログ
  (ブリッジは同意を省いた。確認の中身はモデルと同じ `prepare_instructed` に `module_rig_request_guided` を渡すだけ)、ビューポートでの物理の揺れの見た目
  (数値は `Puppet` のテスト)、`BodyAngleX` を入力にする衣装 (スカート・首の飾り・尻尾) の見た目、取り付け先がキャンバスのスロット、スロットが
  公開するパラメータを軸に持つ本体のワープ。`just test-gpu` は流していない (描画・構造体の変更なし)。Command の enum は足していない。
- **後続の候補:**
  - 追従のワープは本体のワープを 8×8 で近似する (見本で振り向きの移動量の 1 割ほどずれる)。衣装のメッシュのキーフォームを直接写す形も選べるとよい。
  - 本体のメッシュのキーフォームだけで動く部分 (目の開閉など) への追従 (見た目の差分をメッシュからも推定する)。衣装の差分 (A/B) をポーズにする提案。
  - 物理の衝突体 (スロットが公開する本体の衝突体) を使う提案 (長い髪・スカートが体にめり込まない)。
  - プレビューで物理を動かして揺れを確かめる (今は採用後にプレビューのモードで確かめる)。
  - ugoe-rigging の修正で、着せ替えの見本の本体のモジュール (スロットが別々のデフォーマに付く) での座標変換が直ったかを画面で確かめる。
