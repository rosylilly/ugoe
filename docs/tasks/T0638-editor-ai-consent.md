---
id: T0638
title: エディタ: AI の API キーの保存と送信の同意
status: done
milestone: M6
crates: [apps/editor, ugoe-editor-core, docs, Cargo.toml]
depends_on: [T0509]
assignee: claude-worker-T0638
---

# T0638: エディタ: AI の API キーの保存と送信の同意

## 目的

T0509 の `ClaudeProvider` をエディタから安全に使えるようにする。API キーの保存と、画像を外部に送る前の同意の流れを作る。

## 受け入れ条件

- [x] API キーを OS の資格情報の保存先に保存・削除でき、`ApiKey::new` で渡す。キーは画面・ログ・プロジェクトに残らない
- [x] 送る前に `Provider::disclosure` (送り先・モデル・画像の枚数とバイト数) を示し、同意を求める。「今後は確かめない」の扱いを決める
- [x] プロジェクトごとに「外部に送らない」を選べ、選んだプロジェクトでは AI の機能を無効にする
- [x] ヘッドレスのブラウザで流れを確かめる
- [x] `just ci`・`just gen`・エディタのビルドが通る

## 参考

- `docs/tasks/T0509-claude-provider.md` のメモ
- `docs/ai-integration.md#セキュリティとプライバシー`

## メモ

- やったこと
  - バックエンド `apps/editor/src-tauri/src/ai.rs`: API キーの保存先 (`ApiKeys`。OS の資格情報の保存先 → 環境変数
    `ANTHROPIC_API_KEY` の順に読み、`ApiKey::new` に渡す)、送る前の確認 (`check_consent`・`prepare_claude`・`AiReply`)、
    IPC コマンド 6 つ (`ai_settings`・`store_api_key`・`delete_api_key`・`set_ai_send_externally`・`forget_ai_consent`・
    `test_ai_connection`)。保存先の操作は D-Bus の応答待ちで画面を止めないよう `spawn_blocking` で行う
  - プロジェクトごとの「外部に送らない」: `ProjectMeta::ai_send_externally` (`project.json` の `editor.ai.send_externally`。
    既定のときは書かないので既存の `project.json` は変わらず、そのまま読める)。変えると未保存の変更になる
    (`ProjectSession` に `meta_modified` を足した)
  - 画面 `apps/editor/src/lib/ai/`: 設定の画面 (ヘッダーの「Settings」)、確認のダイアログ、`withConsent`・`consentPrompt`・
    `aiSettings` (T0637 が使う呼び出し口)。「Test connection」が今の唯一の呼び出し元
  - 「今後は確かめない」の扱い: プロジェクトを開いている間だけ、同じ送り先 (サービス・URL・モデル) に限る。ファイルには残さない
    (`docs/editor.md#ai-の設定と送信の同意`)
- 範囲外の変更 (team-lead の承認済み。`crates:` に `Cargo.toml` を足した)
  - ルートの `Cargo.toml` に `keyring-core`・`dbus-secret-service-keyring-store` (`crypto-rust`)・`apple-native-keyring-store`
    (`keychain`)・`windows-native-keyring-store` (既定の機能なし) を追加。エディタだけが OS ごとに依存する。
    理由: Linux の保存先は同期の D-Bus で非同期ランタイムが要らず (dbus は tao 経由で既にある)、暗号は Rust 実装なので
    OpenSSL (システムのライブラリ) が要らない。保存先が使えない環境では環境変数を使い、普通のファイルには保存しない
  - `just licenses` は通る (すべて MIT OR Apache-2.0)
- 確かめたこと
  - `just ci` と `just gen` が通る。エディタは `cargo check -p ugoe-editor` と `vite build` で確かめた (`tauri build` の束ねまではしていない)
  - `cargo xwin check --target x86_64-pc-windows-msvc -p ugoe-editor` が通る。macOS のターゲットは入っていないので未確認
  - ヘッドレス Chrome (ビルドした画面 + リポジトリ外の使い捨ての HTTP ブリッジ。AI のコマンドは `ai.rs` と同じ規則を JS で真似た
    もので、実際の保存先・API には触れない): キーの保存 (入力欄が空になり、ページの HTML にキーがない)・削除、確認のダイアログ
    (Cancel で送らない・規約のリンク・Send・「今後は確かめない」で次は確かめない・「Ask again」)、「外部に送らない」で
    Test connection が無効になりプロジェクトが未保存になる、保存先が使えないとき (入力欄を出さず環境変数を使う)
  - 実際のキーチェーン (Secret Service・macOS・Windows) での保存・読み出しと、実際の API への接続の確認は利用者が行う
    (この環境は WSL で保存先がない)
- 後続の候補
  - T0637: 提案の IPC コマンドで `prepare_claude` と `withConsent` を使う。AI のボタンは `aiSettings.canSend` で無効にする
  - モデルや `effort` の選択を設定の画面に出す (今は `ClaudeConfig::default()`)
