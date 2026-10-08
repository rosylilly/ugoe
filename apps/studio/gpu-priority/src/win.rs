//! Windows: WDDM の GPU のスケジューラの、プロセスごとの優先度のクラス (`D3DKMTSetProcessSchedulingPriorityClass`)。
//!
//! 出典:
//! - <https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/nf-d3dkmthk-d3dkmtsetprocessschedulingpriorityclass>
//!   (プロセスのハンドルと優先度のクラスを受け、`NTSTATUS` を返す。gdi32.dll。Windows Vista から)
//! - <https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess>
//!
//! ハンドルに要るアクセス権は文書に無い。CPU の優先度 (`SetPriorityClass`) と同じく `PROCESS_SET_INFORMATION` で開く
//! (足りなければ `STATUS_ACCESS_DENIED` が返り、[`Error::AccessDenied`] になる)。

use windows::Wdk::Graphics::Direct3D::{
    D3DKMT_SCHEDULINGPRIORITYCLASS, D3DKMTSetProcessSchedulingPriorityClass,
};
use windows::Win32::Foundation::{
    CloseHandle, E_ACCESSDENIED, E_INVALIDARG, HANDLE, NTSTATUS, STATUS_ACCESS_DENIED,
    STATUS_PRIVILEGE_NOT_HELD,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_SET_INFORMATION};

use crate::{Error, GpuPriority};

/// スタジオのプロセス (仮想カメラのネイティブの wgpu) の GPU の優先度を変える。
pub fn set_current_process_priority(priority: GpuPriority) -> Result<(), Error> {
    // SAFETY: 引数を取らず、閉じなくてよい疑似ハンドル (今のプロセスを指す) を返すだけ。
    let process = unsafe { GetCurrentProcess() };
    set_priority(process, priority)
}

/// 番号 `pid` のプロセスの GPU の優先度を変える (WebView2 の GPU のプロセス)。
pub fn set_process_priority(pid: u32, priority: GpuPriority) -> Result<(), Error> {
    // SAFETY: 返ったハンドルは `Process` が閉じる。番号が無い・終わったプロセスなら失敗が返るだけ。
    let handle = unsafe { OpenProcess(PROCESS_SET_INFORMATION, false, pid) }.map_err(|e| {
        match e.code() {
            E_ACCESSDENIED => Error::AccessDenied,
            // 無い番号は ERROR_INVALID_PARAMETER (HRESULT にすると E_INVALIDARG と同じ値)
            E_INVALIDARG => Error::ProcessNotFound,
            _ => Error::Other(e.message()),
        }
    })?;
    let process = Process(handle);
    set_priority(process.0, priority)
}

/// 開いたプロセスのハンドル (落とすと閉じる)。
struct Process(HANDLE);

impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: `OpenProcess` が返した、まだ閉じていないハンドル (ほかでは閉じない)。
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn set_priority(process: HANDLE, priority: GpuPriority) -> Result<(), Error> {
    // SAFETY: `process` は今のプロセスの疑似ハンドルか、呼び出し元が開いたまま持っているハンドル。値は列挙の範囲の中
    // (`GpuPriority::class`)。
    let status = unsafe {
        D3DKMTSetProcessSchedulingPriorityClass(
            process,
            D3DKMT_SCHEDULINGPRIORITYCLASS(priority.class()),
        )
    };
    status_result(status)
}

fn status_result(status: NTSTATUS) -> Result<(), Error> {
    match status {
        s if s.is_ok() => Ok(()),
        STATUS_ACCESS_DENIED | STATUS_PRIVILEGE_NOT_HELD => Err(Error::AccessDenied),
        s => Err(Error::Other(format!(
            "NTSTATUS {:#010x}: {}",
            s.0,
            s.to_hresult().message()
        ))),
    }
}

/// WebView2 の GPU のプロセス (出力ウィンドウと操作ウィンドウの描画)。
///
/// どの関数も、WebView2 を作ったスレッド (Tauri のメインのスレッド。`with_webview` の中) で呼ぶ。
/// 出典: <https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environment8>、
/// <https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model>
pub mod webview2 {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PROCESS_KIND, COREWEBVIEW2_PROCESS_KIND_GPU, ICoreWebView2Environment8,
    };
    use webview2_com::ProcessInfosChangedEventHandler;
    use windows::Win32::Foundation::E_NOINTERFACE;
    use windows::core::Interface;

    pub use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment;

    use crate::Error;

    /// 一覧から読むプロセスの数の上限 (WebView2 の環境のプロセスは、ブラウザ・GPU・ユーティリティ・描画のプロセスなど数個)。
    const MAX_PROCESSES: u32 = 256;

    /// WebView2 の環境 (データの場所ごと) の GPU のプロセスの番号 (ふつうは 1 つ。GPU を使っていなければ空)。
    pub fn gpu_process_ids(environment: &ICoreWebView2Environment) -> Result<Vec<u32>, Error> {
        gpu_ids(&environment8(environment)?)
    }

    /// WebView2 の環境のプロセスが変わる (GPU のプロセスが作り直されるなど) たびに、GPU のプロセスの番号で `on_change` を呼ぶ。
    /// 返すのは今の GPU のプロセスの番号。`on_change` は WebView2 を作ったスレッドで呼ばれる。登録はアプリの終わりまで外さない
    /// (環境もアプリの終わりまで残る)。
    pub fn watch_gpu_processes(
        environment: &ICoreWebView2Environment,
        mut on_change: impl FnMut(Result<Vec<u32>, Error>) + 'static,
    ) -> Result<Vec<u32>, Error> {
        let environment = environment8(environment)?;
        let handler = ProcessInfosChangedEventHandler::create(Box::new(move |sender, _| {
            if let Some(sender) = sender {
                on_change(environment8(&sender).and_then(|e| gpu_ids(&e)));
            }
            Ok(())
        }));
        let mut token = 0_i64;
        // SAFETY: `environment` と `handler` は生きている COM のインターフェース。`token` は書ける i64。WebView2 は `handler` の
        // 参照を数えて持つ。
        unsafe { environment.add_ProcessInfosChanged(&handler, &mut token) }
            .map_err(|e| Error::Other(e.message()))?;
        gpu_ids(&environment)
    }

    fn environment8(
        environment: &ICoreWebView2Environment,
    ) -> Result<ICoreWebView2Environment8, Error> {
        environment.cast().map_err(|e| {
            if e.code() == E_NOINTERFACE {
                Error::WebView2TooOld
            } else {
                Error::Other(e.message())
            }
        })
    }

    fn gpu_ids(environment: &ICoreWebView2Environment8) -> Result<Vec<u32>, Error> {
        let other = |e: windows::core::Error| Error::Other(e.message());
        // SAFETY: `environment` は生きている COM のインターフェース。
        let infos = unsafe { environment.GetProcessInfos() }.map_err(other)?;
        let mut count = 0_u32;
        // SAFETY: `infos` は生きている COM のインターフェースで、`count` は書ける u32。
        unsafe { infos.Count(&mut count) }.map_err(other)?;
        let mut ids = Vec::new();
        for index in 0..count.min(MAX_PROCESSES) {
            // SAFETY: `index` は数えた個数より小さい。
            let info = unsafe { infos.GetValueAtIndex(index) }.map_err(other)?;
            let mut kind = COREWEBVIEW2_PROCESS_KIND::default();
            // SAFETY: `info` は生きている COM のインターフェースで、`kind` は書ける列挙の値。
            unsafe { info.Kind(&mut kind) }.map_err(other)?;
            if kind != COREWEBVIEW2_PROCESS_KIND_GPU {
                continue;
            }
            let mut pid = 0_i32;
            // SAFETY: `info` は生きている COM のインターフェースで、`pid` は書ける i32。
            unsafe { info.ProcessId(&mut pid) }.map_err(other)?;
            if let Ok(pid) = u32::try_from(pid)
                && pid != 0
            {
                ids.push(pid);
            }
        }
        Ok(ids)
    }
}
