//! ケースの一覧 (`cases.json`) と、それを展開した計画 (`plan.json`・`plan.txt`)。
//!
//! 計画は各バインディングの実行器が読む。パスは絶対パス、時間を進める `update` の `dt` は列にしてあるので、
//! 実行器は計算をせずにそのとおり呼べばよい (浮動小数点の計算の順番の違いで結果がずれないように)。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, read_json};

/// 画像を比べるときの背景 (`#808080`、不透明)。
///
/// 公開仕様の互換の基準は透明な黒の上で比べるが、ブラウザの canvas の読み戻しはアルファを外して丸めるので、
/// 不透明な背景の上で比べる (Godot の `build.mjs --conformance` と同じ)。
pub const BACKGROUND: [u8; 3] = [128, 128, 128];

/// 時間を進めるときの既定の 1 秒あたりの `update` の回数 (`ugoe render --fps` の既定と同じ)。
pub const DEFAULT_FPS: f32 = 60.0;

/// `cases.json` の中身。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseFile {
    pub cases: Vec<CaseSpec>,
}

/// `cases.json` のケース 1 つ。`spec` を書いたものは、公開仕様の評価の例の `expected.json` の
/// ケースごとに展開する (モデルとパラメータはそこから取る)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseSpec {
    /// ケースの名前 (出力のファイル名になる)。`spec` では省略でき、`spec-<例の名前>-<番号>` になる。
    pub name: Option<String>,
    #[serde(default)]
    pub summary: String,
    /// モデル (`.ugoe` かモデルの JSON)。リポジトリのルートからの相対パス。
    pub model: Option<String>,
    /// [`crate::bundles`] が作るモデルの名前 (`model` の代わり)。
    pub generated: Option<String>,
    /// 公開仕様の評価の例のディレクトリ (`spec/examples/<名前>`)。`model`・`parameters` とは一緒に使えない。
    pub spec: Option<String>,
    /// 付ける着せ替えのモジュール (`.ugoemod`、付ける順)。
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, f32>,
    /// 先に描く値 (計画の [`Case::before`])。
    #[serde(default)]
    pub before: BTreeMap<String, f32>,
    /// 時間を進める秒数 (`ugoe render --time`)。省略すると時間を進めない。
    pub time: Option<f32>,
    pub fps: Option<f32>,
    /// 時間を進めるときに目パチと呼吸を使うか (既定は使わない)。
    #[serde(default)]
    pub effects: bool,
    /// 光源 (描画だけに効く)。
    pub lighting: Option<Lighting>,
    /// 画像も比べるか (既定は比べる)。
    #[serde(default = "yes")]
    pub image: bool,
}

fn yes() -> bool {
    true
}

fn white() -> [f32; 3] {
    [1.0; 3]
}

/// 光源。形は wasm の `WebPuppet::set_lighting_json` と同じ (`crates/ugoe-wasm/src/lighting.rs`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    #[serde(default = "white")]
    pub ambient: [f32; 3],
    #[serde(default)]
    pub lights: Vec<Light>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Light {
    Directional {
        direction: [f32; 3],
        #[serde(default = "white")]
        color: [f32; 3],
    },
    Point {
        position: [f32; 3],
        #[serde(default = "white")]
        color: [f32; 3],
        #[serde(skip_serializing_if = "Option::is_none")]
        range: Option<f32>,
    },
}

/// 計画 (`plan.json`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// 画像の背景 (ストレートアルファの RGB。不透明)。
    pub background: [u8; 3],
    pub cases: Vec<Case>,
}

/// 展開したケース 1 つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub name: String,
    pub summary: String,
    /// モデルの絶対パス。
    pub model: PathBuf,
    /// モデルが `.ugoe` バンドルか (違えばモデルの JSON)。
    pub bundle: bool,
    /// 付けるモジュールの絶対パス (この順に付ける)。
    pub modules: Vec<PathBuf>,
    /// 設定するパラメータ (値域の外はクランプされる)。
    pub parameters: BTreeMap<String, f32>,
    /// 先に描く値。空でなければ、実行器はまずこの値で (数フレーム) 描き、`parameters` に変えた直後のフレームを結果にする。
    /// 値を変えてから描画に反映するまでにフレームの遅れが入りうる実行器 (Godot) で、遅れが無いことを確かめるためのもの。
    /// 結果は `before` が無いときと同じでなければならないので、参照と、同期して描く実行器 (C ABI・wasm) は使わない。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub before: BTreeMap<String, f32>,
    /// 時間を進める秒数 (`ugoe render --time`)。
    pub time: Option<f32>,
    /// 時間を進める `update` の `dt` の列。空なら時間を進めない
    /// (すべての段 (物理・目パチ・呼吸・リップシンク・ポーズ) を止めて 1 回評価する)。
    pub steps: Vec<f32>,
    /// 時間を進めるときに目パチと呼吸を使うか。物理・リップシンク・ポーズは常に使う。
    pub effects: bool,
    pub lighting: Option<Lighting>,
    /// 画像も比べるか。
    pub image: bool,
    /// 参照の画像を作る `ugoe render` の引数 (`-o` を除く)。
    pub render_args: Vec<String>,
}

impl Plan {
    /// `cases.json` を読んで展開する。相対パスは `root` (リポジトリのルート) から、`generated` のモデルは
    /// `generated` のディレクトリ ([`crate::bundles::write_all`] で書いたところ) から解決する。
    pub fn load(cases: &Path, root: &Path, generated: &Path) -> Result<Self, Error> {
        let file: CaseFile = read_json(cases)?;
        let root = root
            .canonicalize()
            .map_err(Error::io(format!("resolving {}", root.display())))?;
        let mut out = Vec::new();
        for spec in file.cases {
            out.extend(expand(spec, &root, generated)?);
        }
        let mut names = std::collections::BTreeSet::new();
        for case in &out {
            if !names.insert(case.name.as_str()) {
                return Err(Error::Invalid(format!(
                    "case `{}` appears twice",
                    case.name
                )));
            }
            if case.name.is_empty()
                || !case
                    .name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(Error::Invalid(format!(
                    "case name `{}` must be ASCII letters, digits, `-` and `_`",
                    case.name
                )));
            }
        }
        Ok(Self {
            background: BACKGROUND,
            cases: out,
        })
    }

    /// C の実行器が読む行の形式 (`tests/conformance/c/runner.c`)。
    ///
    /// ```text
    /// case <名前>
    /// model <bundle|json> <絶対パス>
    /// module <絶対パス>
    /// param <ID> <値>
    /// effects <0|1>
    /// update <dt>
    /// end
    /// ```
    ///
    /// 数は Rust の `f32` の最短の表記で書くので、`strtof` で同じ値に戻る。
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        for case in &self.cases {
            let kind = if case.bundle { "bundle" } else { "json" };
            let _ = writeln!(s, "case {}", case.name);
            let _ = writeln!(s, "model {kind} {}", case.model.display());
            for module in &case.modules {
                let _ = writeln!(s, "module {}", module.display());
            }
            for (id, value) in &case.parameters {
                let _ = writeln!(s, "param {id} {value}");
            }
            let _ = writeln!(s, "effects {}", u8::from(case.effects));
            for dt in &case.steps {
                let _ = writeln!(s, "update {dt}");
            }
            let _ = writeln!(s, "end");
        }
        s
    }
}

fn expand(spec: CaseSpec, root: &Path, generated: &Path) -> Result<Vec<Case>, Error> {
    let Some(example) = &spec.spec else {
        let name = spec
            .name
            .clone()
            .ok_or_else(|| Error::Invalid("a case needs `name` (or `spec`)".to_owned()))?;
        let model = match (&spec.model, &spec.generated) {
            (Some(model), None) => resolve(root, model)?,
            (None, Some(model)) => resolve(generated, &format!("{model}.ugoe"))?,
            _ => {
                return Err(Error::Invalid(format!(
                    "case `{name}` needs one of `model`, `generated` and `spec`"
                )));
            }
        };
        let parameters = spec.parameters.clone();
        return Ok(vec![case(
            name,
            spec.summary.clone(),
            model,
            parameters,
            &spec,
            root,
        )?]);
    };
    if spec.model.is_some() || spec.generated.is_some() || !spec.parameters.is_empty() {
        return Err(Error::Invalid(format!(
            "`spec: {example}` takes the model and the parameters from expected.json"
        )));
    }
    #[derive(Deserialize)]
    struct Expected {
        model: String,
        cases: Vec<ExpectedCase>,
    }
    #[derive(Deserialize)]
    struct ExpectedCase {
        #[serde(default)]
        parameters: BTreeMap<String, f32>,
    }
    let dir = resolve(root, example)?;
    let expected: Expected = read_json(&dir.join("expected.json"))?;
    let model = resolve(&dir, &expected.model)?;
    let base = spec.name.clone().unwrap_or_else(|| {
        let leaf = dir.file_name().map(|n| n.to_string_lossy().into_owned());
        format!("spec-{}", leaf.unwrap_or_default())
    });
    expected
        .cases
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let summary = format!("{example} (case {i})");
            case(
                format!("{base}-{i}"),
                summary,
                model.clone(),
                c.parameters,
                &spec,
                root,
            )
        })
        .collect()
}

fn case(
    name: String,
    summary: String,
    model: PathBuf,
    parameters: BTreeMap<String, f32>,
    spec: &CaseSpec,
    root: &Path,
) -> Result<Case, Error> {
    let modules = spec
        .modules
        .iter()
        .map(|m| resolve(root, m))
        .collect::<Result<Vec<_>, _>>()?;
    let bundle = crate::read(&model)?.starts_with(&ugoe_format::MAGIC);
    if !modules.is_empty() && !bundle {
        return Err(Error::Invalid(format!(
            "case `{name}`: modules need a .ugoe bundle as the model"
        )));
    }
    if let Some(id) = spec.before.keys().find(|id| !parameters.contains_key(*id)) {
        return Err(Error::Invalid(format!(
            "case `{name}`: `before` sets `{id}`, which `parameters` does not"
        )));
    }
    let fps = spec.fps.unwrap_or(DEFAULT_FPS);
    let steps = match spec.time {
        Some(time) => steps(time, fps)
            .ok_or_else(|| Error::Invalid(format!("case `{name}`: invalid time or fps")))?,
        None => Vec::new(),
    };
    let mut case = Case {
        name,
        summary,
        model,
        bundle,
        modules,
        parameters,
        before: spec.before.clone(),
        time: spec.time,
        steps,
        effects: spec.effects,
        lighting: spec.lighting.clone(),
        image: spec.image,
        render_args: Vec::new(),
    };
    case.render_args = render_args(&case, spec.time.map(|t| (t, fps)));
    Ok(case)
}

/// `ugoe render --time` と同じ `update` の列 (`ugoe_headless::animate::advance`)。
/// `1 / fps` 秒を `floor(time * fps)` 回、余りがあればもう 1 回。`time` が 0 なら `update(0)` を 1 回。
pub fn steps(time: f32, fps: f32) -> Option<Vec<f32>> {
    if !(time.is_finite() && time >= 0.0 && fps.is_finite() && fps > 0.0) {
        return None;
    }
    let step = 1.0 / fps;
    let whole = (time * fps + 1e-4).floor();
    let rest = (time - whole * step).max(0.0);
    let mut steps = vec![step; whole as usize];
    if rest > 1e-6 || steps.is_empty() {
        steps.push(rest);
    }
    Some(steps)
}

/// 参照の画像を作る `ugoe render` の引数。
fn render_args(case: &Case, time: Option<(f32, f32)>) -> Vec<String> {
    let mut args = vec![case.model.display().to_string()];
    for module in &case.modules {
        args.extend(["--module".to_owned(), module.display().to_string()]);
    }
    for (id, value) in &case.parameters {
        args.extend(["--param".to_owned(), format!("{id}={value}")]);
    }
    let [r, g, b] = BACKGROUND;
    args.extend(["--background".to_owned(), format!("#{r:02x}{g:02x}{b:02x}")]);
    if let Some((time, fps)) = time {
        args.extend([
            "--time".to_owned(),
            time.to_string(),
            "--fps".to_owned(),
            fps.to_string(),
        ]);
        if !case.effects {
            args.push("--no-effects".to_owned());
        }
    }
    if let Some(lighting) = &case.lighting {
        let [r, g, b] = lighting.ambient;
        args.extend(["--ambient".to_owned(), format!("{r},{g},{b}")]);
        for light in &lighting.lights {
            let spec = match light {
                Light::Directional { direction, color } => {
                    format!("directional:{}:{}", join(direction), join(color))
                }
                Light::Point {
                    position,
                    color,
                    range,
                } => {
                    let range = range.map_or("inf".to_owned(), |r| r.to_string());
                    format!("point:{}:{}:{range}", join(position), join(color))
                }
            };
            args.extend(["--light".to_owned(), spec]);
        }
    }
    args
}

fn join(v: &[f32; 3]) -> String {
    format!("{},{},{}", v[0], v[1], v[2])
}

fn resolve(base: &Path, path: &str) -> Result<PathBuf, Error> {
    let joined = base.join(path);
    joined
        .canonicalize()
        .map_err(Error::io(format!("resolving {}", joined.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_match_ugoe_render() {
        assert_eq!(steps(0.0, 60.0), Some(vec![0.0]));
        assert_eq!(steps(0.05, 60.0).map(|s| s.len()), Some(3));
        let s = steps(0.5, 60.0).unwrap_or_default();
        assert_eq!(s.len(), 30);
        assert!(s.iter().all(|&dt| dt == 1.0 / 60.0));
        // 余りは最後の 1 回
        let s = steps(0.025, 60.0).unwrap_or_default();
        assert_eq!(s.len(), 2);
        assert!((s[1] - (0.025 - 1.0 / 60.0)).abs() < 1e-6);
        assert_eq!(steps(-1.0, 60.0), None);
        assert_eq!(steps(1.0, 0.0), None);
    }

    #[test]
    fn committed_cases_expand() {
        let plan = crate::tests::committed_plan();
        assert!(plan.cases.len() > 10);
        // 公開仕様の評価の例は expected.json のケースごとに展開する
        assert!(plan.cases.iter().any(|c| c.name == "spec-single-quad-0"));
        let text = plan.to_text();
        assert_eq!(text.matches("\ncase ").count() + 1, plan.cases.len());
        assert_eq!(text.matches("\nend\n").count(), plan.cases.len());
    }

    #[test]
    fn render_args_follow_the_case() {
        let root = crate::repo_root();
        let spec: CaseSpec = serde_json::from_value(serde_json::json!({
            "name": "x",
            "model": "assets/samples/avatar.ugoe",
            "parameters": { "AngleX": 10 },
            "time": 0.5,
            "lighting": {
                "ambient": [0.5, 0.5, 0.5],
                "lights": [
                    { "type": "directional", "direction": [1, 0, -1] },
                    { "type": "point", "position": [1, 2, 3], "color": [1, 0.5, 0] }
                ]
            }
        }))
        .unwrap();
        let root = root.canonicalize().unwrap();
        let cases = expand(spec, &root, &root).unwrap();
        let args = &cases[0].render_args;
        let args: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
        assert_eq!(
            args,
            [
                "--param",
                "AngleX=10",
                "--background",
                "#808080",
                "--time",
                "0.5",
                "--fps",
                "60",
                "--no-effects",
                "--ambient",
                "0.5,0.5,0.5",
                "--light",
                "directional:1,0,-1:1,1,1",
                "--light",
                "point:1,2,3:1,0.5,0:inf",
            ]
        );
        assert!(cases[0].bundle);
        assert_eq!(cases[0].steps.len(), 30);
    }
}
