use std::convert::Infallible;

use ugoe::format::module_file::{ModuleFile, write_module};
use ugoe::format::{TextureData, TextureFormat};
use ugoe::model::fixtures::{dress_module, dress_up_base, long_hair_module};

use super::*;

/// レンダラへの登録を記録する。画像は 1x1 なので、先頭の画素の R で見分ける。
#[derive(Debug, Default)]
struct Recorder {
    ops: Vec<Op>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Texture(u32, u8),
    NormalMap(u32, Option<u8>),
}

impl TextureSink for Recorder {
    type Error = Infallible;

    fn set_texture(&mut self, index: u32, image: &Rgba8) -> Result<(), Infallible> {
        self.ops.push(Op::Texture(index, image.pixels[0]));
        Ok(())
    }

    fn set_normal_map(&mut self, index: u32, image: Option<&Rgba8>) -> Result<(), Infallible> {
        self.ops
            .push(Op::NormalMap(index, image.map(|i| i.pixels[0])));
        Ok(())
    }
}

impl Recorder {
    fn take(&mut self) -> Vec<Op> {
        std::mem::take(&mut self.ops)
    }
}

fn encode_png(rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&rgba).unwrap();
    writer.finish().unwrap();
    out
}

fn png(rgba: [u8; 4]) -> TextureData {
    TextureData {
        format: TextureFormat::Png,
        bytes: encode_png(rgba),
    }
}

/// テクスチャが R = `red` の 1x1 の不透明な画像の `.ugoemod`。`normal` があれば法線マップも付ける。
fn module_bytes(module: Module, red: u8, normal: Option<u8>) -> Vec<u8> {
    let mut module = module;
    if normal.is_some() {
        module.textures[0].normal_map = Some("normal".into());
    }
    let mut file = ModuleFile::new(module);
    file.textures = vec![png([red, 0, 0, 255])];
    file.normal_maps = normal.map(|n| png([n, 128, 255, 0])).into_iter().collect();
    write_module(&file).unwrap()
}

fn loaded(module: Module, red: u8, normal: Option<u8>) -> LoadedModule {
    from_module_file(&module_bytes(module, red, normal)).unwrap()
}

fn image(red: u8) -> Rgba8 {
    Rgba8 {
        width: 1,
        height: 1,
        pixels: vec![red, 0, 0, 255],
    }
}

/// 本体 (テクスチャ 0 番は R = 10) を読み込んだ Puppet と、`LongHair` (R = 20・法線マップ R = 21) と
/// `Dress` (R = 30) を読み込んだ Wardrobe。
fn setup(sink: &mut Recorder) -> (Puppet, Wardrobe) {
    let mut puppet = Puppet::new(dress_up_base()).unwrap();
    let mut wardrobe = Wardrobe::default();
    let warnings = wardrobe
        .load_base(&mut puppet, &[], &[image(10)], &[None], sink)
        .unwrap();
    assert!(warnings.is_empty());
    assert_eq!(sink.take(), [Op::Texture(0, 10), Op::NormalMap(0, None)]);
    for module in [
        loaded(long_hair_module(), 20, Some(21)),
        loaded(dress_module(), 30, None),
    ] {
        wardrobe.load(Some(&mut puppet), module, sink).unwrap();
    }
    // 読み込むだけでは付けない。
    assert!(sink.take().is_empty());
    assert!(puppet.modules().is_empty());
    (puppet, wardrobe)
}

#[test]
fn reads_module_files_with_normal_maps() {
    let module = loaded(long_hair_module(), 20, Some(21));
    assert_eq!(module.module.name, "LongHair");
    assert_eq!(module.textures, [image(20)]);
    // 法線マップはアルファを掛けない。
    assert_eq!(
        module.normal_maps,
        [Some(Rgba8 {
            width: 1,
            height: 1,
            pixels: vec![21, 128, 255, 0],
        })]
    );
    assert!(matches!(
        from_module_file(b"not a module"),
        Err(LoadError::Model(ugoe::LoadError::Format(_)))
    ));
}

/// 読み込みの上限を超えたモジュールは、`invalid_file` ではなく上限の種類の失敗になる (T0152)。
#[test]
fn modules_over_the_read_limits_are_rejected_with_a_code() {
    let mut file = ModuleFile::new(long_hair_module());
    file.textures = vec![png([20, 0, 0, 255])];
    let bytes = ugoe::format::module_file::write_module_with(
        &file,
        ugoe::format::WriteOptions { compress: true },
    )
    .unwrap();
    let limits = LoadLimits {
        read: ugoe::limits::ReadLimits::default().with_max_total_decompressed(1),
        ..LoadLimits::default()
    };
    let error = WardrobeError::from(from_module_file_with(&bytes, limits).unwrap_err());
    let info = error_info(&error);
    assert_eq!(info.code, "read_limit_exceeded");
    assert!(info.fields.contains(&("max", 1.into())), "{info:?}");
    assert!(from_module_file_with(&bytes, LoadLimits::default()).is_ok());
    let error = WardrobeError::from(from_module_file(b"not a module").unwrap_err());
    assert_eq!(error_info(&error).code, "invalid_file");
}

#[test]
fn attach_and_detach_register_textures_in_order() {
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);

    wardrobe.attach(&mut puppet, "LongHair", &mut sink).unwrap();
    assert_eq!(
        sink.take(),
        [Op::Texture(1, 20), Op::NormalMap(1, Some(21))]
    );
    wardrobe.attach(&mut puppet, "Dress", &mut sink).unwrap();
    assert_eq!(sink.take(), [Op::Texture(2, 30), Op::NormalMap(2, None)]);
    assert_eq!(attached_names(Some(&puppet)), ["LongHair", "Dress"]);

    // 前のモジュールを外すと、後ろのモジュールのテクスチャの番号が 1 つずれる。
    // 空いた 2 番は 1x1 の白で置き換えて、前の画像を捨てる。
    wardrobe.detach(&mut puppet, "LongHair", &mut sink).unwrap();
    assert_eq!(
        sink.take(),
        [
            Op::Texture(1, 30),
            Op::NormalMap(1, None),
            Op::Texture(2, 255),
            Op::NormalMap(2, None),
        ]
    );

    wardrobe.set_modules(&mut puppet, &[], &mut sink).unwrap();
    assert_eq!(sink.take(), [Op::Texture(1, 255), Op::NormalMap(1, None)]);
    assert!(puppet.modules().is_empty());
    // 何も付けていなければ、外すものも無い。
    wardrobe.set_modules(&mut puppet, &[], &mut sink).unwrap();
    assert!(sink.take().is_empty());
}

/// 合成したモデルのパーツの ID は合成の順で、モジュールのパーツは合成の ID になる (T1400)。
#[test]
fn part_ids_follow_the_composed_model() {
    assert!(part_ids(None).is_empty());
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);
    let base: Vec<String> = puppet
        .base_model()
        .parts
        .iter()
        .map(|p| p.id.to_string())
        .collect();
    assert!(base.contains(&"Hair".to_string()), "{base:?}");
    assert_eq!(part_ids(Some(&puppet)), base);

    wardrobe.attach(&mut puppet, "Dress", &mut sink).unwrap();
    let mut dressed = base.clone();
    dressed.push("Dress/Dress".into());
    assert_eq!(part_ids(Some(&puppet)), dressed);

    wardrobe.detach(&mut puppet, "Dress", &mut sink).unwrap();
    assert_eq!(part_ids(Some(&puppet)), base);
}

#[test]
fn reloading_an_attached_module_replaces_its_texture() {
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);
    wardrobe
        .set_modules(&mut puppet, &["LongHair".into(), "Dress".into()], &mut sink)
        .unwrap();
    sink.take();

    // 同じ名前の新しい版を読み込むと、付けたまま差し替える。
    let new = loaded(long_hair_module(), 40, None);
    wardrobe.load(Some(&mut puppet), new, &mut sink).unwrap();
    assert_eq!(sink.take(), [Op::Texture(1, 40), Op::NormalMap(1, None)]);
    assert_eq!(wardrobe.library().len(), 2);
    assert_eq!(wardrobe.library()[0].textures, [image(40)]);

    // 捨てると外れる。
    wardrobe
        .unload(Some(&mut puppet), "LongHair", &mut sink)
        .unwrap();
    assert_eq!(attached_names(Some(&puppet)), ["Dress"]);
    assert_eq!(wardrobe.library().len(), 1);
    assert!(matches!(
        wardrobe.attach(&mut puppet, "LongHair", &mut sink),
        Err(WardrobeError::NotLoaded(_))
    ));
}

#[test]
fn reloading_the_base_reattaches_modules_and_drops_leftovers() {
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);
    wardrobe
        .set_modules(&mut puppet, &["LongHair".into(), "Dress".into()], &mut sink)
        .unwrap();
    sink.take();
    let attached: Vec<String> = attached_names(Some(&puppet))
        .into_iter()
        .map(str::to_owned)
        .collect();

    // 編集で読み直したモデル (同じスロット): 付けていたモジュールを付け直し、すべて登録し直す。
    let mut reloaded = Puppet::new(dress_up_base()).unwrap();
    let warnings = wardrobe
        .load_base(
            &mut reloaded,
            &attached,
            &[image(11)],
            &[Some(image(12))],
            &mut sink,
        )
        .unwrap();
    assert!(warnings.is_empty());
    assert!(wardrobe.detached().is_empty());
    assert_eq!(attached_names(Some(&reloaded)), ["LongHair", "Dress"]);
    assert_eq!(
        sink.take(),
        [
            Op::Texture(0, 11),
            Op::NormalMap(0, Some(12)),
            Op::Texture(1, 20),
            Op::NormalMap(1, Some(21)),
            Op::Texture(2, 30),
            Op::NormalMap(2, None),
        ]
    );

    // スロット `Outfit` の無いモデル: `Dress` は付けられないので外して警告し、残った画像を捨てる。
    let mut base = dress_up_base();
    base.slots.retain(|s| s.id != "Outfit");
    let mut other = Puppet::new(base).unwrap();
    let warnings = wardrobe
        .load_base(&mut other, &attached, &[image(13)], &[], &mut sink)
        .unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("`Dress` was detached"), "{warnings:?}");
    // 同じものを構造で読める (文の警告は `Detached::warning` から作る)。
    let detached = wardrobe.detached();
    assert_eq!(detached.len(), 1);
    assert_eq!(detached[0].warning(), warnings[0]);
    assert_eq!(
        detached[0].to_json(),
        serde_json::json!({
            "code": "module_detached",
            "module": "Dress",
            "slot": "Outfit",
            "error": {
                "code": "unknown_slot",
                "message": detached[0].error.to_string(),
                "module": "Dress",
                "slot": "Outfit",
            },
            "warning": warnings[0],
        })
    );
    // 読み込みのほかの警告と同じ一覧では、外したものは後ろに並ぶ (戻り値の文の警告と同じ順。T0125)。
    let texture = crate::load::LoadWarning::TextureMissing { texture: 0 };
    let list = crate::load::warnings_json(std::slice::from_ref(&texture), detached);
    assert_eq!(
        list,
        [crate::load::warning_json(&texture), detached[0].to_json()]
    );
    assert_eq!(attached_names(Some(&other)), ["LongHair"]);
    assert_eq!(
        sink.take(),
        [
            Op::Texture(0, 13),
            Op::NormalMap(0, None),
            Op::Texture(1, 20),
            Op::NormalMap(1, Some(21)),
            Op::Texture(2, 255),
            Op::NormalMap(2, None),
        ]
    );
    // 読み込んだモジュールは捨てない (別のモデルに付けられる)。
    assert_eq!(wardrobe.infos(Some(&other)).len(), 2);
}

#[test]
fn failed_attach_changes_nothing_and_reports_a_code() {
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);
    let mut stray = long_hair_module();
    stray.name = "Stray".into();
    stray.slot = "Hat".into();
    wardrobe
        .load(Some(&mut puppet), loaded(stray, 50, None), &mut sink)
        .unwrap();

    let error = wardrobe
        .attach(&mut puppet, "Stray", &mut sink)
        .unwrap_err();
    assert!(puppet.modules().is_empty());
    assert!(sink.take().is_empty());
    assert_eq!(
        error_info(&error),
        ErrorInfo {
            code: "unknown_slot",
            fields: vec![("module", "Stray".into()), ("slot", "Hat".into())],
        }
    );

    let error = wardrobe
        .detach(&mut puppet, "Dress", &mut sink)
        .unwrap_err();
    assert_eq!(error_info(&error).code, "not_attached");
    let error = wardrobe.attach(&mut puppet, "Nope", &mut sink).unwrap_err();
    assert_eq!(error_info(&error).code, "not_loaded");
}

/// GPU を失った後の付け外し (`with_wardrobe!`) は、ほかの着せ替えの失敗と同じ `DressUpError` で `code` が `device_lost` (T0457)。
#[test]
fn device_lost_is_a_coded_dress_up_error() {
    assert_eq!(WardrobeError::NAME, "DressUpError");
    let error = WardrobeError::DeviceLost;
    assert_eq!(
        error_info(&error),
        ErrorInfo {
            code: "device_lost",
            fields: vec![],
        }
    );
    // 文は T0457 の前の例外 (コード無しの `JsError`) と同じ。
    assert_eq!(error.to_string(), "the GPU device was lost");
}

/// 補足の値は C ABI (`crates/ugoe-ffi/src/dress_up.rs` の `numbered_and_referenced_fields_reach_the_c_abi`) と同じキーで出る。
/// 番号は数 (C ABI では 10 進の文字列)。無い値 (本体がプロファイルを持たないときの `actual`) はキーごと無い。
#[test]
fn numbered_and_referenced_fields_reach_js() {
    use ugoe::model::validate::ValidationError;
    use ugoe::runtime::ComposeError;

    let s = |v: &str| v.to_owned();
    let invalid = ValidationError::InvalidModuleName { name: s("") };
    type Fields = Vec<(&'static str, serde_json::Value)>;
    let cases: Vec<(DressUpError, &str, Fields)> = vec![
        (
            ComposeError::UnknownMask {
                module: s("m"),
                mesh: s("x"),
                mask: s("k"),
            }
            .into(),
            "unknown_mask",
            vec![
                ("module", "m".into()),
                ("mesh", "x".into()),
                ("mask", "k".into()),
            ],
        ),
        (
            ComposeError::UnknownCollider {
                module: s("m"),
                rig: s("r"),
                collider: s("c"),
            }
            .into(),
            "unknown_collider",
            vec![
                ("module", "m".into()),
                ("rig", "r".into()),
                ("collider", "c".into()),
            ],
        ),
        (
            ComposeError::PoseNotExposed {
                module: s("m"),
                slot: s("s"),
                part: s("p"),
            }
            .into(),
            "pose_not_exposed",
            vec![
                ("module", "m".into()),
                ("slot", "s".into()),
                ("part", "p".into()),
            ],
        ),
        (
            ComposeError::TextureOutOfRange {
                module: s("m"),
                mesh: s("x"),
                texture: 3,
            }
            .into(),
            "texture_out_of_range",
            vec![
                ("module", "m".into()),
                ("mesh", "x".into()),
                ("texture", 3.into()),
            ],
        ),
        (
            ComposeError::AvatarLevel {
                module: s("m"),
                required: 2,
                actual: Some(1),
            }
            .into(),
            "avatar_level",
            vec![
                ("module", "m".into()),
                ("required", 2.into()),
                ("actual", 1.into()),
            ],
        ),
        (
            ComposeError::AvatarLevel {
                module: s("m"),
                required: 2,
                actual: None,
            }
            .into(),
            "avatar_level",
            vec![("module", "m".into()), ("required", 2.into())],
        ),
        // 検査のエラーの種類の一覧は、出た順に重なりを除いて `, ` でつないだ文字列 (T0168)。
        (
            DressUpError::Physics(vec![
                invalid.clone(),
                ValidationError::DuplicateId {
                    kind: "slot",
                    id: s("s"),
                },
                invalid.clone(),
            ]),
            "invalid_physics",
            vec![
                ("errors", 3.into()),
                ("error_codes", "invalid_module_name, duplicate_id".into()),
            ],
        ),
        (
            ComposeError::Invalid(vec![invalid]).into(),
            "invalid_composition",
            vec![
                ("errors", 1.into()),
                ("error_codes", "invalid_module_name".into()),
            ],
        ),
    ];
    for (error, code, fields) in cases {
        let info = error_info(&WardrobeError::DressUp(error));
        assert_eq!(info, ErrorInfo { code, fields }, "{code}");
    }
}

#[test]
fn failed_attach_reports_numbered_fields() {
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);
    let mut picky = long_hair_module();
    picky.name = "Picky".into();
    picky.requires.avatar_level = Some(2);
    wardrobe
        .load(Some(&mut puppet), loaded(picky, 50, None), &mut sink)
        .unwrap();

    let error = wardrobe
        .attach(&mut puppet, "Picky", &mut sink)
        .unwrap_err();
    assert_eq!(
        error_info(&error),
        ErrorInfo {
            code: "avatar_level",
            fields: vec![("module", "Picky".into()), ("required", 2.into())],
        }
    );
    // `detached_modules_json` の `error` も同じ値 (番号は数)。
    let detached = Detached {
        module: "Picky".into(),
        slot: "Hair".into(),
        error,
    };
    assert_eq!(detached.to_json()["error"]["required"], 2);
}

#[test]
fn lists_modules_and_slots() {
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);
    wardrobe.attach(&mut puppet, "Dress", &mut sink).unwrap();
    let json = serde_json::to_value(wardrobe.infos(Some(&puppet))).unwrap();
    assert_eq!(json[0]["name"], "LongHair");
    assert_eq!(json[0]["slot"], "Hair");
    assert_eq!(json[0]["attached"], false);
    assert_eq!(json[1]["name"], "Dress");
    assert_eq!(json[1]["attached"], true);
    assert_eq!(json[1]["requires"]["parameters"][0], "BodySway");
    assert_eq!(json[1]["textures"], 1);

    // スロットはモジュールを付けていても本体のもの。
    let ids: Vec<&str> = slots(&puppet).iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["Hair", "Outfit"]);
}

#[test]
fn reloading_keeps_the_base_physics_not_the_composed_one() {
    let mut sink = Recorder::default();
    let (mut puppet, mut wardrobe) = setup(&mut sink);
    // 本体に物理が無くても、LongHair のリグで物理が加わる。
    wardrobe.attach(&mut puppet, "LongHair", &mut sink).unwrap();
    assert!(puppet.physics().is_some());

    let mut reloaded = Puppet::new(dress_up_base()).unwrap();
    let warnings = crate::animation::carry_over(&puppet, &mut reloaded, true);
    // 合成した設定 (モジュールのリグ) を本体に写そうとして外れる、ということが起きない。
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(reloaded.physics().is_none());
    wardrobe
        .load_base(
            &mut reloaded,
            &["LongHair".into()],
            &[image(10)],
            &[],
            &mut sink,
        )
        .unwrap();
    assert_eq!(
        reloaded.physics().unwrap().settings().rigs[0].id.as_str(),
        "LongHair/Sway"
    );
}

/// 公開するスロットは本体のアバタープロファイルの `slots` (モジュールを付けても本体のもの)。
/// プロファイルが無ければ `None` (T0715)。
#[test]
fn published_slots_come_from_the_base_avatar_profile() {
    let mut model = dress_up_base();
    model.avatar = None;
    let puppet = Puppet::new(model.clone()).unwrap();
    assert_eq!(published_slots(&puppet), None);

    let first = model.slots[0].id.clone();
    model.avatar = Some(ugoe::model::AvatarProfile {
        slots: vec![first.clone()],
        ..ugoe::model::AvatarProfile::default()
    });
    let mut puppet = Puppet::new(model).unwrap();
    assert_eq!(published_slots(&puppet), Some(&[first.clone()][..]));
    puppet
        .attach_module(std::sync::Arc::new(long_hair_module()))
        .unwrap();
    assert_eq!(published_slots(&puppet), Some(&[first][..]));
}
