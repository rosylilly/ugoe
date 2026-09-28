//! `assets/fixtures/` の PSD の中身。コミットしたファイルがこの定義と一致することをテストする
//! (作り直すときは `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-psd --test fixtures`)。

use std::path::PathBuf;

use super::shapes::{Image, Shape};
use super::{Layer as W, levels_identity, write_psd};

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/fixtures")
        .join(name)
}

// ---- layers.psd ----

pub const LAYERS_WIDTH: u32 = 64;
pub const LAYERS_HEIGHT: u32 = 48;

/// `layers.psd` の中身 (下から上)。取り込みの構造を試すための小さな PSD で、画素は模様。
pub fn layers() -> Vec<W> {
    vec![
        W {
            id: Some(1),
            ..W::raster("背景", 0, 0, LAYERS_WIDTH, LAYERS_HEIGHT)
        },
        W::group_end(),
        W {
            id: Some(3),
            ..W::raster("胴", 10, 20, 20, 24)
        },
        W::group_end(),
        W {
            id: Some(5),
            compression: 0,
            ..W::raster("肌", 16, 4, 32, 20)
        },
        W {
            id: Some(6),
            clipping: true,
            fill_opacity: Some(204),
            blend: *b"mul ",
            ..W::raster("影", 20, 8, 8, 8)
        },
        W {
            id: Some(7),
            hidden: true,
            ..W::raster("目", 22, 10, 6, 4)
        },
        W {
            id: Some(8),
            ..W::group("顔")
        },
        W {
            id: Some(9),
            opacity: 230,
            // 閉じたフォルダで、`lsct` にブレンドモードを持たない。
            section: Some((2, None)),
            ..W::group("体")
        },
        W {
            id: Some(10),
            opacity: 128,
            ..W::raster("メモ", -4, -2, 12, 8)
        },
        W {
            id: Some(11),
            // 何も変えないレベル補正の調整レイヤー。
            extra: vec![(*b"levl", levels_identity())],
            ..W::new("レベル補正")
        },
    ]
}

pub fn layers_bytes() -> Vec<u8> {
    write_psd(LAYERS_WIDTH, LAYERS_HEIGHT, &layers())
}

// ---- character.psd ----

pub const CHARACTER_WIDTH: u32 = 320;
pub const CHARACTER_HEIGHT: u32 = 400;

pub const BACKGROUND: [u8; 4] = [222, 232, 242, 255];
pub const HAIR: [u8; 4] = [92, 58, 44, 255];
pub const SKIN: [u8; 4] = [250, 222, 204, 255];
/// 顔の影 (乗算)。
pub const SHADE: [u8; 4] = [236, 200, 186, 255];
pub const CHEEK: [u8; 4] = [244, 140, 150, 255];
pub const EYE_WHITE: [u8; 4] = [255, 255, 255, 255];
pub const IRIS: [u8; 4] = [60, 96, 170, 255];
pub const PUPIL: [u8; 4] = [26, 30, 52, 255];
pub const MOUTH: [u8; 4] = [196, 72, 84, 255];
pub const MOUTH_OPEN: [u8; 4] = [140, 36, 56, 255];
pub const SHIRT: [u8; 4] = [70, 120, 200, 255];
pub const COLLAR: [u8; 4] = [245, 245, 250, 255];

/// 目の中心 (キャラクターの右目 = 画面の左、左目 = 画面の右)。
pub const RIGHT_EYE: (f32, f32) = (126.0, 150.0);
pub const LEFT_EYE: (f32, f32) = (194.0, 150.0);
pub const MOUTH_CENTER: (f32, f32) = (160.0, 200.0);
pub const FACE_CENTER: (f32, f32) = (160.0, 150.0);

/// キャンバスいっぱいの画像に描いて、描いた範囲で切り出したレイヤーを作る。
fn drawn(name: &str, id: u32, paint: impl FnOnce(&mut Image)) -> W {
    let mut image = Image::new(CHARACTER_WIDTH, CHARACTER_HEIGHT);
    paint(&mut image);
    let (left, top, width, height, rgba) = image.crop();
    W {
        id: Some(id),
        ..W::image(name, left, top, width, height, rgba)
    }
}

fn eye(name: &str, id: u32, (cx, cy): (f32, f32)) -> W {
    drawn(name, id, |img| {
        img.fill(&Shape::ellipse(cx, cy, 17.0, 21.0), EYE_WHITE)
            .fill(&Shape::ellipse(cx, cy + 3.0, 11.0, 15.0), IRIS)
            .fill(&Shape::ellipse(cx, cy + 4.0, 5.0, 7.0), PUPIL)
            .fill(&Shape::ellipse(cx - 4.0, cy - 3.0, 3.5, 3.5), EYE_WHITE);
    })
}

/// `character.psd` の中身 (下から上)。取り込んで目で確かめるための、単色の図形で描いた顔と体。
///
/// ```text
/// 背景
/// 後ろ髪
/// 体 (グループ、通常)
///   首, 服
/// 頭 (グループ、通過)
///   顔
///   顔の影 (顔でクリッピング、乗算)
///   頬 (不透明度 50%)
///   目 (グループ、通過): 右目, 左目
///   口
///   口 (開き) (非表示)
///   前髪
/// ```
pub fn character() -> Vec<W> {
    let (w, h) = (CHARACTER_WIDTH as f32, CHARACTER_HEIGHT as f32);
    let (mx, my) = MOUTH_CENTER;
    let bangs = Shape::Union(vec![
        Shape::ellipse(160.0, 86.0, 90.0, 56.0).minus(Shape::rect(0.0, 96.0, w, h, 0.0)),
        Shape::Polygon(vec![
            (70.0, 90.0),
            (74.0, 132.0),
            (92.0, 100.0),
            (106.0, 124.0),
            (124.0, 98.0),
            (146.0, 120.0),
            (160.0, 96.0),
            (174.0, 120.0),
            (196.0, 98.0),
            (214.0, 124.0),
            (228.0, 100.0),
            (246.0, 132.0),
            (250.0, 90.0),
        ]),
    ]);
    vec![
        drawn("背景", 1, |img| {
            img.fill(&Shape::rect(0.0, 0.0, w, h, 0.0), BACKGROUND);
        }),
        drawn("後ろ髪", 2, |img| {
            img.fill(&Shape::ellipse(160.0, 140.0, 98.0, 104.0), HAIR)
                .fill(&Shape::rect(62.0, 140.0, 258.0, 292.0, 34.0), HAIR);
        }),
        W {
            id: Some(3),
            ..W::group_end()
        },
        drawn("首", 4, |img| {
            img.fill(&Shape::rect(140.0, 210.0, 180.0, 276.0, 8.0), SKIN);
        }),
        drawn("服", 5, |img| {
            img.fill(
                &Shape::Polygon(vec![
                    (86.0, h),
                    (96.0, 300.0),
                    (126.0, 268.0),
                    (194.0, 268.0),
                    (224.0, 300.0),
                    (234.0, h),
                ]),
                SHIRT,
            )
            .fill(
                &Shape::Polygon(vec![(136.0, 268.0), (184.0, 268.0), (160.0, 300.0)]),
                COLLAR,
            )
            .fill(
                &Shape::Polygon(vec![(142.0, 268.0), (178.0, 268.0), (160.0, 290.0)]),
                SKIN,
            );
        }),
        W {
            id: Some(6),
            section: Some((1, Some(*b"norm"))),
            ..W::group("体")
        },
        W {
            id: Some(7),
            ..W::group_end()
        },
        drawn("顔", 8, |img| {
            img.fill(&Shape::ellipse(160.0, 150.0, 78.0, 88.0), SKIN);
        }),
        W {
            clipping: true,
            blend: *b"mul ",
            ..drawn("顔の影", 9, |img| {
                img.fill(&Shape::ellipse(160.0, 84.0, 100.0, 50.0), SHADE);
            })
        },
        W {
            opacity: 128,
            ..drawn("頬", 10, |img| {
                img.fill(&Shape::ellipse(114.0, 186.0, 15.0, 8.0), CHEEK)
                    .fill(&Shape::ellipse(206.0, 186.0, 15.0, 8.0), CHEEK);
            })
        },
        W {
            id: Some(11),
            ..W::group_end()
        },
        eye("右目", 12, RIGHT_EYE),
        eye("左目", 13, LEFT_EYE),
        W {
            id: Some(14),
            ..W::group("目")
        },
        drawn("口", 15, |img| {
            img.fill(
                &Shape::ellipse(mx, my, 18.0, 11.0).minus(Shape::ellipse(mx, my - 6.0, 21.0, 11.0)),
                MOUTH,
            );
        }),
        W {
            hidden: true,
            ..drawn("口 (開き)", 16, |img| {
                img.fill(&Shape::ellipse(mx, my + 2.0, 13.0, 10.0), MOUTH_OPEN)
                    .fill(&Shape::ellipse(mx, my + 7.0, 8.0, 4.0), CHEEK);
            })
        },
        drawn("前髪", 17, |img| {
            img.fill(&bangs, HAIR);
        }),
        W {
            id: Some(18),
            ..W::group("頭")
        },
    ]
}

pub fn character_bytes() -> Vec<u8> {
    write_psd(CHARACTER_WIDTH, CHARACTER_HEIGHT, &character())
}
