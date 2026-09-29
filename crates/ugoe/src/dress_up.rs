//! 着せ替え: [`Puppet`] へのモジュールの付け外し (`docs/spec/modules.md#puppet-での着せ替え`)。
//!
//! 最初にモジュールを付けたときの Puppet のモデルと物理の設定を「本体」として覚え、付け外しのたびに
//! 本体とモジュールの組を合成 ([`ugoe_runtime::compose`]) してコンパイルし直す。

use std::sync::Arc;

use ugoe_model::Model;
use ugoe_model::module::Module;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::{ValidationError, validate_physics};
use ugoe_runtime::{
    CompileError, CompiledModel, ComposeError, ModelInstance, ParamIndex, ParameterValues, compose,
    compose_physics,
};

use crate::Puppet;

/// 合成したモデルのテクスチャ (`Model::textures` の 1 つ) の出どころ。
///
/// レンダラには、この出どころの画像を合成したモデルのテクスチャの番号で渡す
/// (`ugoe_render::Renderer::set_texture` / `set_normal_map`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureSource {
    /// 本体の `Model::textures` の添字 (バンドルの画像と同じ順)。
    Base(u32),
    /// モジュール `module` の `Module::textures` の添字 (`ugoe_format::module_file::ModuleFile::textures` と同じ順)。
    Module { module: String, index: u32 },
}

/// 付け外しで変わったもの。レンダラの利用者は、これを見てテクスチャを設定し直す。
///
/// メッシュの並びなどモデルの形は変わるが、レンダラは別のコンパイル済みモデルを渡されると
/// 自分で作り直すので、利用者がすることはテクスチャの設定だけ。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DressUpChange {
    /// 出どころ ([`TextureSource`]) が変わったか、新しく増えたテクスチャの番号 (昇順)。
    /// 同じ名前の別のモジュール (`Arc` が違う) に差し替えたときも、そのテクスチャを含む。
    /// この番号のテクスチャ (と法線マップ) を [`Puppet::texture_sources`] に従って設定し直す。
    pub textures: Vec<u32>,
    /// 合成したモデルのテクスチャの数。これ以上の番号のテクスチャはもう使われない (捨ててよい)。
    pub texture_count: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum DressUpError {
    #[error(transparent)]
    Compose(#[from] ComposeError),
    #[error(transparent)]
    Compile(#[from] CompileError),
    /// 合成した物理の設定が、合成したモデルに対する検証を通らない。
    #[error("composed physics failed validation: {0:?}")]
    Physics(Vec<ValidationError>),
    #[error("module `{0}` is not attached")]
    NotAttached(String),
}

/// 着せ替えの状態: 本体と、取り付けているモジュール。
#[derive(Debug, Clone)]
pub(crate) struct Wardrobe {
    base: Model,
    /// 本体の物理の設定 (最初にモジュールを付けたときの設定)。
    base_physics: Option<PhysicsSettings>,
    modules: Vec<Arc<Module>>,
    textures: Vec<TextureSource>,
}

impl Puppet {
    /// 取り付けているモジュール (合成の順)。
    pub fn modules(&self) -> &[Arc<Module>] {
        self.wardrobe.as_ref().map_or(&[], |w| &w.modules)
    }

    /// 本体のモデル (モジュールを付ける前のモデル)。モジュールを付けたことがなければ今のモデル。
    pub fn base_model(&self) -> &Model {
        match &self.wardrobe {
            Some(wardrobe) => &wardrobe.base,
            None => self.instance.compiled().model(),
        }
    }

    /// 合成したモデルのテクスチャごとの出どころ (`Model::textures` と同じ順)。
    /// 本体のテクスチャが先頭に同じ順で並び、モジュールのテクスチャがモジュールの順に続く。
    pub fn texture_sources(&self) -> Vec<TextureSource> {
        match &self.wardrobe {
            Some(wardrobe) => wardrobe.textures.clone(),
            None => texture_sources(self.instance.compiled().model(), &[]),
        }
    }

    /// モジュールを取り付ける。同じスロットか同じ名前のモジュールが付いていれば、それと差し替える
    /// (合成の順の同じ位置に入る)。そうでなければ最後に加える。
    ///
    /// 失敗したときは何も変えない。組み替えの規則は [`Puppet::set_modules`] を参照。
    pub fn attach_module(&mut self, module: Arc<Module>) -> Result<DressUpChange, DressUpError> {
        let mut modules = self.modules().to_vec();
        let same = |m: &Arc<Module>| m.slot == module.slot || m.name == module.name;
        match modules.iter().position(same) {
            Some(i) => {
                modules[i] = Arc::clone(&module);
                let mut k = 0;
                modules.retain(|m| {
                    k += 1;
                    k - 1 == i || !same(m)
                });
            }
            None => modules.push(module),
        }
        self.set_modules(modules)
    }

    /// 名前が `name` のモジュールを外す。付いていなければ [`DressUpError::NotAttached`]。
    pub fn detach_module(&mut self, name: &str) -> Result<DressUpChange, DressUpError> {
        let mut modules = self.modules().to_vec();
        let before = modules.len();
        modules.retain(|m| m.name != name);
        if modules.len() == before {
            return Err(DressUpError::NotAttached(name.to_owned()));
        }
        self.set_modules(modules)
    }

    /// 取り付けるモジュールの組を `modules` にする (空なら本体だけに戻す)。
    ///
    /// 本体とモジュールを合成し (`ugoe_runtime::compose` / `compose_physics`)、コンパイルし直して
    /// モデルを差し替える。失敗したときは何も変えない。差し替えでは次のように状態を扱う
    /// (`docs/spec/modules.md#puppet-での着せ替え`):
    ///
    /// - パラメータの値・基準値は同じ ID のものを引き継ぐ (`ParameterValues::carry_over`)。
    ///   合成はパラメータを変えないので、モーション・表情・目パチ・呼吸・リップシンク・トラッキングは続きから動く。
    /// - ポーズは作り直し、本体のグループの表示中の要素と不透明度を引き継ぐ。
    /// - 物理は合成した設定で作り直し、粒子は次の `update` でその時の入力に対する静止姿勢から始める
    ///   (本体のリグの揺れも止まる)。
    /// - 差し替えた直後に 1 回評価するので、`instance().drawables()` はすぐに新しいモデルのものになる。
    ///
    /// 本体の物理が無くてもモジュールにリグがあれば、重力 `(0, 1)`・風なし・60 Hz の設定に加える。
    pub fn set_modules(
        &mut self,
        modules: Vec<Arc<Module>>,
    ) -> Result<DressUpChange, DressUpError> {
        let wardrobe = &*self.wardrobe.get_or_insert_with(|| {
            let base = self.instance.compiled().model().clone();
            Wardrobe {
                textures: texture_sources(&base, &[]),
                base,
                base_physics: self.physics.as_ref().map(|p| p.settings().clone()),
                modules: Vec::new(),
            }
        });
        let refs: Vec<&Module> = modules.iter().map(|m| m.as_ref()).collect();
        let base = &wardrobe.base;
        let model = compose(base, &refs)?;
        let has_rigs = refs.iter().any(|m| !m.physics.rigs.is_empty());
        let physics = match &wardrobe.base_physics {
            Some(settings) => Some(compose_physics(base, settings, &refs)?),
            None if has_rigs => Some(compose_physics(base, &empty_physics(), &refs)?),
            None => None,
        };
        if let Some(physics) = &physics {
            let errors = validate_physics(physics, &model);
            if !errors.is_empty() {
                return Err(DressUpError::Physics(errors));
            }
        }
        let textures = texture_sources(base, &modules);
        let compiled = CompiledModel::compile(model)?;

        let change = texture_change(
            (&wardrobe.textures, &wardrobe.modules),
            (&textures, &modules),
        );
        self.swap_model(compiled, physics);
        if let Some(wardrobe) = &mut self.wardrobe {
            wardrobe.modules = modules;
            wardrobe.textures = textures;
        }
        Ok(change)
    }

    /// モデルを `compiled` に差し替え、状態を引き継ぐ ([`Puppet::set_modules`] の規則)。
    fn swap_model(&mut self, compiled: Arc<CompiledModel>, physics: Option<PhysicsSettings>) {
        let mut instance = ModelInstance::new(compiled);
        let old = self.instance.parameters();
        instance.parameters_mut().carry_over(old);
        let new = instance.parameters();
        self.base = carry_values(old, &self.base, new);
        self.output = carry_values(old, &self.output, new);
        self.tracking_map = ugoe_motion::TrackingMap::for_model(instance.compiled().model(), new);
        self.instance = instance;

        let previous = self.pose.clone();
        self.set_pose(self.instance.compiled().model().pose.clone());
        self.pose.carry_over(&previous);

        self.replace_physics(physics);
        self.apply_part_opacities(0.0);
        self.instance.update();
    }
}

/// 本体に物理が無いときに、モジュールの物理を加える空の設定。
fn empty_physics() -> PhysicsSettings {
    PhysicsSettings {
        gravity: ugoe_core::Vec2::new(0.0, 1.0),
        wind: ugoe_core::Vec2::ZERO,
        fps: ugoe_physics::DEFAULT_FPS,
        rigs: Vec::new(),
        colliders: Vec::new(),
    }
}

/// `old` の並びの値 `values` を、同じ ID で `new` の並びに写す (値域にクランプする)。
fn carry_values(old: &ParameterValues, values: &[f32], new: &ParameterValues) -> Vec<f32> {
    let mut previous = old.clone();
    for (i, &value) in values.iter().enumerate() {
        previous.set(ParamIndex(i as u32), value);
    }
    let mut next = new.clone();
    next.carry_over(&previous);
    next.as_slice().to_vec()
}

/// 合成の順 (本体、モジュールの順) に並べたテクスチャの出どころ。
fn texture_sources(base: &Model, modules: &[Arc<Module>]) -> Vec<TextureSource> {
    let base = (0..base.textures.len() as u32).map(TextureSource::Base);
    let modules = modules.iter().flat_map(|m| {
        (0..m.textures.len() as u32).map(|index| TextureSource::Module {
            module: m.name.clone(),
            index,
        })
    });
    base.chain(modules).collect()
}

/// 付け外しの前後のテクスチャの出どころを比べる。同じ名前でも別のモジュール (`Arc` が違う) に
/// 差し替えたなら、画像が違いうるので変わったものとして数える。
fn texture_change(
    (before, before_modules): (&[TextureSource], &[Arc<Module>]),
    (after, after_modules): (&[TextureSource], &[Arc<Module>]),
) -> DressUpChange {
    let find = |modules: &[Arc<Module>], name: &str| {
        modules.iter().find(|m| m.name == name).map(Arc::as_ptr)
    };
    let same = |i: usize, source: &TextureSource| {
        before.get(i) == Some(source)
            && match source {
                TextureSource::Base(_) => true,
                TextureSource::Module { module, .. } => {
                    find(before_modules, module) == find(after_modules, module)
                }
            }
    };
    let textures = after
        .iter()
        .enumerate()
        .filter(|&(i, source)| !same(i, source))
        .map(|(i, _)| i as u32)
        .collect();
    DressUpChange {
        textures,
        texture_count: after.len() as u32,
    }
}

#[cfg(test)]
mod tests;
