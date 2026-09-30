---
id: T0645
title: トラッキング: MediaPipe の Pose Landmarker で体の向きと肩を取り込む
status: done
milestone: M4
crates: [packages, apps/viewer, apps/editor, ugoe-motion, docs, spec, ugoe-model, ugoe-wasm, ugoe-cli]
depends_on: [T0636, T0703]
assignee: claude-worker-T0645
---

# T0645: トラッキング: MediaPipe の Pose Landmarker で体の向きと肩を取り込む

## 目的

利用者の要望。顔のトラッキング (Face Landmarker) に Pose Landmarker を組み合わせ、体の向き (`BodyAngleX/Y/Z`) と肩の上下を取り込む。
アバタープロファイルの仕様は「体の向きは、体のトラッキングがあればその回転、なければ頭の回転 × 0.3」としており、その「体のトラッキング」を作る。

## 受け入れ条件

- [x] `packages/face-tracking` に Pose Landmarker を足す (モデルはビルドのときに取得して SHA-256 を検証。リポジトリにはコミットしない。既定は軽いモデル)。顔と体の推定を同じフレームで行い、体のトラッキングは画面で切れる
- [x] 体の点から `BodyAngleX` (左右の回転)・`BodyAngleY` (前後の傾き)・`BodyAngleZ` (肩の線の傾き) を求める式と、肩の上下 (すくめ) のパラメータ。肩のパラメータはアバタープロファイルの拡張のパラメータとして定める (ID・値域・意味)。頭の角度から体の回転を引くかどうか (体ごと回ったときに頭が二重に回らないように) を決める
- [x] 肩が映っていない・信頼度が低いときは、今の「頭の回転 × 0.3」に戻る。平滑化・キャリブレーション・記録と再生 (T0703) に体の値も含める
- [x] アバタープロファイルの仕様 (`docs/spec/avatar-profile.md` と英語の `spec/avatar-profile.md`) のトラッキングの入力・式・拡張のパラメータを更新する (`just spec` が通る)。Rust 側の対応 (ugoe-motion) に体の入力が要るなら足す
- [x] ビューアとエディタのプレビューで動く。記録した値の列でのテスト、ヘッドレスのブラウザでの確認。操作マニュアルとサードパーティのライセンスの表記を更新する
- [x] `just ci`・`just licenses`・エディタとビューアのビルドが通る

## 参考

- `docs/tasks/T0703-viewer-face-tracking.md`・`docs/tasks/T0636-editor-preview-tracking.md` のメモ
- `docs/spec/avatar-profile.md`

## メモ

### 決めたこと

- **頭の角度から体の回転を引かない。** `AngleX` などはカメラに対する頭の向きのまま (体のトラッキングの有無で意味が変わらない)。
  理由: 体のトラッキングが無いときの規則 (頭そのまま + 体は頭 × 0.3) がもともと「カメラに対する頭の向き」で、モデルもそれに
  合わせて作られている。単眼の Pose のヨーは画像によって 10〜15° ずれ、引くとその誤差が最も目立つ `AngleX` に乗る。
  `BodyAngle*` の値域 (±10) は狭く、重なりは小さい。体に対する角度がほしいモデルは上書きで作れる (仕様に例)。
  ARKit・MediaPipe の顔の変換もカメラに対する向きなので、そのまま使える。
- **肩のパラメータ:** アバタープロファイルの「体の拡張パラメータ」`ShoulderLY` / `ShoulderRY` (−1〜1、既定 0、正ですくめる。
  `ugoe_model::standard::BODY_EXTENDED`、`TRACKED_PARAMETERS` に追加)。どのレベルにも必要なく、レベル 3 の 16 個にも数えない。
  入力が無ければ値が無い (書き込まない。実装は NaN = 仕様の「有限でない値は書き込まない」と同じ扱い) ので、顔だけの
  トラッキングで肩のモーションを潰さない。
- **profile_version は上げない (0 のまま)。** 追加は省略できる入力とどのレベルにも要らないパラメータだけで、知らないアプリは
  動かさないだけ・既存のモデルの読み方も変わらないため。英語版の Stability と Versions の文を「どのレベルにも要らない追加は
  版の中で足すことがある」に合わせた (日本語版はもともとそう書いていた)。
- **体の入力は軸ごとに省略できる** (`ugoe_motion::BodyInput`。`TrackingInput.body` を `Option<Rotation>` から変えた)。
  机の前では腰が映らずピッチだけ無いことが多いため。省略した軸は頭 × 0.3 (T0306 の規則のまま)。
- **MediaPipe の点からの式** (`packages/face-tracking/src/pose.ts`、詳細は `docs/spec/avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker`):
  ロールは画像の肩の線の傾き (`worldLandmarks` のロールは正面の画像で 14° 傾くなど不安定だった)、ヨーは `worldLandmarks` の
  肩の奥行き、ピッチは腰→肩の向き (腰が見えるときだけ)、肩の上下は耳の中点と肩の線の距離 ÷ 肩幅 (ヨーで補正) の基準からの減り。
  左右の肩を別々には求めない (片方の肩だけの動きは肩の点だけでは体の傾きと区別できないので、ロールに任せ、両肩に同じ値)。
  ピッチと肩はカメラの高さ・体格で基準が大きく変わる (正面の画像でピッチ −15〜−20°) ので、キャリブレーションした後だけ渡す。
  見えているかは点の組ごとに visibility のヒステリシス (0.6 で見え始め、0.4 未満で見えない)。
- **モデルは lite だけを同梱** (float16 版 1、5.5 MiB、SHA-256 `59929e1d…`)。full (9.0 MiB)・heavy (29.2 MiB) の URL とハッシュは
  スクリプトのコメントに書いた。試した画像では full の方がヨーの左右反転の一貫性がよかったが、heavy はかえってロールが崩れた。
  .deb は 13 MiB 前後 → 18.65 MiB。
- 体のトラッキングは既定で入れる (パネルの「体 (肩) も」/「Body (shoulders)」で切れる)。顔が見つからないフレームでは体も使わない。
- 記録の形式を版 2 にした (`body`。値の無いキーは省く。版 1 も読める)。
- `setPuppetTracking(puppet, frame)` をパッケージに置き、ビューアの Player とエディタの PreviewMode の `set_tracking` の呼び出しを
  それに置き換えた (エディタの変更はこの 1 か所と stub の型だけ)。

### 範囲外の変更 (team-lead の承認済み。crates: に足した)

- `ugoe-model`: 肩の ID と値域を検証ツール・対応付けが知る必要があるため。`standard::SHOULDER_L_Y` / `SHOULDER_R_Y` / `BODY_EXTENDED`、
  `range()` と `TRACKED_PARAMETERS` への追加、検査のテスト 1 つ。
- `ugoe-wasm`: 体の入力を JS から渡すため。`WebPuppet.set_tracking` に省略できる体の引数 5 つ (`Option<f32>`)。
- `ugoe-cli`: `ugoe standard-params` の一覧に体の拡張パラメータを出すため (2 行とテスト 1 行)。
- 回帰の確認: 体の入力が無いときの値が以前の式と同じ (`ugoe-motion`)、見本のアバター (`assets/samples/avatar.ugoe`) の見た目が
  体の入力なし・肩の入力だけのときで同じ (`crates/ugoe/tests/m3_puppet.rs`)。見本のファイル自体は変えていない。

### 確かめたこと

- Rust: 軸ごとの体の入力と頭 × 0.3 への戻り、肩は入力があるときだけ書き込む (無い入力を読む上書きも書き込まない)、
  値域のクランプ、頭の角度は体を引かない (`ugoe-motion` のテスト)。wasm の入力の組み立て。肩はレベル 3 に数えない (`check`)。
- TS (Vitest 40 件): 合成した点でロール・ヨー・ピッチ・肩の式と符号、見えない部分の扱いとヒステリシス、背中向きの除外、
  キャリブレーション (ピッチと肩は後だけ)・平均・平滑化 (途切れたら忘れる)・左右の入れ替え・記録の版 2 と版 1 の読み込み。
- ヘッドレスの Chrome (SwiftShader) で、ビルドしたビューア (`?model=` に肩のパラメータを足した見本のモデル) とエディタの
  プレビュー (本物の `Document` への使い捨ての HTTP ブリッジ) に画像を入れた。素材はリポジトリの外: MediaPipe の試験用の
  `portrait.jpg`・`pose.jpg`、Wikimedia Commons の写真 (体をひねった座像 (PD)、サッカー選手 (CC BY 2.0。顔が小さく、アプリでは顔が見つからないので体も使わなかった)、肩をすくめた人 (CC BY-SA 4.0) など)。
  - 正面の portrait: 体のロール −2°、時計回りに 15° 回した画像で +14〜+16° (`BodyAngleZ` は 10 にクランプ)。左右反転でヨーの
    符号が変わる (−3° → +10°。lite では画像によって 10〜15° ずれる)。体を右へひねった座像で −68° (`BodyAngleX` −10)。
  - キャリブレーション後に `BodyAngleY`・`ShoulderLY`・`ShoulderRY` が書き込まれる (前は書き込まれない)。
  - 体の値を含む記録 (合成、2 秒) の再生で `BodyAngleX`・`BodyAngleZ`・肩がモデルに付き、「Stop」で既定値に戻る。
  - 画面に体の点と線 (青) が出る。スクリーンショット (リポジトリの外): `scratchpad/pose/shots/viewer-portrait-rot15.png`・
    `editor-preview-body.png`・`editor-tracking-panel.png`・`editor-preview-replay.png`。
- 重さ: 同じ画像の検出の頻度 (SwiftShader = CPU の WebGL) が、体あり 6 fps・体なし 14 fps。実機の GPU では測っていない。
- `just ci`・`just wasm-check`・`just spec`・ビューアとエディタの `pnpm build` (モデルの取得と検証を含む)・`cargo check -p ugoe-editor -p ugoe-viewer`・
  `just viewer-build --bundles deb` が通る。

### 確かめていないこと・利用者に確かめてほしいこと

- **実際のカメラ** (この環境には無い): 「開始」→ 肩まで映る位置で、体を左右にひねる (`BodyAngleX` が自分の左で正)・体を傾ける
  (`BodyAngleZ`、画面で時計回りが正)・「キャリブレーション」の後に肩をすくめる (`ShoulderLY`/`RY` が 1 に近づく)・椅子で後ろに
  反る (`BodyAngleY` が正。腰が映るときだけ)。
- 肩の係数 (中立 0.62・すくめたときの減り 0.2) は、肩をすくめた写真で中立との差がはっきり出ず、見積もりのまま。実際のカメラで
  すくめたときの値 (パネルの「Body」の shoulderLeft) が 1 前後になるか。小さすぎ・大きすぎなら `pose.ts` の `SHOULDER_RANGE` を直す。
- 体のピッチの符号 (`worldLandmarks` の z の向きは公式文書の「小さいほどカメラに近い」に従った。画像では確かめられなかった)。
- 実機の GPU での検出の頻度。Tauri のウィンドウ (WebKitGTK・WebView2・WKWebView) での Pose Landmarker (顔と同じく canvas を渡している)。
- 体のトラッキングが見えなくなった・戻ったときに、体の角度が頭 × 0.3 との間で跳ぶ (平滑化はその値のフィルタを忘れるので追わない)。
  気になるなら途切れのフェードを入れる。

### 後続の候補

- 途切れたとき・戻ったときのフェード (顔と体)。full モデルの選択肢。
- 画面の「頭から体の回転を差し引く」の切り替え (上の判断)。
- 見本のモデル (T0702) に肩のパラメータとキーフォームを足す。
- 片方の肩だけの上下 (腰が見えるときは背骨の傾きで体の傾きと分けられる)。
