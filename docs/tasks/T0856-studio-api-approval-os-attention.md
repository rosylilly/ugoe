---
id: T0856
title: "スタジオ: 外部 API の承認を待つアプリがあることを、操作ウィンドウが隠れていても OS の側で知らせる (判断が要る)"
status: done
milestone: M8
crates: [apps/studio, docs, Cargo.toml, Cargo.lock]
depends_on: []
assignee: claude-worker-T0856
---

# T0856: スタジオ: 承認の OS への知らせ (判断が要る)

## 目的

外部 API (T0805) に初めて接続したクライアントは、操作ウィンドウの上の知らせ (`ApiApproval.svelte`) で利用者が承認するまで応答を
待たされる。配信中は操作ウィンドウを最小化したり裏に置いたりすることが多く、Stream Deck などから接続しても知らせに気づかない。
操作ウィンドウが隠れているときは、OS の側でも知らせる。

**判断が要る** (新しい依存を入れるか)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. 依存を足さず、ウィンドウの注意を引く:** Tauri の `WebviewWindow::request_user_attention(Some(UserAttentionType::Critical))`
  (Windows はタスクバーの点滅、macOS は Dock のアイコンの跳ね、Linux はウィンドウマネージャの「注意を要する」の印)。
  承認・拒否・後で、で止める (`None`)。文は出せないが、どの OS でも追加の依存なしで動く。
- **B. `tauri-plugin-notification` で OS の通知を出す:** 「<アプリの名前> が操作 API への接続を求めています」の文を出せる。
  新しい依存 (tauri-plugin-notification と、Linux の notify-rust (D-Bus)・macOS の mac-notification-sys・Windows の
  tauri-winrt-notification。ライセンスは MIT か Apache-2.0 との二重の見込みで、`just licenses` と `third-party-notices.txt` で確かめる)。
  通知の権限 (macOS は初回に許可を求める) と、capabilities の許可が要る。デスクトップでは通知のボタン (承認・拒否) は使えないので、
  押すと操作ウィンドウを前に出すだけ。
- **C. A と B の両方:** 通知は設定で切れるようにする。

### 推奨

A から始める。依存も権限も増えず、Stream Deck のボタンを押した直後の「何か待っている」は点滅か跳ねで十分に伝わる。
文で知らせたいという要望があれば B を足す (C)。承認そのものは今までどおり操作ウィンドウでだけ行う (OS の通知から承認はしない。
誤って押したときの取り消しが操作ウィンドウにあるため)。

## 受け入れ条件

- [x] 利用者に案を示して方針を決め、決めたことと理由を docs/studio.md#外部-api に書いた
- [x] 承認を待つクライアントが現れたとき、操作ウィンドウが隠れている (最小化・前面に無い) なら決めた方法で知らせ、承認・拒否・後で・
      切断のどれかで待ちが無くなったら止める。操作ウィンドウが前面にあるときは知らせない。どの場合に知らせるかの判断はテストで確かめた
- [ ] B か C なら: 依存をルートの `[workspace.dependencies]` に足し、`just licenses` と同梱のライセンスの表記 (`third-party-notices.txt`。
      docs/release.md#同梱物のライセンスの表記) を揃え、PR の本文に依存を入れた理由を書いた。通知の文は @ugoe/i18n の日英で出す
- [x] 実機 (Windows・macOS・Linux の X11 と Wayland) での見え方を確かめる手順を docs/manual-check.md に足した
      (確かめられないものは docs/studio.md#実機で確かめていないこと に書く)
- [x] `just ci` が通る

## 参考

- T0805 のメモ (後続の候補)、docs/studio.md#外部-api・#操作ウィンドウ (承認の知らせ)
- apps/studio/src-tauri/src/api/ (承認の待ち)、apps/studio/src/ApiApproval.svelte
- Tauri の `request_user_attention`: <https://docs.rs/tauri/2/tauri/webview/struct.WebviewWindow.html#method.request_user_attention>

## メモ

- T0075 で起票した (元: docs/roadmap.md の M8 の「起票していない後続の候補」の「承認を OS の通知にも出す (T0805)」)。
- T0855 の後にする: どちらもスタジオの `src-tauri/Cargo.toml` と Cargo.lock を触りうるため (A を選ぶなら依存は増えず、この順は要らない)。
- 判断 (2026-10-08): 利用者が案 A (Tauri の request_user_attention でウィンドウの注意を引く。依存を足さない) を選んだ。
- 案 A は依存を足さないので、T0855 との Cargo.toml の重なりが無くなり、依存を外した (2026-10-08)。
- やったこと (2026-10-08):
  - `apps/studio/src-tauri/src/attention.rs` (新規): いつ注意を引き・止めるかの決め方 (`ApprovalAttention`)。承認を待つ ID の集まりに
    新しいものが現れ、操作ウィンドウがフォーカスを持っていなければ引く (`Critical`)。待ちが無くなったら止め、操作ウィンドウがフォーカスを
    得たら止める (「後で」は前に出してから押すので、ここで止まる)。前から待っているクライアントでは引き直さない。
  - 前面にあるかは操作ウィンドウの `WindowEvent::Focused` で覚える (起動時は `is_focused` で初期化)。`send_api_view` のたびにウィンドウに
    尋ねると、メインのスレッドへの往復を待つことになり、ほかのロックを握った呼び手 (ブラウザソースの配信など) と行き違いうるため。
    `request_user_attention` はメインのスレッドへ送るだけで待たない。
  - `api/mod.rs` の `send_api_view` (承認の待ちが変わる口はすべてここを通る) と `control_focused` から `request_control_attention` を呼ぶ。
  - テスト: `attention.rs` の 4 つ (前面のとき引かない・同じクライアントで引き直さない・待ちが無くなったら止める・前に出たら止める) と、
    `api/tests.rs` の `waiting_clients_draw_attention_until_nothing_waits` (本物の名乗り・承認・拒否・切断の口を通す)。
  - 文書: docs/studio.md#外部-api (決めたことと理由・動き)・#実機で確かめていないこと、docs/manual-check.md の S31、docs/manual/studio.md、
    apps/studio/README.md。docs/roadmap.md の M8 の後続 (と M7 の T0191) の「判断が要る」を決めた形に直した (T0191・T0852・T0855・T0856・T0860・T0861)。
  - `just ci` と `just e2e --only studio` (13 件) が通った。Windows 向けの cfg は触っていないので `just cross-check` は流していない。
- 確かめていないこと: 実機の見え方 (Windows の点滅・macOS の跳ね・Linux の X11 の urgency hint・Wayland で何か出るか)。WSL の上では
  Tauri の実物を前面・背面にして試せない。docs/manual-check.md の S31 に手順を書いた。GTK 3 の urgency hint は X11 のものなので、
  Wayland では何も出ない見込み (docs/studio.md#実機で確かめていないこと)。
- やらなかったこと: OS の通知 (案 B・C) は利用者の判断で見送り。
- 起票の候補: Wayland でも知らせたいなら、xdg-activation (GTK 4 か直接の Wayland のプロトコル) か案 B の通知を足す (S31 の結果を見てから)。
