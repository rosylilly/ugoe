---
id: T1665
title: "エディタ: 重い負荷の下で Secret Service の呼び出しが 2 秒で諦め、API キーの保存・読み出しが失敗するのを直す"
status: done
milestone: M10
crates: [apps/editor, tests, docs]
depends_on: []
assignee: claude-worker-T1665
---

# T1665: エディタ: 重い負荷の下で Secret Service の呼び出しが 2 秒で諦め、API キーの保存・読み出しが失敗するのを直す

## 目的

T0050 で e2e に鍵の保管庫 (gnome-keyring) を足したところ、重い負荷の下 (並行のビルドで load average 20 ほど) で API キーの保存が
「DBus error: Did not receive a reply」で 1 回失敗した。dbus-secret-service 4.1 は Secret Service の呼び出しを 2 秒で諦める
(`proxy/mod.rs` の `Duration::from_millis(2000)`)。e2e の筋書きは、この失敗のときだけ知らせを閉じて 1 回やり直している。
Linux の利用者も、重いときに保存・読み出し・削除が失敗し得る。

## 受け入れ条件

- [x] 時間の上限を延ばせるか (dbus-secret-service-keyring-store・dbus-secret-service の口や feature) を調べ、延ばすか、
      「返事が無い」失敗のときだけ 1 回 (か上限のある回数) やり直すかを決めた。理由と調べた版をメモに書いた
- [x] 保存・読み出し・削除のすべての経路 (`apps/editor/src-tauri/src/ai.rs` の保管庫の trait の実装) に同じ扱いをした。やり直しは Linux の
      Secret Service だけで、ほかの OS の保管庫の振る舞いは変えない。呼び出しはメインのスレッドを止めない (async のコマンドのまま)
- [x] Rust のテスト: 偽の保管庫で、1 回目が「返事が無い」で失敗し 2 回目が通ると成功し、続けて失敗すると今のエラーのコードを返す。ほかの失敗はやり直さない
- [x] tests/e2e の筋書きの「1 回やり直す」を外せるなら外し、外さないなら理由を書いた。`just e2e --only editor` が通る
- [x] docs/editor.md (AI の設定・API キーの保存) に書いた。`just ci` が通る

## 参考

- T0050 のメモ (分かったこと・起票の候補)、tests/e2e/README.md の「鍵の保管庫」
- apps/editor/src-tauri/src/ai.rs、ルートの Cargo.toml (`dbus-secret-service-keyring-store`・`keyring-core`)

## メモ

- T0052 で起票した (元の候補: T0050 (Secret Service の 2 秒の上限))。
- 新しい依存や feature を足すなら PR の本文に理由を書く (AGENTS.md)。
- 調べたこと (Cargo.lock の版。どれも crates.io の最新 (2026-10-07 に API で確かめた)):
  - dbus-secret-service 4.1.0: `src/proxy/mod.rs` の `new_proxy` が `Duration::from_millis(2000)` を決め打ちし、すべての呼び出し
    (`OpenSession`・`SearchItems`・`CreateItem`・`Delete` など) がこれを使う。変える関数・feature は無い (`connect_with_max_prompt_timeout` は
    解錠の確認を待つ秒数で、呼び出しの上限ではない)。出典: https://github.com/brotskydotcom/dbus-secret-service/blob/v4.1.0/src/proxy/mod.rs
  - dbus-secret-service-keyring-store 1.0.1: `Store::new_with_configuration` は空でない設定を `NotSupportedByStore` で断る (`src/store.rs`)。
    失敗は `src/errors.rs` の `decode_error` で `Locked`・`NoResult`・`Prompt` が `NoStorageAccess`、ほか (D-Bus の失敗を含む) が
    `PlatformFailure(Box<dbus_secret_service::Error>)` になる。出典: https://github.com/open-source-cooperative/dbus-secret-service-keyring-store
  - 時間切れの D-Bus のエラーの名前は `org.freedesktop.DBus.Error.NoReply`: 遅い D-Bus のサービス (python-dbus で 3 秒眠る) を
    `dbus-send --reply-timeout=500` (dbus クレートと同じ libdbus) で呼んで確かめた。
- 決めたこと: 時間の上限は延ばせない (フォークや zbus の secret-service への載せ替えは重い) ので、`NoReply` の失敗のときだけ、合わせて 3 回まで試す
  (`ai::with_retry`。`KEYRING_ATTEMPTS = 3`。最悪で約 6 秒待つ)。保存は `CreateItem` の置き換え、削除は無ければ成功なので、
  前の呼び出しが遅れて済んでいても結果は変わらない。
- やったこと:
  - `apps/editor/src-tauri/src/ai.rs`: `KeyringStore` の開く (`platform_store`)・読み出し・保存・削除をすべて `with_retry` に通す。
    `is_no_reply` は Linux だけで、`PlatformFailure` の原因の連なりから `dbus::Error` を探して名前が `NoReply` かを見る。ほかの OS は常に
    `false` (やり直さない)。IPC コマンドは前のとおり async + `blocking` (別のスレッド)。
  - 依存: ルートの `[workspace.dependencies]` に `dbus = "0.9"` と `dbus-secret-service = "4"` を足し (範囲の外の小さな変更)、エディタの Linux の
    依存に `dbus`、Linux の dev-dependencies に `dbus-secret-service` (テストで本物と同じ形のエラーを作るため)。どちらも前から Cargo.lock に
    あった推移的な依存 (MIT/Apache-2.0)。PR の本文に理由を書くこと: 時間切れの失敗をエラーの型 (`dbus::Error::name`) で見分けるため。
  - テスト (ai_tests.rs、Linux だけ): 偽の `CredentialStoreApi` (失敗を順に返す) を `KeyringStore` で包み、1 回目の `NoReply` の後は
    保存・読み出し・削除が通る、3 回続くと `ai.credential_store` を返す、ほかの失敗 (`ServiceUnknown`・施錠・確認を閉じた) は 1 回で返す。
    `KEYRING_ATTEMPTS = 1` にすると 1 つ目のテストが落ちることを確かめた。
  - 本物で確かめた (コミットしていない一時のテスト): 使い捨ての D-Bus のセッション・`XDG_RUNTIME_DIR`・`XDG_DATA_HOME` で gnome-keyring を動かし、
    保存の直前に `kill -STOP` で 3 秒止めると保存は約 3.0 秒で成功し、8 秒止めると約 6.0 秒で今のエラー (`Did not receive a reply`) を返した。
    利用者の保管庫 (`~/.local/share/keyrings`) の更新の時刻は変わっていない。
  - e2e の筋書き (`tests/e2e/specs/editor.test.mjs`) の「1 回やり直す」を外した (エディタがやり直すため)。`just e2e --only editor` は
    load average 14 ほどで 16 件すべて通った (鍵の保管庫の筋書きも飛ばさずに通った)。
  - docs/editor.md (API キーの保存先) に書いた。
- `just ci` と `just cross-check` (Windows の clippy) が通った。
- 確かめていないこと: CI (ubuntu-24.04) の e2e。macOS・Windows の実機 (やり直しは cfg で Linux だけ。振る舞いは変えていない)。
- 起票の候補:
  - 開くとき (`ApiKeys::store` の `OnceLock`) の失敗は、3 回の `NoReply` でもアプリを閉じるまで「使えない」のまま覚える。一時的な失敗なら
    次に使うときに開き直すか (今は開くのもやり直すので起きにくい)。
