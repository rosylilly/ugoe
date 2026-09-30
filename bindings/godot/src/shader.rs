//! Godot のシェーダ (canvas_item) のソースを作る。式は `docs/spec/rendering.md`。
//!
//! - テクスチャは乗算済みアルファで渡す (読み込み時に [`crate::model::premultiply`] で変換する)。
//! - マスクは、マスク用メッシュのカバレッジ `c_i` をアルファに描いたテクスチャ (マスクのビューポート) から読む。
//!   サンプラの配列を動的な添字で読めない環境 (GLSL ES 3.0) があるので、[`MAX_MASKS`] 個に展開して書く。
//! - 合成モードは Godot の `blend_premul_alpha` (One / OneMinusSrcAlpha) と `blend_mul` (Dst / Zero) で表す。
//!   - `normal`: `blend_premul_alpha` でそのまま出す。
//!   - `additive`: `blend_premul_alpha` でアルファを 0 にして出す。`src + dst`、アルファは `dst.a` になる。
//!   - `multiply`: `blend_mul` で `rgb + (1 − a)` とアルファ 1 を出す。`src × dst + dst × (1 − src.a)`、アルファは `dst.a`。

use std::fmt::Write;

use crate::model::MAX_MASKS;

/// シェーダの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderKind {
    /// 通常の合成と加算 (uniform `additive` で切り替える)。
    PremulAlpha,
    /// 乗算。
    Multiply,
    /// マスクのビューポートに、カバレッジ `c = α × k` をアルファとして描く。
    Coverage,
}

/// マスクの uniform の名前 (番号 `i` のテクスチャ)。
pub fn mask_texture_param(i: usize) -> String {
    format!("mask_texture_{i}")
}

/// マスクの uniform の名前 (番号 `i` のテクスチャが覆うキャンバス上の矩形)。
pub fn mask_rect_param(i: usize) -> String {
    format!("mask_rect_{i}")
}

/// シェーダのソース。
pub fn source(kind: ShaderKind) -> String {
    let blend = match kind {
        ShaderKind::Multiply => "blend_mul",
        ShaderKind::PremulAlpha | ShaderKind::Coverage => "blend_premul_alpha",
    };
    let mut s = String::new();
    // `write!` を String に書くのは失敗しない。
    let _ = write!(
        s,
        "// ugoe-godot が生成する (bindings/godot/src/shader.rs)。
shader_type canvas_item;
render_mode unshaded, {blend};

// ノードの座標にキャンバスの原点を足すとキャンバス座標になる。
uniform vec2 canvas_origin = vec2(0.0);
uniform float opacity = 1.0;
uniform vec3 multiply_color = vec3(1.0);
uniform vec3 screen_color = vec3(0.0);
uniform bool additive = false;
uniform float mask_strength = 1.0;
uniform int mask_count = 0;
// ビット i が立っていれば、番号 i のマスクは反転マスク。
uniform int mask_inverts = 0;
uniform bool mask_intersection = false;
"
    );
    for i in 0..MAX_MASKS {
        let _ = writeln!(
            s,
            "uniform sampler2D {} : filter_linear, repeat_disable;\nuniform vec4 {};",
            mask_texture_param(i),
            mask_rect_param(i)
        );
    }
    let _ = write!(
        s,
        "
varying vec2 canvas_pos;
varying vec4 modulate;

void vertex() {{
\tcanvas_pos = VERTEX + canvas_origin;
\tmodulate = COLOR;
}}

// マスクのテクスチャが覆う矩形の外は 0。
float coverage(sampler2D tex, vec4 rect, vec2 p) {{
\tvec2 uv = (p - rect.xy) / rect.zw;
\tif (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) {{
\t\treturn 0.0;
\t}}
\treturn texture(tex, uv).a;
}}

void fragment() {{
\t// マスク係数 k (docs/spec/rendering.md#マスクの効き具合)
\tbool has_pos = false;
\tfloat pos_union = 1.0;
\tfloat pos_all = 1.0;
\tfloat neg_union = 1.0;
"
    );
    for i in 0..MAX_MASKS {
        let _ = write!(
            s,
            "\tif (mask_count > {i}) {{
\t\tfloat c = coverage({tex}, {rect}, canvas_pos);
\t\tif ((mask_inverts & {bit}) != 0) {{
\t\t\tneg_union *= 1.0 - c;
\t\t}} else {{
\t\t\thas_pos = true;
\t\t\tpos_union *= 1.0 - c;
\t\t\tpos_all *= c;
\t\t}}
\t}}
",
            tex = mask_texture_param(i),
            rect = mask_rect_param(i),
            bit = 1u32 << i,
        );
    }
    s.push_str(
        "\tfloat pos = has_pos ? (mask_intersection ? pos_all : 1.0 - pos_union) : 1.0;
\tfloat m = pos * neg_union;
\tfloat k = mask_count > 0 ? 1.0 - clamp(mask_strength, 0.0, 1.0) * (1.0 - m) : 1.0;
\tvec4 c = texture(TEXTURE, UV);
",
    );
    match kind {
        ShaderKind::Coverage => s.push_str("\tCOLOR = vec4(0.0, 0.0, 0.0, c.a * k);\n"),
        ShaderKind::PremulAlpha | ShaderKind::Multiply => {
            s.push_str(
                "\t// docs/spec/rendering.md#描画可能要素ごとのシェーディング
\tvec3 rgb = c.rgb * multiply_color;
\trgb = rgb + screen_color * c.a - rgb * screen_color;
\tvec4 o = vec4(rgb, c.a) * opacity * k;
\t// ノードの modulate (乗算済みにして掛ける)
\to = vec4(o.rgb * modulate.rgb, o.a) * modulate.a;
",
            );
            s.push_str(match kind {
                ShaderKind::Multiply => "\tCOLOR = vec4(o.rgb + (1.0 - o.a), 1.0);\n",
                _ => "\tCOLOR = additive ? vec4(o.rgb, 0.0) : o;\n",
            });
        }
    }
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_declares_all_mask_slots() {
        for kind in [
            ShaderKind::PremulAlpha,
            ShaderKind::Multiply,
            ShaderKind::Coverage,
        ] {
            let src = source(kind);
            for i in 0..MAX_MASKS {
                assert!(src.contains(&format!("uniform sampler2D {}", mask_texture_param(i))));
                assert!(src.contains(&format!("uniform vec4 {};", mask_rect_param(i))));
            }
            assert_eq!(src.matches('{').count(), src.matches('}').count());
        }
    }

    #[test]
    fn blend_modes_follow_the_spec() {
        assert!(source(ShaderKind::PremulAlpha).contains("blend_premul_alpha"));
        assert!(source(ShaderKind::PremulAlpha).contains("additive ? vec4(o.rgb, 0.0) : o"));
        assert!(source(ShaderKind::Multiply).contains("blend_mul"));
        assert!(source(ShaderKind::Multiply).contains("o.rgb + (1.0 - o.a)"));
        assert!(source(ShaderKind::Coverage).contains("c.a * k"));
    }
}
