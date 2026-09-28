// ugoe 参照シェーダ: 描画可能要素 1 つ分。
// 式の正は docs/spec/rendering.md。エンジン移植時は naga で HLSL/GLSL/MSL/SPIR-V に変換して使う。
//
// 出力は乗算済みアルファ (premultiplied alpha)。

struct Globals {
    // キャンバス座標 → クリップ座標
    view_proj: mat3x3<f32>,
};

struct DrawUniforms {
    multiply_color: vec4<f32>, // rgb, a = opacity
    screen_color: vec4<f32>,   // rgb, a = 未使用
    // x: マスクを使うか (0/1), y: 反転マスク (0/1), zw: 未使用
    mask_flags: vec4<f32>,
    // マスクアトラス内の矩形 (xy = offset, zw = size, いずれも 0..1)
    mask_rect: vec4<f32>,
    // マスクアトラスのチャンネル選択 (one-hot)
    mask_channel: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> draw: DrawUniforms;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var tex_sampler: sampler;
@group(3) @binding(0) var mask_tex: texture_2d<f32>;
@group(3) @binding(1) var mask_sampler: sampler;

struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // マスクアトラス参照用 (クリップ座標を 0..1 に写したもの)
    @location(1) screen_uv: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let p = globals.view_proj * vec3<f32>(in.position, 1.0);
    var out: VsOut;
    out.clip = vec4<f32>(p.xy, 0.0, 1.0);
    out.uv = in.uv;
    out.screen_uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // テクスチャは乗算済みアルファで格納されている前提
    var c = textureSample(tex, tex_sampler, in.uv);
    // 乗算色 (rgb のみ)
    var rgb = c.rgb * draw.multiply_color.rgb;
    // スクリーン色: s + d - s*d を乗算済み空間で適用
    rgb = rgb + draw.screen_color.rgb * c.a - rgb * draw.screen_color.rgb;
    c = vec4<f32>(rgb, c.a) * draw.multiply_color.a;

    let mask_uv = draw.mask_rect.xy + in.screen_uv * draw.mask_rect.zw;
    let m = dot(textureSample(mask_tex, mask_sampler, mask_uv), draw.mask_channel);
    let coverage = mix(m, 1.0 - m, draw.mask_flags.y);
    return c * mix(1.0, coverage, draw.mask_flags.x);
}
