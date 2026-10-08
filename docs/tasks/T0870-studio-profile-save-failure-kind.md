---
id: T0870
title: "スタジオ: 自動の保存の失敗を、書けない場所と一時の失敗 (ディスクが一杯など) に分けるか決める (判断が要る)"
status: done
milestone: M8
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0870
---

# T0870: スタジオ: 自動の保存の失敗を、書けない場所と一時の失敗に分けるか決める (判断が要る)

## 目的

T0866 で、今の保存先への保存に失敗したら、そのプロファイルを書き換えないプロファイル (`ReadOnly::NotWritable`) にして自動の保存を止めるようにした
(`Stage::save_profile`。失敗を 1 度知らせ、以後は写しを保存するか開き直すまで書かない)。失敗の種類は見ていないので、ディスクが一杯・
ネットワークのドライブが一時的に切れた・ほかのアプリがファイルを開いていた (Windows の共有違反) などの一時の失敗でも、そのあと書かなくなる。
空きができても自動の保存は戻らず、利用者は「写しを保存」で同じファイルを選び直す必要がある。失敗の種類で扱いを分けるか決める。

## 判断が要る

今の `ProfileError::Io` は `io::Error` を文字列にして持つので、種類 (`io::ErrorKind`) が残っていない。分けるなら、`Profile::write` の失敗に種類を残す。

- **案 A (推奨): 種類で分ける。** `PermissionDenied`・`ReadOnlyFilesystem` (と、開くときの `profile::is_writable` と同じ判定) だけを
  書き換えないプロファイルにする。それ以外 (`StorageFull`・`QuotaExceeded`・`ResourceBusy`・Windows の共有違反など) は保存先をそのまま残し、
  エラーを知らせて次の自動の保存でもう一度書く。同じ失敗が続く間は知らせを繰り返さない (成功するか種類が変わるまで 1 度だけ)。
- 案 B: 今のまま (どの失敗でも書き換えないプロファイルにする)。写しを保存するか開き直せば戻る。単純で、失敗し続けて知らせが増える心配が無い。
- 案 C: 種類は見ず、失敗の後も自動の保存を続け、連続して N 回 (例: 3 回) 失敗したら書き換えないプロファイルにする。

推奨は A。権限の失敗は何度書いても同じだが、一時の失敗は待てば直るので、自動の保存を止めると利用者が気づかないまま変更を失いやすい。
Windows の共有違反 (`ERROR_SHARING_VIOLATION`) は `io::ErrorKind` では `Other`/`Uncategorized` になりうるので、一時の失敗の側に入れる。

## 受け入れ条件

- [x] 判断をメモに書いた
- [x] (A) 保存の失敗に種類を残し (`ProfileError` の欄か、`Stage` の中だけで使う区別)、権限の失敗だけ書き換えないプロファイルにした。
      一時の失敗では保存先が残り、次の保存で書けたら知らせが消えて自動の保存が続くことを、stage.rs のテスト (失敗を差し込める書き出しの口か、
      Unix の権限を戻す形) で確かめた。同じ失敗が続く間に知らせを繰り返さないことを確かめた (画面の `autosave.ts` のテスト)
- [x] (A) 文言は日英 (`@ugoe/i18n` の検査が通る)。docs/studio.md#書き換えないプロファイル と docs/manual/studio.md を直した
- [x] 画面を変えたら `just e2e --only studio` と `just ui-audit --only studio` が通る
- [x] `just ci` が通る

## 参考

- T0866 のメモ (やらなかったこと)、docs/studio.md#書き換えないプロファイル
- apps/studio/src-tauri の `stage.rs` (`save_profile`)・`profile.rs` (`ProfileError`・`Profile::write`・`is_writable`)、apps/studio/src/lib/autosave.ts
- `ugoe-fs` の `write_atomic` (一時ファイルを経た置き換え。失敗の種類は一時ファイルの作成と置き換えのどちらでも出る)

## メモ

- T0080 で起票した (元の候補: T0866 (一時の保存の失敗でも読み取り専用とみなす。一時の失敗と書けない場所を見分けるか))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- ディスクが一杯・Windows の共有違反は手元で起こしにくい。テストは失敗を差し込む形で確かめ、実機で確かめていないことをメモに書く。
- T0869 (案 A を選んだ場合) と同じ `stage.rs` を触る。並べて進めるなら T0869 の判断の後にする。
- 判断 (2026-10-08): 利用者が案 A (権限の失敗だけを読み取り専用にし、一時の失敗では自動の保存を続ける) を選んだ。
- やったこと (2026-10-08):
  - `profile.rs`: `Profile::write_file` (失敗を `io::Error` のまま返す) と `ProfileError::io`、`is_denied` を足した。
    `PermissionDenied` (Unix の `EACCES`・`EPERM`、Windows の `ERROR_ACCESS_DENIED`) と `ReadOnlyFilesystem` (`EROFS`・`ERROR_WRITE_PROTECT`)
    だけを書けない場所とし、ほかはすべて一時の失敗 (`StorageFull`・`QuotaExceeded`・`ResourceBusy`・Windows の共有違反 (`Uncategorized`) など)。
    出典は Rust 1.98.1 の std の `sys/io/error/{unix,windows}.rs` の対応づけ (docs/studio.md#保存の失敗の種類 に URL)。
  - Windows では、置き換え (`MoveFileExW`) の先をほかのアプリ (ウイルス対策ソフトなど) が開いていても `ERROR_ACCESS_DENIED` になりうる
    (Git の `compat/mingw.c` の `mingw_rename` が同じ理由で試し直している) ので、`PermissionDenied` ならファイルを書く口で開き直して
    確かめる (`cannot_reopen`。開ければ一時の失敗、`PermissionDenied`・`NotFound` なら書けない場所)。`cfg!` で切り替え、関数は
    どの OS でもコンパイルして Linux でもテストする。
  - `Stage::save_profile`: 今の保存先への失敗は、`is_denied` のときだけ `ReadOnly::NotWritable` にする。一時の失敗は保存先と `dirty` を残す。
    テスト用に失敗を差し込む口 (`#[cfg(test)] fail_writes`) を足し、`keeps_saving_after_temporary_failures` で確かめた
    (既存の Unix の権限のテストもそのまま通る)。`profile.rs` に `tells_denied_failures_from_temporary_ones`。
  - 画面: `Autosave` を options の形にし、失敗したら `onError` で知らせ (同じ鍵 = エラーのコードと引数が続く間は 1 度だけ)、30 秒後に
    もう一度保存する。失敗している間は変えても待ち直さない (30 秒に 1 度だけ試す。0.8 秒ごとの保存を繰り返さない)。保存できた・
    手で保存した・写しを保存した・別のプロファイルにしたら `onRecovered` で知らせを消す。保存の通知 (`with_stage` は失敗でも
    `ProfileView` を送る) が保存の失敗より先に届いても、後に届いても同じに動く。`autosave.test.ts` に 3 つのテスト。
  - `Control.svelte`: 自動の保存の失敗は別の赤の帯 (`autosaveError`) に出し、一時の失敗 (書き換えないプロファイルでない) なら
    「変更を残し、少し待ってからもう一度自動で保存する」を添える (`profile.autosaveRetryHint`、日英)。
  - 文書: docs/studio.md (#書き換えないプロファイル の箇条を直し、#保存の失敗の種類 を足した)・docs/manual/studio.md。
  - 検査: `just crate ugoe-studio`・`just cross-check`・`just e2e --only studio` (14 件通過)・`just ui-audit --only studio` (問題 0)・`just ci` が通った。
- 確かめていないこと: 実機の Windows (共有違反・ほかのアプリが開いているファイルへの置き換え・読み取り専用の属性での `cannot_reopen` の判定)・
  macOS・実際にディスクが一杯になったとき・ネットワークのドライブの切断。テストは失敗を差し込む形と Unix の権限で確かめた。
  Windows でフォルダに書けない (ACL) が、ファイル自体は書けるときは、開き直しで一時の失敗とみなす (30 秒ごとに試し、知らせは 1 度)。
  ui-audit の場面に、自動の保存の失敗の帯は入れていない (偽のホストで失敗を起こす場面が無い)。
- 起票の候補:
  - ui-audit の偽のホストに「自動の保存が一時の失敗をする」場面を足し、帯 (`autosaveError`) のコントラストやはみ出しを数える。
  - Windows の実機で、共有違反・ウイルス対策ソフトの置き換えの失敗・読み取り専用の属性の判定を確かめる。
