//! 標準パラメータ ID。
//!
//! モーション・表情・物理プリセット・AI 自動リギングがモデル間で再利用できるよう、
//! 意味の決まったパラメータ ID を定義する。値域の慣例は `docs/spec/model.md#標準パラメータ`。

pub const ANGLE_X: &str = "AngleX";
pub const ANGLE_Y: &str = "AngleY";
pub const ANGLE_Z: &str = "AngleZ";
pub const BODY_ANGLE_X: &str = "BodyAngleX";
pub const BODY_ANGLE_Y: &str = "BodyAngleY";
pub const BODY_ANGLE_Z: &str = "BodyAngleZ";
pub const BREATH: &str = "Breath";
pub const EYE_L_OPEN: &str = "EyeLOpen";
pub const EYE_R_OPEN: &str = "EyeROpen";
pub const EYE_L_SMILE: &str = "EyeLSmile";
pub const EYE_R_SMILE: &str = "EyeRSmile";
pub const EYE_BALL_X: &str = "EyeBallX";
pub const EYE_BALL_Y: &str = "EyeBallY";
pub const BROW_L_Y: &str = "BrowLY";
pub const BROW_R_Y: &str = "BrowRY";
pub const MOUTH_FORM: &str = "MouthForm";
pub const MOUTH_OPEN_Y: &str = "MouthOpenY";
pub const CHEEK: &str = "Cheek";
pub const HAIR_FRONT: &str = "HairFront";
pub const HAIR_SIDE: &str = "HairSide";
pub const HAIR_BACK: &str = "HairBack";

/// 全標準パラメータと既定の値域 `(id, min, default, max)`。
pub const ALL: &[(&str, f32, f32, f32)] = &[
    (ANGLE_X, -30.0, 0.0, 30.0),
    (ANGLE_Y, -30.0, 0.0, 30.0),
    (ANGLE_Z, -30.0, 0.0, 30.0),
    (BODY_ANGLE_X, -10.0, 0.0, 10.0),
    (BODY_ANGLE_Y, -10.0, 0.0, 10.0),
    (BODY_ANGLE_Z, -10.0, 0.0, 10.0),
    (BREATH, 0.0, 0.0, 1.0),
    (EYE_L_OPEN, 0.0, 1.0, 1.0),
    (EYE_R_OPEN, 0.0, 1.0, 1.0),
    (EYE_L_SMILE, 0.0, 0.0, 1.0),
    (EYE_R_SMILE, 0.0, 0.0, 1.0),
    (EYE_BALL_X, -1.0, 0.0, 1.0),
    (EYE_BALL_Y, -1.0, 0.0, 1.0),
    (BROW_L_Y, -1.0, 0.0, 1.0),
    (BROW_R_Y, -1.0, 0.0, 1.0),
    (MOUTH_FORM, -1.0, 0.0, 1.0),
    (MOUTH_OPEN_Y, 0.0, 0.0, 1.0),
    (CHEEK, 0.0, 0.0, 1.0),
    (HAIR_FRONT, -1.0, 0.0, 1.0),
    (HAIR_SIDE, -1.0, 0.0, 1.0),
    (HAIR_BACK, -1.0, 0.0, 1.0),
];
