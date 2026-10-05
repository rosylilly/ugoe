//! 着せ替え: [`Puppet`] へのモジュールの付け外し (`docs/spec/modules.md#puppet-での着せ替え`)。
//!
//! 最初にモジュールを付けたときの Puppet のモデルと物理の設定を「本体」として覚え、付け外しのたびに
//! 本体とモジュールの組を合成 ([`ugoe_runtime::compose`]) してコンパイルし直す。本体の物理の設定は
//! [`Puppet::set_base_physics`] で差し替えられる。

use std::sync::Arc;

use ugoe_model::Model;
use ugoe_model::module::Module;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::{ValidationError, describe_errors, validate_physics};
use ugoe_runtime::{
    CompileError, CompiledModel, ComposeError, ModelInstance, ParamIndex, ParameterValues, compose,
    compose_physics, compose_physics_without_base,
};

use crate::{FieldValue, Puppet};

/// 合成したモデルのテクスチャ (`Model::textures` の 1 つ) の出どころ。
///
/// レンダラには、この出どころの画像を合成したモデルのテクスチャの番号で渡す
/// (`ugoe_render::Renderer::set_texture` / `set_normal_map`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureSource {
    /// 本体の `Model::textures` の添字 (バンドルの画像と同じ順)。
    Base(u32),
    /// モジュール `module` の `Module::textures` の添字 (`ugoe_format::module_file::ModuleFile::textures` と同じ順)。
    Module {
        /// モジュールの名前 (`Module::name`)。
        module: String,
        /// モジュールの `Module::textures` の添字。
        index: u32,
    },
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

/// 着せ替え (付け外し・本体の物理の差し替え) の失敗。失敗したときは何も変えていない。
///
/// 組み込み先は [`DressUpError::code`] と [`DressUpError::fields`] で種類と補足の値を読む (`Display` の文は安定した値として扱わない)。
#[derive(Debug, thiserror::Error)]
pub enum DressUpError {
    /// 本体とモジュールを合成できない (スロットが無い・埋まっているなど)。
    #[error(transparent)]
    Compose(#[from] ComposeError),
    /// 合成したモデルをコンパイルできない。
    #[error(transparent)]
    Compile(#[from] CompileError),
    /// 物理の設定が検証を通らない (本体の設定と本体のモデル、または合成した設定と合成したモデル)。
    #[error("{}", describe_errors("physics settings", .0))]
    Physics(Vec<ValidationError>),
    /// 外そうとした名前のモジュールが付いていない。
    #[error("module `{0}` is not attached")]
    NotAttached(String),
}

impl DressUpError {
    /// `.ugoemod` を読めなかったときの種類。付け外しの失敗ではないが、組み込み先 (wasm の `DressUpError`・
    /// C ABI の `ugoe_last_error_kind`) が同じ一覧で扱うので、ここで決める。
    pub const INVALID_FILE: &'static str = "invalid_file";

    /// 機械が読む種類 (`"unknown_slot"`・`"slot_occupied"` など)。wasm の `DressUpError` の `code` と
    /// C ABI の `ugoe_last_error_kind` はこの値を使う。値は組み込み先が照らし合わせるので変えない。
    pub fn code(&self) -> &'static str {
        self.info().0
    }

    /// 補足の値 (キーと値)。値は ID・名前なら [`FieldValue::Text`]、番号・数なら [`FieldValue::Index`]
    /// (wasm は文字列と数、C ABI は 10 進の文字列にする)。キー:
    ///
    /// - `module`・`slot`・`parameter`・`mesh`・`part` (ポーズの要素): ID・名前。`attached`: `slot_occupied` で付いていたモジュール。
    ///   `mask`: `unknown_mask` のマスクの参照先。`rig`・`collider`: `unknown_collider` のリグと衝突体。
    /// - `texture`: `texture_out_of_range` のテクスチャの番号 (モジュールの `Module::textures` の添字)。
    /// - `required`・`actual`: `avatar_level` で、モジュールが求めるレベルと本体のレベル。本体がアバタープロファイルを
    ///   持たなければ `actual` は無い。
    /// - `errors`: `invalid_physics`・`invalid_composition` の検査のエラーの数 (1 以上)。個々のエラーは文
    ///   (`Display`) にだけある (検査のエラーには機械が読む種類が無い)。
    pub fn fields(&self) -> Vec<(&'static str, FieldValue<'_>)> {
        self.info().1
    }

    fn info(&self) -> (&'static str, Vec<(&'static str, FieldValue<'_>)>) {
        use FieldValue::{Index, Text};
        match self {
            Self::NotAttached(name) => ("not_attached", vec![("module", Text(name))]),
            Self::Compile(_) => ("compile", vec![]),
            Self::Physics(errors) => ("invalid_physics", vec![("errors", Index(errors.len()))]),
            // 種類は ugoe-runtime の `ComposeError::code` (エディタの `compose.*` と同じ名前)。
            Self::Compose(e) => (e.code(), compose_fields(e)),
        }
    }
}

/// 合成の失敗の補足の値 ([`DressUpError::fields`])。
fn compose_fields(error: &ComposeError) -> Vec<(&'static str, FieldValue<'_>)> {
    use ComposeError as C;
    use FieldValue::{Index, Text};
    match error {
        C::InvalidModuleName(name) | C::DuplicateModule(name) => vec![("module", Text(name))],
        C::UnknownSlot { module, slot } | C::MissingSlot { module, slot } => {
            vec![("module", Text(module)), ("slot", Text(slot))]
        }
        C::SlotOccupied {
            slot,
            first,
            second,
        } => vec![
            ("slot", Text(slot)),
            ("module", Text(second)),
            ("attached", Text(first)),
        ],
        C::AvatarLevel {
            module,
            required,
            actual,
        } => {
            let mut fields = vec![
                ("module", Text(module)),
                ("required", Index(usize::from(*required))),
            ];
            if let Some(actual) = actual {
                fields.push(("actual", Index(usize::from(*actual))));
            }
            fields
        }
        C::MissingParameter { module, parameter } => {
            vec![("module", Text(module)), ("parameter", Text(parameter))]
        }
        C::ParameterNotExposed {
            module,
            slot,
            parameter,
        } => vec![
            ("module", Text(module)),
            ("slot", Text(slot)),
            ("parameter", Text(parameter)),
        ],
        C::UnknownMask { module, mesh, mask } => vec![
            ("module", Text(module)),
            ("mesh", Text(mesh)),
            ("mask", Text(mask)),
        ],
        C::TextureOutOfRange {
            module,
            mesh,
            texture,
        } => vec![
            ("module", Text(module)),
            ("mesh", Text(mesh)),
            ("texture", Index(*texture as usize)),
        ],
        C::UnknownCollider {
            module,
            rig,
            collider,
        } => vec![
            ("module", Text(module)),
            ("rig", Text(rig)),
            ("collider", Text(collider)),
        ],
        C::PoseNotExposed { module, slot, part } => vec![
            ("module", Text(module)),
            ("slot", Text(slot)),
            ("part", Text(part)),
        ],
        C::Invalid(errors) => vec![("errors", Index(errors.len()))],
    }
}

/// 着せ替えの状態: 本体と、取り付けているモジュール。
#[derive(Debug, Clone)]
pub(crate) struct Wardrobe {
    base: Model,
    /// 本体の物理の設定 (最初にモジュールを付けたときの設定か、[`Puppet::set_base_physics`] で差し替えた設定)。
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

    /// 本体の物理の設定 (モジュールを付ける前の設定。[`Puppet::set_base_physics`] で差し替えられる)。
    /// モジュールを付けたことがなければ今の設定。
    ///
    /// モデルを読み直すホストが、合成した設定 (モジュールのリグを含む) ではなく本体の設定を引き継ぐために使う。
    pub fn base_physics(&self) -> Option<&PhysicsSettings> {
        match &self.wardrobe {
            Some(wardrobe) => wardrobe.base_physics.as_ref(),
            None => self.physics.as_ref().map(|p| p.settings()),
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
    /// 本体の物理が無くてもモジュールにリグがあれば、重力 `(0, 1)`・風なし・60 Hz の設定に加える。このとき
    /// スロットが `colliders` で公開する本体の衝突体は何も指さず、リグはそれに当たらない
    /// (`ugoe_runtime::compose_physics_without_base`。モデルだけで作った Puppet でも付けられる)。
    pub fn set_modules(
        &mut self,
        modules: Vec<Arc<Module>>,
    ) -> Result<DressUpChange, DressUpError> {
        // 本体は組み替えに成功したときに覚える (失敗したら何も変えない。覚えた後は `set_physics` が本体の設定を変えなくなる)。
        let dressed = self.wardrobe.is_some();
        let mut wardrobe = self.wardrobe.take().unwrap_or_else(|| {
            let base = self.instance.compiled().model().clone();
            Wardrobe {
                textures: texture_sources(&base, &[]),
                base,
                base_physics: self.physics.as_ref().map(|p| p.settings().clone()),
                modules: Vec::new(),
            }
        });
        let Composed {
            compiled,
            physics,
            textures,
        } = match wardrobe.compose(&modules) {
            Ok(composed) => composed,
            Err(e) => {
                if dressed {
                    self.wardrobe = Some(wardrobe);
                }
                return Err(e);
            }
        };
        let change = texture_change(
            (&wardrobe.textures, &wardrobe.modules),
            (&textures, &modules),
        );
        wardrobe.modules = modules;
        wardrobe.textures = textures;
        self.swap_model(compiled, physics);
        self.wardrobe = Some(wardrobe);
        Ok(change)
    }

    /// 本体の物理の設定を差し替える (`None` で外す)。モジュールを付けていれば、付けているモジュールの組と合成し直した
    /// 設定に差し替え、この後の付け外しでもこの設定から合成する (`docs/spec/modules.md#puppet-での着せ替え`)。
    /// モジュールを付けたことがなければ [`Puppet::set_physics`] と同じ。
    ///
    /// 設定は本体のモデルと合わせて `ugoe_model::validate::validate_physics` で検査し、合成した設定も合成したモデルと
    /// 合わせて検査する。失敗したとき (検査・合成) は何も変えない。粒子は次の `update` で、その時の入力に対する
    /// 静止姿勢から始める (付けているモジュールのリグの揺れも止まる)。
    ///
    /// [`Puppet::set_physics`] は今の (合成した) 設定を差し替えるだけなので、モジュールを付けている間に渡した設定は
    /// 次の付け外しで失われる。付けている間も本体の設定を変えたいホストはこちらを使う。
    pub fn set_base_physics(
        &mut self,
        settings: Option<PhysicsSettings>,
    ) -> Result<(), DressUpError> {
        if let Some(settings) = &settings {
            let errors = validate_physics(settings, self.base_model());
            if !errors.is_empty() {
                return Err(DressUpError::Physics(errors));
            }
        }
        let Some(wardrobe) = &self.wardrobe else {
            self.replace_physics(settings);
            return Ok(());
        };
        let (model, physics) = compose_all(&wardrobe.base, settings.as_ref(), &wardrobe.modules)?;
        // 本体の物理の有無で、合成したモデルのスロットの `colliders` が変わることがある
        // (`compose_physics_without_base`)。変わらなければ物理だけを差し替える。
        if model.slots == self.instance.compiled().model().slots {
            self.replace_physics(physics);
        } else {
            let compiled = CompiledModel::compile(model)?;
            self.swap_model(compiled, physics);
        }
        if let Some(wardrobe) = &mut self.wardrobe {
            wardrobe.base_physics = settings;
        }
        Ok(())
    }

    /// モデルを `compiled` に差し替え、状態を引き継ぐ ([`Puppet::set_modules`] の規則)。
    fn swap_model(&mut self, compiled: Arc<CompiledModel>, physics: Option<PhysicsSettings>) {
        let mut instance = ModelInstance::new(compiled);
        let old = self.instance.parameters();
        instance.parameters_mut().carry_over(old);
        let new = instance.parameters();
        self.base = carry_values(old, &self.base, new);
        self.output = carry_values(old, &self.output, new);
        self.before_physics = carry_values(old, &self.before_physics, new);
        self.tracking_map = ugoe_motion::TrackingMap::for_model(instance.compiled().model(), new);
        self.instance = instance;
        self.resolve_tracking_kept();

        let previous = self.pose.clone();
        self.set_pose(self.instance.compiled().model().pose.clone());
        self.pose.carry_over(&previous);

        self.replace_physics(physics);
        // 受け手へ配るパーツの番号は合成の前のモデルのものなので捨てる (次の `update` で作り直す)。
        self.part_opacities_applied.clear();
        self.write_part_opacities(0.0, false);
        self.instance.update();
    }
}

/// 本体にモジュールの組を合成した結果 ([`Wardrobe::compose`])。
struct Composed {
    compiled: Arc<CompiledModel>,
    physics: Option<PhysicsSettings>,
    textures: Vec<TextureSource>,
}

impl Wardrobe {
    /// 本体に `modules` を合成してコンパイルし、合成した物理の設定とテクスチャの出どころと合わせて返す。
    fn compose(&self, modules: &[Arc<Module>]) -> Result<Composed, DressUpError> {
        let (model, physics) = compose_all(&self.base, self.base_physics.as_ref(), modules)?;
        Ok(Composed {
            compiled: CompiledModel::compile(model)?,
            physics,
            textures: texture_sources(&self.base, modules),
        })
    }
}

/// 本体 `base` と物理の設定 `base_physics` に `modules` を合成し、合成した物理を合成したモデルと合わせて検査する
/// ([`Puppet::set_modules`] の規則)。
fn compose_all(
    base: &Model,
    base_physics: Option<&PhysicsSettings>,
    modules: &[Arc<Module>],
) -> Result<(Model, Option<PhysicsSettings>), DressUpError> {
    let refs: Vec<&Module> = modules.iter().map(|m| m.as_ref()).collect();
    let mut model = compose(base, &refs)?;
    let has_rigs = refs.iter().any(|m| !m.physics.rigs.is_empty());
    let physics = match base_physics {
        Some(settings) => Some(compose_physics(base, settings, &refs)?),
        None if has_rigs => Some(compose_physics_without_base(
            base,
            &mut model,
            &empty_physics(),
            &refs,
        )?),
        None => None,
    };
    if let Some(physics) = &physics {
        let errors = validate_physics(physics, &model);
        if !errors.is_empty() {
            return Err(DressUpError::Physics(errors));
        }
    }
    Ok((model, physics))
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
