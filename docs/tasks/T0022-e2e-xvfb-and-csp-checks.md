---
id: T0022
title: "e2e: Xvfb の番号の衝突をなくし、3 つのアプリで CSP の違反が無いことを確かめる"
status: done
milestone: M9
crates: [tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0022
---

# T0022: e2e: Xvfb の番号の衝突をなくし、3 つのアプリで CSP の違反が無いことを確かめる

## 目的

- **Xvfb の番号 (T0820):** `tests/e2e/run.mjs` は決まった番号で Xvfb を起動するので、同じ機械で並行に流すと
  `SocketCreateListener failed` で起動できない (M9 の並列のワーカーで起きた。T0820 は自分で `Xvfb :157` を立てて `--headed` で流した)。
  空いている番号を選ぶ (`Xvfb -displayfd` で番号を受け取るなど)。
- **CSP の違反 (T0686):** 3 つのアプリは CSP を持つようになった (エディタ T0686・ビューア T0727・スタジオ T0820) が、e2e は
  CSP が止めたもの (`securitypolicyviolation`) を集めていない。止まっても画面の一部が静かに動かなくなるだけで気づけない。
  共通の手順として、各アプリの筋書きの終わりに違反が 0 件であることを確かめる。

## 受け入れ条件

- [x] `run.mjs` が空いている Xvfb の番号を使う。2 つの `just e2e` を同時に流して両方が起動することを確かめた
- [x] 3 つのアプリの筋書きで `securitypolicyviolation` を集め、最後に 0 件であることを確かめる共通の補助がある。CSP を壊す (例: `script-src` から `'wasm-unsafe-eval'` を外す) と失敗することを確かめた
- [x] tests/e2e の README (または docs/testing.md) に書いた
- [x] `just e2e` と `just ci` が通る

## 参考

- T0820・T0686 のメモ (起票の候補)
- tests/e2e/run.mjs・tests/e2e/specs/

## メモ

- T0019 で起票した (元の候補: T0820 の Xvfb、T0686 の CSP の違反)。
- T0022 (やったこと):
  - Xvfb: `run.mjs` は、ロックファイル・`/tmp/.X11-unix` のソケット・抽象ソケット (`/proc/net/unix`) のどれかがある番号を飛ばし、
    探し始める番号をプロセスごとにずらし (99 + pid % 400)、Xvfb が起動できなければ次の番号で試し直す (20 回まで)。
    2 つの `just e2e --no-build --only viewer,studio` を同時に流して両方が起動し通ること (:104 と :105)、同じ番号 (:700) から探す
    3 つの Xvfb の起動を同時に行って :700・:701・:702 に分かれることを確かめた。
  - CSP: `lib/harness.mjs` のページに入れる仕掛けが、`securitypolicyviolation` のイベント・`ReportingObserver` の `csp-violation`
    (buffered)・"Content Security Policy" を含む例外と console を `window.__E2E_CSP__` に集める。`checkNoCspViolations(app)` を
    3 つの筋書きの最後の手順にし、画面に出た CSP のエラーの文も数えて 0 件を確かめる。スタジオのわざと `new Function` を止めさせる手順は、
    止まったことを `takeCspViolations` で確かめてから取り除く。console.json に CSP の違反も書く。
  - 分かったこと: WebKitGTK 2.52 は wasm のコンパイルを CSP で止めても `securitypolicyviolation` を出さず (報告も無い)、`CompileError` を
    投げるだけ。エディタはそれを起動の直後 (仕掛けを入れる前) に起こしてビューポートの HUD に出すだけなので、画面の文も数えるようにし、
    エディタの筋書きに「ビューポートが描ける」を足した。
  - 壊した CSP: 設定のファイルは書き換えず、`tauri build --config` で `script-src 'self'` (`'wasm-unsafe-eval'` なし) にしたビルドで、
    エディタ・ビューア・スタジオとも `has no CSP violations` が失敗することを確かめた (ビューア・スタジオは例外から、エディタは画面の文から)。
  - 文書: tests/e2e/README.md (「Xvfb の番号」「CSP の違反」)、docs/testing.md。
  - `just e2e` (3 つとも通る) と `just ci` が通った。
- 確かめていないこと・気づいたこと:
  - `ReportingObserver` の buffered が WebKitGTK で CSP の報告を返すかは確かめていない (wasm では届かなかった。`eval` はイベントか報告のどちらかで届いた)。
  - 見逃すもの: 起動の直後に起き、画面にも出ないもの・アプリが握りつぶしたもの・Worker の中の違反。
  - 他のワーカーと並行してビルドした直後の 1 回、エディタの「opens the sample project」が見本を開けず (20 秒の待ちで時間切れ) 失敗した。
    その後の 3 回は通った。負荷の下の揺らぎと見ているが、原因は調べていない。
- 起票の候補: e2e のエディタの筋書きにプレビューのトラッキング (画像で MediaPipe の wasm とモデルを読む) を足す (T0686 の候補の残り。
  MediaPipe の wasm は Worker でなく主スレッドで読むなら今回の仕掛けで違反が拾える)。
