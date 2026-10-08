---
id: T0856
title: "スタジオ: 外部 API の承認を待つアプリがあることを、操作ウィンドウが隠れていても OS の側で知らせる (判断が要る)"
status: in-progress
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

- [ ] 利用者に案を示して方針を決め、決めたことと理由を docs/studio.md#外部-api に書いた
- [ ] 承認を待つクライアントが現れたとき、操作ウィンドウが隠れている (最小化・前面に無い) なら決めた方法で知らせ、承認・拒否・後で・
      切断のどれかで待ちが無くなったら止める。操作ウィンドウが前面にあるときは知らせない。どの場合に知らせるかの判断はテストで確かめた
- [ ] B か C なら: 依存をルートの `[workspace.dependencies]` に足し、`just licenses` と同梱のライセンスの表記 (`third-party-notices.txt`。
      docs/release.md#同梱物のライセンスの表記) を揃え、PR の本文に依存を入れた理由を書いた。通知の文は @ugoe/i18n の日英で出す
- [ ] 実機 (Windows・macOS・Linux の X11 と Wayland) での見え方を確かめる手順を docs/manual-check.md に足した
      (確かめられないものは docs/studio.md#実機で確かめていないこと に書く)
- [ ] `just ci` が通る

## 参考

- T0805 のメモ (後続の候補)、docs/studio.md#外部-api・#操作ウィンドウ (承認の知らせ)
- apps/studio/src-tauri/src/api/ (承認の待ち)、apps/studio/src/ApiApproval.svelte
- Tauri の `request_user_attention`: <https://docs.rs/tauri/2/tauri/webview/struct.WebviewWindow.html#method.request_user_attention>

## メモ

- T0075 で起票した (元: docs/roadmap.md の M8 の「起票していない後続の候補」の「承認を OS の通知にも出す (T0805)」)。
- T0855 の後にする: どちらもスタジオの `src-tauri/Cargo.toml` と Cargo.lock を触りうるため (A を選ぶなら依存は増えず、この順は要らない)。
- 判断 (2026-10-08): 利用者が案 A (Tauri の request_user_attention でウィンドウの注意を引く。依存を足さない) を選んだ。
- 案 A は依存を足さないので、T0855 との Cargo.toml の重なりが無くなり、依存を外した (2026-10-08)。
