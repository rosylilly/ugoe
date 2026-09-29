//! PNG の描画。`ugoe render` (ugoe-cli) を子プロセスとして呼ぶ。
//!
//! 描画の規則 (大きさ・背景・パラメータのクランプ・テクスチャの扱い) を `ugoe render` と完全に
//! 同じにするため、描画のコードを写さずに CLI を使う。GPU のデバイスをサーバのプロセスに持たない
//! ので、GPU のドライバの不調 (`crates/ugoe-cli/README.md` の WSL2 の注意) がサーバを落とさない。
//! 今のドキュメントを一時的な `.ugoe` バンドルに書き出し、それを描かせる。
//!
//! 実行ファイルは設定で変えられる ([`find_ugoe_bin`])。見つからないときは探した場所と直し方を
//! エラーで示し、版がこのサーバと違うときは描画の結果に警告を付ける。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use ugoe_editor_core::{Project, ProjectError, export_bundle};

/// 描画の指定。値は `ugoe render` のオプションにそのまま渡す。
#[derive(Debug, Clone, Default)]
pub struct RenderRequest {
    /// パラメータの ID と値。
    pub params: Vec<(String, f32)>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub background: Option<String>,
    pub motion: Option<String>,
    pub expression: Option<String>,
    pub time: Option<f32>,
    /// PNG を書き出す先。`None` なら一時ファイルに書いて、読んだら消す。
    pub output: Option<PathBuf>,
}

/// 描画の結果。
#[derive(Debug)]
pub struct Rendered {
    pub png: Vec<u8>,
    /// `ugoe render` の標準出力と、標準エラーの `warning: ` の行 (GPU のドライバの雑多な出力は除く)。
    pub log: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("failed to export the project as a bundle: {0}")]
    Export(#[from] ProjectError),
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: std::io::Error,
    },
    #[error(
        "`ugoe` (ugoe-cli) was not found at `{bin}`. Build it next to ugoe-mcp \
         (`cargo build -p ugoe-cli -p ugoe-mcp` or `cargo install --path crates/ugoe-cli`), \
         or pass --ugoe-bin <PATH> / set UGOE_BIN"
    )]
    NotFound { bin: String },
    #[error("`{bin}` failed: {stderr}")]
    Failed { bin: String, stderr: String },
}

/// `ugoe` の実行ファイルを探す: 明示した指定 (`--ugoe-bin` / `UGOE_BIN`)、このサーバと同じ
/// ディレクトリの `ugoe` (cargo で一緒にビルド・インストールした場合)、`PATH` の `ugoe` の順。
pub fn find_ugoe_bin(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    if let Some(path) = std::env::var_os("UGOE_BIN") {
        return PathBuf::from(path);
    }
    let name = format!("ugoe{}", std::env::consts::EXE_SUFFIX);
    if let Some(sibling) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&name)))
        .filter(|p| p.is_file())
    {
        return sibling;
    }
    PathBuf::from(name)
}

/// 一時ファイルの名前が重ならないようにする番号。
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_path(ext: &str) -> PathBuf {
    let n = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("ugoe-mcp-{}-{n}.{ext}", std::process::id()))
}

/// 破棄するときにファイルを消す。
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// `project` を `ugoe render` で描く。
pub fn render(bin: &Path, project: &Project, req: &RenderRequest) -> Result<Rendered, RenderError> {
    let bundle = TempFile(temp_path("ugoe"));
    std::fs::write(&bundle.0, export_bundle(project)?).map_err(|source| RenderError::Io {
        context: format!("failed to write {}", bundle.0.display()),
        source,
    })?;
    let temp_output;
    let output = match &req.output {
        Some(path) => path.clone(),
        None => {
            temp_output = TempFile(temp_path("png"));
            temp_output.0.clone()
        }
    };

    // 値は `--name=value` の形で渡し、`-` で始まる値がオプションと解釈されないようにする。
    let mut cmd = Command::new(bin);
    cmd.arg("render").arg(&bundle.0).arg("-o").arg(&output);
    for (id, value) in &req.params {
        cmd.arg(format!("--param={id}={value}"));
    }
    if let Some(w) = req.width {
        cmd.arg(format!("--width={w}"));
    }
    if let Some(h) = req.height {
        cmd.arg(format!("--height={h}"));
    }
    if let Some(bg) = &req.background {
        cmd.arg(format!("--background={bg}"));
    }
    if let Some(motion) = &req.motion {
        cmd.arg(format!("--motion={motion}"));
    }
    if let Some(expression) = &req.expression {
        cmd.arg(format!("--expression={expression}"));
    }
    if let Some(time) = req.time {
        cmd.arg(format!("--time={time}"));
    }
    let out = cmd.output().map_err(|source| spawn_error(bin, source))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(RenderError::Failed {
            bin: bin.display().to_string(),
            stderr: stderr.trim().to_owned(),
        });
    }
    let png = std::fs::read(&output).map_err(|source| RenderError::Io {
        context: format!("failed to read {}", output.display()),
        source,
    })?;
    let log = stdout
        .lines()
        .chain(stderr.lines().filter(|l| l.starts_with("warning: ")))
        .map(str::to_owned)
        .chain(version_warning(bin))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Rendered { png, log })
}

fn spawn_error(bin: &Path, source: std::io::Error) -> RenderError {
    if source.kind() == std::io::ErrorKind::NotFound {
        RenderError::NotFound {
            bin: bin.display().to_string(),
        }
    } else {
        RenderError::Io {
            context: format!("failed to run `{}`", bin.display()),
            source,
        }
    }
}

/// `ugoe --version` がこのサーバの版と違えば警告を返す (違う版の CLI は、今のモデルの形を
/// 読めないことがある)。版を読めなければ何も言わない (描画は成功しているので)。
fn version_warning(bin: &Path) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let version = text.split_whitespace().nth(1)?;
    let ours = env!("CARGO_PKG_VERSION");
    (version != ours).then(|| {
        format!(
            "warning: `{}` is version {version} but ugoe-mcp is {ours}; \
             build both from the same checkout",
            bin.display()
        )
    })
}
