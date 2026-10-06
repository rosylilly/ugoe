//! Godot のシェーダ (canvas_item) のソースを作る。式は `docs/spec/rendering.md`。
//!
//! - テクスチャは乗算済みアルファで渡す (読み込み時に参照の `ugoe::format::image::premultiply` で変換する)。
//! - マスクは、マスク用メッシュのカバレッジ `c_i` をアルファに描いたアトラス (マスクのビューポート) から読む。
//!   どのカバレッジがアトラスのどこにあるか、要素がどのマスクを読むかは、浮動小数点のテクスチャの表
//!   ([`crate::model::MaskTable`]) から `texelFetch` で読む。サンプラは 1 つなので、読めるマスクの数に上限はない。
//! - ライティング (`docs/spec/rendering.md#ライティング`) は参照レンダラ (`crates/ugoe-render/src/shaders/drawable.wgsl`)
//!   と同じ式で、光源は先頭の [`MAX_LIGHTS`] 個まで。
//! - 合成モードは Godot の `blend_premul_alpha` (One / OneMinusSrcAlpha) と `blend_mul` (Dst / Zero) で表す。
//!   - `normal`: `blend_premul_alpha` でそのまま出す。
//!   - `additive`: `blend_premul_alpha` でアルファを 0 にして出す。`src + dst`、アルファは `dst.a` になる。
//!   - `multiply`: `blend_mul` で `rgb + (1 − a)` とアルファ 1 を出す。`src × dst + dst × (1 − src.a)`、アルファは `dst.a`。

use ugoe::lighting::MAX_LIGHTS;

use crate::model::MASK_TABLE_WIDTH;

/// シェーダの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderKind {
    /// 通常の合成と加算 (uniform `additive` で切り替える)。
    PremulAlpha,
    /// 乗算。
    Multiply,
    /// マスクのアトラスに、カバレッジ `c = α × k` をアルファとして描く。
    Coverage,
    /// 前の段のアトラスをそのまま写す (合成しない)。
    Copy,
}

/// マスクを読む部分 (要素とカバレッジで共通)。
const MASK_SOURCE: &str = "
// マスク (docs/spec/rendering.md#マスクの効き具合)。表 mask_table の読み方は src/model.rs の MaskTable。
uniform sampler2D mask_atlas : filter_linear, repeat_disable;
uniform sampler2D mask_table : filter_nearest, repeat_disable;
uniform int mask_first = 0;
uniform int mask_count = 0;
uniform bool mask_intersection = false;
uniform float mask_strength = 1.0;

vec4 table_texel(int i) {
\treturn texelFetch(mask_table, ivec2(i % TABLE_WIDTH, i / TABLE_WIDTH), 0);
}

// カバレッジ source の、キャンバス座標 p での値。矩形の外は 0。
float coverage(int source, vec2 p) {
\tvec4 rect = table_texel(2 * source);
\tvec4 slot = table_texel(2 * source + 1);
\tvec2 q = (p - rect.xy) / rect.zw;
\tif (q.x < 0.0 || q.y < 0.0 || q.x > 1.0 || q.y > 1.0) {
\t\treturn 0.0;
\t}
\treturn textureLod(mask_atlas, slot.xy + q * slot.zw, 0.0).a;
}

// マスク係数 k。
float mask_factor(vec2 p) {
\tif (mask_count <= 0) {
\t\treturn 1.0;
\t}
\tbool has_pos = false;
\tfloat pos_union = 1.0;
\tfloat pos_all = 1.0;
\tfloat neg_union = 1.0;
\tfor (int i = 0; i < mask_count; i++) {
\t\tvec4 r = table_texel(mask_first + i);
\t\tfloat c = coverage(int(r.x + 0.5), p);
\t\tif (r.y > 0.5) {
\t\t\tneg_union *= 1.0 - c;
\t\t} else {
\t\t\thas_pos = true;
\t\t\tpos_union *= 1.0 - c;
\t\t\tpos_all *= c;
\t\t}
\t}
\tfloat pos = has_pos ? (mask_intersection ? pos_all : 1.0 - pos_union) : 1.0;
\tfloat m = pos * neg_union;
\treturn 1.0 - clamp(mask_strength, 0.0, 1.0) * (1.0 - m);
}
";

/// ライティングの部分 (要素だけ)。
const LIGHTING_SOURCE: &str = "
// ライティング (docs/spec/rendering.md#ライティング)。lit_mode: 0 = 受けない、1 = 法線 (0, 0, 1)、2 = 法線マップを読む。
uniform int lit_mode = 0;
uniform sampler2D normal_map : filter_linear, repeat_disable;
uniform vec3 ambient_light = vec3(1.0);
uniform int light_count = 0;
// 平行光源: xyz = 光源へ向かう単位ベクトル, w = 0。点光源: xyz = 位置, w = 1。
uniform vec4 light_vectors[MAX_LIGHTS];
// rgb = 色, a = 点光源の 1 / range。
uniform vec4 light_colors[MAX_LIGHTS];

// 画素の法線 (キャンバス座標 + 奥行き、単位ベクトル)。微分を使うので、一様な制御フローで呼ぶ。
vec3 surface_normal(vec2 uv, vec2 canvas) {
\tvec3 c = texture(normal_map, uv).rgb * 2.0 - 1.0;
\t// 法線マップの G は画像の上向きなので、UV 空間 (v は下向き) では y の符号が逆になる。
\tvec3 n = lit_mode == 2 ? vec3(c.x, -c.y, c.z) : vec3(0.0, 0.0, 1.0);
\t// UV 空間 → キャンバス座標の写像のヤコビ行列の列 t = dP/du, b = dP/dv (画面での微分から求める)。
\tvec2 dp_x = dFdx(canvas);
\tvec2 dp_y = dFdy(canvas);
\tvec2 duv_x = dFdx(uv);
\tvec2 duv_y = dFdy(uv);
\tfloat s = sign(duv_x.x * duv_y.y - duv_y.x * duv_x.y);
\tvec2 t = (dp_x * duv_y.y - dp_y * duv_x.y) * s;
\tvec2 b = (dp_y * duv_x.x - dp_x * duv_y.x) * s;
\t// 反転していれば u 軸を反転した写像の回転成分を使い、法線の x の符号を反転する。
\tif (t.x * b.y - b.x * t.y < 0.0) {
\t\tt = -t;
\t\tn.x = -n.x;
\t}
\t// 回転成分 (極分解の回転) の cos, sin。写像が潰れていれば回さない。
\tvec2 r = vec2(t.x + b.y, t.y - b.x);
\tfloat r_len = length(r);
\tvec2 cs = r_len > 0.0 ? r / r_len : vec2(1.0, 0.0);
\tvec3 v = vec3(cs.x * n.x - cs.y * n.y, cs.y * n.x + cs.x * n.y, n.z);
\tfloat v_len = length(v);
\treturn v_len > 0.0 ? v / v_len : vec3(0.0, 0.0, 1.0);
}

// light = ambient + Σ color_i × max(0, dot(n, l_i)) × attenuation_i
vec3 light_at(vec2 canvas, vec3 n) {
\tvec3 light = ambient_light;
\tvec3 p = vec3(canvas, 0.0);
\tfor (int i = 0; i < min(light_count, MAX_LIGHTS); i++) {
\t\tvec4 source = light_vectors[i];
\t\tvec3 l = source.xyz;
\t\tfloat attenuation = 1.0;
\t\tif (source.w > 0.5) {
\t\t\tvec3 to_light = source.xyz - p;
\t\t\tfloat d = length(to_light);
\t\t\tl = d > 0.0 ? to_light / d : vec3(0.0, 0.0, 1.0);
\t\t\tfloat falloff = clamp(1.0 - d * light_colors[i].a, 0.0, 1.0);
\t\t\tattenuation = falloff * falloff;
\t\t}
\t\tlight += light_colors[i].rgb * max(0.0, dot(n, l)) * attenuation;
\t}
\treturn light;
}
";

/// シェーダのソース。
pub fn source(kind: ShaderKind) -> String {
    let blend = match kind {
        ShaderKind::Multiply => "blend_mul",
        ShaderKind::PremulAlpha | ShaderKind::Coverage => "blend_premul_alpha",
        ShaderKind::Copy => "blend_disabled",
    };
    let mut s = format!(
        "// ugoe-godot が生成する (bindings/godot/src/shader.rs)。
shader_type canvas_item;
render_mode unshaded, {blend};

const int TABLE_WIDTH = {MASK_TABLE_WIDTH};
const int MAX_LIGHTS = {MAX_LIGHTS};
"
    );
    if kind == ShaderKind::Copy {
        s.push_str(
            "
void fragment() {
\tCOLOR = texture(TEXTURE, UV);
}
",
        );
        return s;
    }
    s.push_str(
        "
// ノードの座標にキャンバスの原点を足すとキャンバス座標になる。
uniform vec2 canvas_origin = vec2(0.0);
uniform float opacity = 1.0;
uniform vec3 multiply_color = vec3(1.0);
uniform vec3 screen_color = vec3(0.0);
uniform bool additive = false;
",
    );
    s.push_str(MASK_SOURCE);
    if kind != ShaderKind::Coverage {
        s.push_str(LIGHTING_SOURCE);
    }
    s.push_str(
        "
varying vec2 canvas_pos;
varying vec4 modulate;

void vertex() {
\tcanvas_pos = VERTEX + canvas_origin;
\tmodulate = COLOR;
}

void fragment() {
\tfloat k = mask_factor(canvas_pos);
\tvec4 c = texture(TEXTURE, UV);
",
    );
    match kind {
        ShaderKind::Coverage => s.push_str("\tCOLOR = vec4(0.0, 0.0, 0.0, c.a * k);\n"),
        _ => {
            s.push_str(
                "\tvec3 normal = surface_normal(UV, canvas_pos);
\t// docs/spec/rendering.md#描画可能要素ごとのシェーディング
\tvec3 rgb = c.rgb * multiply_color;
\trgb = rgb + screen_color * c.a - rgb * screen_color;
\t// ライティング (乗算色・スクリーン色の後、不透明度とマスクの前)
\tif (lit_mode > 0) {
\t\trgb = rgb * light_at(canvas_pos, normal);
\t}
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

    const KINDS: [ShaderKind; 4] = [
        ShaderKind::PremulAlpha,
        ShaderKind::Multiply,
        ShaderKind::Coverage,
        ShaderKind::Copy,
    ];

    #[test]
    fn braces_balance() {
        for kind in KINDS {
            let src = source(kind);
            assert_eq!(
                src.matches('{').count(),
                src.matches('}').count(),
                "{kind:?}"
            );
            assert_eq!(
                src.matches('(').count(),
                src.matches(')').count(),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn masks_are_read_from_the_table() {
        for kind in [
            ShaderKind::PremulAlpha,
            ShaderKind::Multiply,
            ShaderKind::Coverage,
        ] {
            let src = source(kind);
            assert!(src.contains("uniform sampler2D mask_atlas"));
            assert!(src.contains("for (int i = 0; i < mask_count; i++)"));
            assert!(src.contains(&format!("const int TABLE_WIDTH = {MASK_TABLE_WIDTH};")));
        }
        assert!(!source(ShaderKind::Copy).contains("mask_atlas"));
    }

    #[test]
    fn lighting_is_only_for_drawables() {
        assert!(source(ShaderKind::PremulAlpha).contains("light_at(canvas_pos, normal)"));
        assert!(source(ShaderKind::Multiply).contains("uniform vec4 light_vectors[MAX_LIGHTS];"));
        assert!(!source(ShaderKind::Coverage).contains("light_at"));
    }

    #[test]
    fn blend_modes_follow_the_spec() {
        assert!(source(ShaderKind::PremulAlpha).contains("blend_premul_alpha"));
        assert!(source(ShaderKind::PremulAlpha).contains("additive ? vec4(o.rgb, 0.0) : o"));
        assert!(source(ShaderKind::Multiply).contains("blend_mul"));
        assert!(source(ShaderKind::Multiply).contains("o.rgb + (1.0 - o.a)"));
        assert!(source(ShaderKind::Coverage).contains("c.a * k"));
        assert!(source(ShaderKind::Copy).contains("blend_disabled"));
    }
}
