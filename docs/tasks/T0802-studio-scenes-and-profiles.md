---
id: T0802
title: スタジオ: シーンとプロファイル (モデル・着せ替え・ライティング・配置・背景の保存と切り替え)
status: done
milestone: M8
crates: [apps/studio, packages/player, docs]
depends_on: [T0801, T0903]
assignee: claude-worker-T0802
---

# T0802: スタジオ: シーンとプロファイル (モデル・着せ替え・ライティング・配置・背景の保存と切り替え)

## 目的

配信の場面ごとの設定を保存して、すぐ切り替えられるようにする。

## 受け入れ条件

- [x] プロファイルのファイル (JSON。版の番号付き) に、シーンの一覧を保存する。シーンはモデル・付けるモジュール・ライティング・位置と大きさと回転・背景 (透過・単色・クロマキー・画像) を持つ
- [x] シーンの追加・複製・並べ替え・切り替え。最近のプロファイル
- [x] 形式を docs/studio.md に書く (モデルのファイル形式ではなく、アプリの設定として)
- [x] `just ci` が通る

## 参考

- docs/studio.md

## メモ

### やったこと (2026-10-02)

- バックエンド (`apps/studio/src-tauri`):
  - `profile.rs`: プロファイルの形式 (`format: "ugoe-studio-profile"`・`version: 1`・シーン・`activeScene`・`output.size`)。読むときは `format` と
    `version` を先に見て、未来の版は中身を読まずに `futureVersion` のエラー。値の範囲を収め、ID の空と重複を付け直す。一時ファイルに書いてから置き換える。
    最近のプロファイル (`app_config_dir` の `recent-profiles.json`、10 個)。配置 (`transform`) は `f64` (f32 の丸めで画面とのやり取りが揺れたため)。
  - `stage.rs`: 舞台 (プロファイル・保存先・配信中のシーン・受け手に出すファイル)。シーンの追加・複製・並べ替え・削除・設定の置き換え・切り替え。
    同じモデル (同じパス) のシーンどうしは読み直さず値を引き継ぐ。読めない・付けられないときは何も変えない (配信中のシーンの編集なら元に戻す)。
    背景の画像だけは読めなければ透過で続ける。エラーは `{code, ...}` (ADR 0029 の 4.)。
  - `conductor.rs`: `present(model?, modules?, look)` と `clear`。シーンの通知に `modelRevision`・`modulesRevision`・モジュールの名前・見せ方を足した。
    同じモデルのまま着せ替えたら次のフレームに物理のリセットの印を載せる (T0801 のフラグ)。
  - `lib.rs`: コマンド (`restore_profile`・`new_profile`・`open_profile`・`save_profile`・`forget_recent_profile`・`add_scene`・`duplicate_scene`・`move_scene`・
    `remove_scene`・`update_scene`・`activate_scene`・`set_output_preset`・`scene_module`・`scene_background`)。`open_model` は消した (「モデルを選ぶ」は
    シーンの設定を変えて切り替える)。プロファイルの通知 `profile` を操作ウィンドウへ。操作ウィンドウに保存のダイアログの権限。
- 操作ウィンドウ: 左にシーンの一覧 (`ScenesPanel.svelte`)、右にシーンの編集 (`SceneEditor.svelte`・`LightingEditor.svelte`)、ツールバーにプロファイルの
  メニュー (新規・開く・保存 Ctrl/⌘+S・名前を付けて保存・最近のプロファイル) と保存の状態。保存先があれば 0.8 秒後に自動で保存、無ければ保存を促し、
  捨てる前に確かめる。モデルが見つからないシーンは「選び直す」の知らせ。出力の大きさはプロファイルへ移した (刻みの速さは T0804 のまま localStorage)。
- 出力ウィンドウ: シーンの通知の版を比べて、読み直す・モジュールを付け直す・見せ方だけを変えるを選ぶ (`lib/outputLayout.ts` の `planScene`)。
  背景は要素の CSS、回転は舞台の CSS (ビューに回転が無いため。回すときだけ対角線の正方形にする)、大きさはズーム、位置はパン。
- `@ugoe/player`: `FrameQueue.continueScene`・`Player.continueScene` (同じモデルのまま次のシーンへ。前のシーンのフレームも捨てずに適用し `seq` の続きを保つ)。
  受け手のモードでは光源と着せ替えの組を覚えず、覚えたものも使わない。
- 文書: docs/studio.md に「操作 (T0802)」「プロファイルの形式」「シーンの切り替えと受け手」と、`scene` の通知の形。apps/studio/README.md・packages/player/README.md。
- テスト: Rust (profile・stage・conductor。版・値の収め方・保存と最近の一覧・同じモデルの切り替え・失敗で何も変えない・背景の代わり)、
  TS (planScene・配置の計算・背景の CSS・エラーの文・FrameQueue.continueScene)。

### 確かめたこと

- `just ci`、`cargo test -p ugoe-studio`、`pnpm --filter @ugoe/studio check/test`、`@ugoe/player` のテスト。
- ヘッドレスの Chrome と使い捨てのブリッジ (本物の Stage と Conductor を HTTP で出す。scratchpad/T0802/bridge) で、操作ウィンドウと出力ウィンドウを
  別のページで開き: モデルを選ぶ → 名前を付けて保存 → 複製して着せ替え (2 つ)・配置・クロマキー → 別のモデル・画像の背景 → 並べ替え → 見せ方だけの変更
  (受け手は読み直さない) → 未来の版のプロファイル (エラー) → モデルが見つからないシーン (選び直す) → 最近のプロファイル。ブリッジを起動し直して
  最後のプロファイルとシーンが戻ること、日本語・ダーク・狭い幅も撮った。スクリーンショットは scratchpad/T0802/shots/。
  何もしていない間に操作ウィンドウがバックエンドへ書き込まないこと (送り合いが止まること) も数えて確かめた。

### 確かめていないこと (利用者の確認に残す)

- 実際の Tauri のウィンドウでのダイアログ (開く・保存)・`app_config_dir` への最近の一覧の保存・Windows と macOS のパス。
- 回転したときの描画の負荷 (舞台を対角線の正方形にするので、1920×1080 なら画素が約 2.3 倍)。

### 範囲外の変更

- `packages/player` (`follower.ts`・`player.svelte.ts`・テスト・README): 受け手が見せ方だけの変更でフレームを捨てない口と、受け手のモードで覚えたものを
  使わない。team-lead に相談済み (返事を待つ間に入れた。不可ならスタジオ側で回避する)。
- `apps/studio/src/Control.svelte` のパラメータのスライダー (T0801): 指揮役の値を表示したときにスライダーが格子に寄せた値で onchange を呼び、
  呼吸などの値を手の値で上書きし続けていたので、手で動かしていないパラメータの半刻み以下の変化を送らないようにした。

### 後続の候補

- `@ugoe/ui` の `Slider`: 外から渡した値が刻みの格子に乗らないと、格子に寄せた値で `onchange` を呼ぶ (利用者が動かしていないのに)。部品の側で直す。
- 仮想カメラ (T0807) とブラウザソース (T0808) の受け手に、シーンの見せ方 (配置・回転・背景・光源) を当てる。仮想カメラはビューに回転が要る
  (`ugoe-render` のビューか、常駐の描画の口で回す)。
- 刻みの速さをプロファイルへ移す (いまは T0804 の localStorage)。ホットキー (T0803) の割り当てをプロファイルに足す。
- ネイティブの受け手 (`follower.rs`) にモジュールの付け直しと `continueScene` 相当を足す (仮想カメラで使うとき)。
- シーンの編集の Undo、ドラッグでの並べ替え、出力ウィンドウ上でのドラッグでの配置。

- 範囲外の変更 (team-lead が承認): packages/player の受け手のモード (continueScene と、受け手では覚えたものを読み書きしない)。ビューアの挙動は変えない。
