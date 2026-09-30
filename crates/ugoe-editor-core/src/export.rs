//! エディタからの `.ugoe`・`.ugoemod` の書き出し: プロジェクトに残す書き出しの設定と、アトラス化を含む書き出し。
//! 設計: `docs/editor.md#書き出し`、アトラス化の規則: `docs/spec/file-formats.md#テクスチャのアトラス化`。

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_model::Model;
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::atlas::{AtlasError, AtlasOptions, AtlasReport, build_module_atlas};
use crate::project::{
    ModuleExport, Project, ProjectError, ProjectMeta, checked_module, export_bundle, module_file,
};

/// `project.json` の `editor` の中で、書き出しの設定を置くキー。
const EXPORT_SETTINGS_KEY: &str = "export";

/// アトラス 1 枚の大きさの上限として選べる最大値 (px)。
pub const MAX_ATLAS_SIZE: u32 = 16384;
/// 周りに複製する画素の幅として選べる最大値 (px)。
pub const MAX_ATLAS_PADDING: u32 = 64;

/// 書き出しの設定。`project.json` の `editor.export` に残す (すべて既定値なら書かない)。
/// 足りないフィールドは既定値で読む。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct ExportSettings {
    /// テクスチャをアトラスにまとめるか。既定は `false`。
    pub atlas: bool,
    /// アトラス 1 枚の幅と高さの上限 (px、1〜[`MAX_ATLAS_SIZE`])。
    pub atlas_max_size: u32,
    /// 各画像の周りに複製する端の画素の幅 (px、0〜[`MAX_ATLAS_PADDING`])。
    pub atlas_padding: u32,
}

impl Default for ExportSettings {
    fn default() -> Self {
        let atlas = AtlasOptions::default();
        Self {
            atlas: false,
            atlas_max_size: atlas.max_size,
            atlas_padding: atlas.padding,
        }
    }
}

impl ExportSettings {
    /// 範囲を検査する。
    pub fn validate(&self) -> Result<(), ExportError> {
        if !(1..=MAX_ATLAS_SIZE).contains(&self.atlas_max_size) {
            return Err(ExportError::InvalidMaxSize(self.atlas_max_size));
        }
        if self.atlas_padding > MAX_ATLAS_PADDING {
            return Err(ExportError::InvalidPadding(self.atlas_padding));
        }
        Ok(())
    }

    /// アトラス化するなら、その設定。
    pub fn atlas_options(&self) -> Option<AtlasOptions> {
        self.atlas.then_some(AtlasOptions {
            max_size: self.atlas_max_size,
            padding: self.atlas_padding,
        })
    }
}

impl ProjectMeta {
    /// プロジェクトに残した書き出しの設定。ない・読めなければ既定値。
    pub fn export_settings(&self) -> ExportSettings {
        self.editor
            .get(EXPORT_SETTINGS_KEY)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }

    /// 書き出しの設定を残す。既定値なら `editor.export` を消す (既存のプロジェクトの `project.json` を変えないため)。
    /// 設定が変わったら `true`。
    pub fn set_export_settings(&mut self, settings: ExportSettings) -> Result<bool, ExportError> {
        settings.validate()?;
        let changed = self.export_settings() != settings;
        if settings == ExportSettings::default() {
            self.editor.remove(EXPORT_SETTINGS_KEY);
        } else {
            let value = serde_json::to_value(settings).map_err(ExportError::Settings)?;
            self.editor.insert(EXPORT_SETTINGS_KEY.to_owned(), value);
        }
        Ok(changed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("atlas max size must be between 1 and {MAX_ATLAS_SIZE} (got {0})")]
    InvalidMaxSize(u32),
    #[error("atlas padding must be at most {MAX_ATLAS_PADDING} (got {0})")]
    InvalidPadding(u32),
    #[error("export settings: {0}")]
    Settings(#[source] serde_json::Error),
    #[error(transparent)]
    Atlas(#[from] AtlasError),
    #[error(transparent)]
    Project(#[from] ProjectError),
}

/// 既定のパラメータで 1 フレームを描くときの、描画の見積もり。
///
/// 参照レンダラの本描画のまとめ方 (`docs/spec/rendering.md#描画の最適化`) を、テクスチャ・合成モード・
/// カリングの有無だけで真似る。マスクアトラスの違い (GPU の上での配置で決まる) は数えないので、
/// マスクを使うモデルでは `ugoe render --stats` の `state_changes` より少なく出ることがある。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
pub struct DrawEstimate {
    /// テクスチャの枚数 (`Model::textures`)。
    pub textures: u32,
    /// 描く要素の数 (表示されていて三角形を持つメッシュ)。
    pub drawables: u32,
    /// まとめた後の描画の回数。
    pub draw_calls: u32,
    /// 前の描画から描画の状態を切り替える回数 (最初の描画は数えない)。
    pub state_changes: u32,
}

/// `model` を既定のパラメータで描くときの描画の回数と状態の切り替えを見積もる。
/// モデルをコンパイルできなければ、宣言順で、マスク専用でないメッシュをすべて描くとみなす。
pub fn estimate_draws(model: &Model) -> DrawEstimate {
    // (メッシュの番号, 合成モード) を描画順に。
    let draws: Vec<(usize, ugoe_model::BlendMode)> = match CompiledModel::compile(model.clone()) {
        Ok(compiled) => {
            let mut instance = ModelInstance::new(compiled);
            instance.update();
            let drawables = instance.drawables();
            instance
                .render_order()
                .iter()
                .map(|&i| i as usize)
                .filter_map(|i| {
                    let state = drawables.get(i)?;
                    state.visible.then_some((i, state.blend))
                })
                .collect()
        }
        Err(_) => model
            .meshes
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.mask_only)
            .map(|(i, m)| (i, m.blend))
            .collect(),
    };
    let mut estimate = DrawEstimate {
        textures: count(model.textures.len()),
        ..DrawEstimate::default()
    };
    let mut last = None;
    for (i, blend) in draws {
        let Some(mesh) = model.meshes.get(i) else {
            continue;
        };
        if mesh.indices.is_empty() {
            continue;
        }
        estimate.drawables += 1;
        let state = (mesh.texture, blend, mesh.culling);
        if last != Some(state) {
            if last.is_some() {
                estimate.state_changes += 1;
            }
            estimate.draw_calls += 1;
            last = Some(state);
        }
    }
    estimate
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// 書き出しの結果の要約 (エディタが書き出しの後に示す)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ExportReport {
    /// 書き出したバンドルの大きさ (バイト)。
    pub bytes: usize,
    /// アトラス化する前のモデルの見積もり。
    pub before: DrawEstimate,
    /// 書き出したモデルの見積もり (アトラス化しなければ `before` と同じ)。
    pub after: DrawEstimate,
    /// アトラス化したときの結果。しなければ `null`。
    pub atlas: Option<AtlasReport>,
}

/// `settings` に従ってプロジェクトを `.ugoe` バンドルのバイト列に書き出す (アトラス化は任意)。
/// プロジェクト自体は変えない。
pub fn export_bundle_with_settings(
    project: &Project,
    settings: &ExportSettings,
) -> Result<(Vec<u8>, ExportReport), ExportError> {
    settings.validate()?;
    let before = estimate_draws(&project.model);
    let (bytes, after, atlas) = match settings.atlas_options() {
        Some(options) => {
            let (atlased, report) = project.with_atlas(&options)?;
            (
                export_bundle(&atlased)?,
                estimate_draws(&atlased.model),
                Some(report),
            )
        }
        None => (export_bundle(project)?, before, None),
    };
    let report = ExportReport {
        bytes: bytes.len(),
        before,
        after,
        atlas,
    };
    Ok((bytes, report))
}

/// モジュールのプロジェクトを `settings` に従って `.ugoemod` のバイト列に書き出す (アトラス化は任意。
/// [`crate::atlas::build_module_atlas`] で、モジュールのテクスチャだけをまとめる)。書き出しの検査は
/// [`crate::export_module`] と同じ。プロジェクト自体は変えない。
pub fn export_module_with_settings(
    project: &Project,
    settings: &ExportSettings,
) -> Result<ModuleExport, ExportError> {
    settings.validate()?;
    let (module, images, warnings) = checked_module(project, None)?;
    let (module, images, atlas) = match settings.atlas_options() {
        Some(options) => {
            let atlased = build_module_atlas(&module, &images, &options)?;
            (atlased.module, atlased.textures, Some(atlased.report))
        }
        None => (module, images, None),
    };
    let file = module_file(module, &images)?;
    let bytes = ugoe_format::write_module(&file).map_err(ProjectError::Export)?;
    Ok(ModuleExport {
        bytes,
        warnings,
        atlas,
    })
}
