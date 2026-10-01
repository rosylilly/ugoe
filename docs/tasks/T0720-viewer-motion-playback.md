---
id: T0720
title: ビューア: モーションと表情を再生する画面
status: done
milestone: M4
crates: [apps/viewer, docs, ugoe-wasm, bindings/web]
depends_on: [T0719]
assignee: claude-worker-T0720
---

# T0720: ビューア: モーションと表情を再生する画面

## 目的

ビューアには、モデルに入っているモーションと表情を再生する画面がない (T0701 のメモの後続の候補、T0719 で見つかった)。見本にモーションと表情が入った (T0719) ので、ビューアから再生できるようにする。

## 受け入れ条件

- [x] モデルのモーションの一覧から再生・停止できる (優先度、ループ、フェード)。表情の一覧から付け外しできる (set と add、すべて止める)
- [x] フェイストラッキングと同時に使ったときの扱い (トラッキングが上書きするパラメータ) を決めて README に書く
- [x] 見本 (avatar・avatar-lit・dress-up) のモーションと表情を、ビューアの「サンプル」から再生できる
- [x] Vitest とヘッドレスのブラウザで確かめる。マニュアルを更新する。`just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0719-samples-with-motions.md`・`docs/tasks/T0701-*.md` のメモ、`crates/ugoe-wasm/README.md` (モーションと表情の API)

## メモ

- **パネル**: `apps/viewer/src/lib/MotionPanel.svelte` (バンドルにモーションか表情があるときだけ出す)。モーションは優先度 (待機・通常・強制) を選んで
  ▶ / ■ と「止める」、名前の下に長さ・ループ・フェード (入り / 抜け)。「繰り返す」はループしないモーションが終わったら同じ優先度で
  始め直す (ビューアだけの機能)。優先度が低くて始まらないときは理由を出し、そのモーションが終わると消す。直近のイベントを 5 つまで出す。
  表情は「これだけ」(set)・「重ねる」(add)・「外す」(名前で stop)・「すべて止める」。再生中のモーションが動かすパラメータのスライダーは「(自動)」。
  一覧の読み取り・使っている表情の求め方・トラッキングとの重なりは `animation.ts` (Vitest 6 件)、状態と操作は `Player`。
- **フェイストラッキングとの併用 (決めたこと)**: ランタイムの適用順のまま (モーション → トラッキング → 表情)。トラッキングが書き込むパラメータでは
  モーションが置き換わり、表情はトラッキングの上に重なる。パネルにその説明と、モーションごとの「トラッキングで隠れる」「一部トラッキング」、
  表情ごとの「トラッキングに重なる」の印を出す。README の「フェイストラッキングと同時に使う」とマニュアル (`docs/manual/motions.md#ビューアで再生する`) に書いた。
- **範囲外の変更 (team-lead の承認済み)**: いまの WebPuppet では表情を重ねられず (set と stop_all だけ)、モーションの長さ・ループ・フェードも取れないので、
  `ugoe-wasm` に `motions_json`・`expressions_json`・`add_expression`・`stop_expression` を足した (README・テスト 2 件)。`bindings/web/stub/ugoe_wasm.d.ts` に
  同じ 4 行を足した (bindings/web の README の規則)。C ABI は変えていない。`docs/spec/motion.md` の JS の操作の一覧にも足した。
- **確かめたこと**: `just ci`・`just wasm-check`・`pnpm --filter @ugoe/runtime check`・`vite build`・`just viewer-build --bundles deb`。
  ヘッドレス Chrome (SwiftShader の WebGL2、`vite preview`) で「サンプル」の アバター・アバター (ライティング)・着せ替え から再生
  (スクリプト `/tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0720/check.mjs`、出力 `run.log`): nod の再生とイベント `nod-end`・待機の idle (ループ) を通常の tilt が置き換え、
  tilt の再生中は待機の idle が始まらない・繰り返し・停止・表情の set → add → stop → set → すべて止める (`MouthForm` などの値で確認)・
  トラッキング (ヨー 20°) と同時に nod と smile (AngleX はトラッキングの 20、MouthForm は smile の 1)・着せ替えで髪を差し替えてもモーションが続く。
  スクリーンショットは `/tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0720/shots/` (`01-avatar-open.png`〜`07-avatar-lit.png`、マニュアル用 `manual-motions-viewer.png` = `docs/manual/images/motions-viewer.png`)。
  読み込み時の 404 が 1 件 (前のタスクと同じもの)。確認に使ったサーバーは止めた。
- **確かめていないこと**: 本物のカメラ・顔でのトラッキングとの併用 (入力は `setTracking` で直接渡した)、Tauri 版の画面 (WebKitGTK)、本物の GPU。
- **後続の候補**: トラッキング中にモーションを優先する (トラッキングを一時的に外す・対象を絞る) 切り替え。待機モーションを自動で流し、ほかのモーションが
  終わったら待機に戻す。WebPuppet が使っている表情の一覧を返す (いまは操作から求めている)。
