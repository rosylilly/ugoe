---
id: T0726
title: ビューアのファイルの読み書きのコマンドを非同期にする
status: done
milestone: M9
crates: [apps/viewer]
depends_on: [T0724]
assignee: claude-worker-T0726
---

# T0726: ビューアのファイルの読み書きのコマンドを非同期にする

## 目的

ビューアのバックエンドのコマンド (`read_model_file`・`write_json_file`・`write_png_file`。`apps/viewer/src-tauri/src/lib.rs`) は
同期の `#[tauri::command]` なので、Tauri のメインスレッドで動く。`read_model_file` は上限 512 MiB までのファイルを読むので、
大きなバンドルを開く・ネットワークのドライブから読む間、ウィンドウ (イベントのループ) が止まる。
コードレビュー (T0724) で見つけた。エディタ (T0685・T0686) とスタジオ (T0819・T0820) の同じ形の所見と揃えて直す。

## 受け入れ条件

- [x] `read_model_file`・`write_json_file`・`write_png_file` が、メインスレッドでファイルを読み書きしない
  (`async` のコマンドにして `tauri::async_runtime::spawn_blocking` で読み書きするなど。エディタ・スタジオと同じ形に揃える)
- [x] 拡張子・大きさ・PNG の検査はいまと同じに働き、テストがある
- [x] `apps/viewer/README.md` の「仕組み」を直す
- [x] `just ci` が通る

## 参考

- T0724 のメモの所見の表
- エディタの T0685・T0686、スタジオの T0819・T0820 (同じ所見)
- https://v2.tauri.app/develop/calling-rust/#async-commands

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
- T0726 (やったこと): 3 つのコマンドを `async fn` にし、読み書きは `lib.rs` の `blocking` (`tauri::async_runtime::spawn_blocking`) に渡した。
  **エディタ (T0685)・スタジオ (T0819) が揃える形:**
  - 本体は同期の関数 (`read_model(path, max)`・`write_json(path, contents)`・`write_png(path, bytes)`) に分け、検査 (拡張子・大きさ・PNG の先頭) もそこに置く。テストはこの関数を直接呼ぶ。
  - コマンドは薄い包み: `#[tauri::command] async fn read_model_file(path: String) -> Result<Response, String> { blocking(move || read_model(Path::new(&path), MAX_FILE_BYTES)).await.map(Response::new) }`。
    引数は所有する型 (`String`) で受け、`move` のクロージャで別のスレッドへ渡す。
  - `blocking` は `async fn blocking<T: Send + 'static>(task: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String>` で、
    `spawn_blocking(task).await` の `JoinError` をエラーの文字列に変える (エディタの `ai.rs` の `blocking` と同じ形。エディタは `AppState` と `IpcResult` を渡す版がすでにある)。
  - 生のバイト列を受けるコマンド (`write_png_file(request: tauri::ipc::Request<'_>)`) も `async` にできる (戻り値が `Result` なら借用の引数を持てる)。
    要求は借りものなので、ヘッダーを解いた後に本文の `Vec<u8>` を `clone` して `blocking` に渡す (写しは 1 回。ディスクへの書き込みに比べて小さい)。
  - テストは `tauri::async_runtime::block_on(command(...))` でコマンドを呼べる。`blocking` が呼び出し元と違うスレッドで動くこともテストした。
- T0726: T0724 の低の所見 (大きさを見てから読む間にファイルが大きくなると上限を超えて読む) も直した。`File::open` → `metadata` で上限を見てから
  `take(max + 1)` で読み、読めたバイト数が上限を超えたら拒む。上限は引数にしてテストで小さい値を渡す。
- T0726 (やらなかったこと): エラーをコードで返す (ADR 0029) のはこのタスクの範囲外なので、いまの英語の文字列のまま。フロントエンドは `invoke` の Promise を待つだけなので変更は要らない。
  実際のウィンドウで大きなファイルを開いて止まらないことは確かめていない (コマンドの単体テストと `just ci` まで)。
- T0726 (起票の候補): なし (一時ファイル経由の保存は T0728 にある)。
