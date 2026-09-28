// ugoe 参照シェーダ: 描画可能要素 1 つ分 (vs_main / fs_main) と、マスクパス (vs_mask / fs_mask)。
// 式の正は docs/spec/rendering.md。エンジン移植時は naga で HLSL/GLSL/MSL/SPIR-V に変換して使う。
//
// 出力は乗算済みアルファ (premultiplied alpha)。

struct Globals {
    // キャンバス座標 → クリップ座標
    view_proj: mat3x3<f32>,
};

// 本描画では描画可能要素 1 つ分、マスクパスではマスク用メッシュ 1 つ分。
struct DrawUniforms {
    multiply_color: vec4<f32>, // rgb, a = opacity (マスクパスでは未使用)
    screen_color: vec4<f32>,   // rgb, a = 未使用 (マスクパスでは未使用)
    // x: マスクを使うか (0/1), y: 反転マスク (0/1), zw: 未使用 (マスクパスでは未使用)
    mask_flags: vec4<f32>,
    // 画面 UV → マスクアトラス UV の変換: atlas_uv = xy + screen_uv * zw
    // (対象要素の画面上の外接矩形を、アトラス内の担当セルに写す)
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
    // マスクアトラス参照用 (クリップ座標を 0..1、左上原点に写したもの)
    @location(1) screen_uv: vec2<f32>,
};

fn to_screen_uv(clip: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(clip.x * 0.5 + 0.5, 0.5 - clip.y * 0.5);
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let p = globals.view_proj * vec3<f32>(in.position, 1.0);
    var out: VsOut;
    out.clip = vec4<f32>(p.xy, 0.0, 1.0);
    out.uv = in.uv;
    out.screen_uv = to_screen_uv(p.xy);
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

// マスクパス: マスク用メッシュを、画面上の位置のままアトラスの担当セルに写して描く。
@vertex
fn vs_mask(in: VsIn) -> VsOut {
    let p = globals.view_proj * vec3<f32>(in.position, 1.0);
    let atlas_uv = draw.mask_rect.xy + to_screen_uv(p.xy) * draw.mask_rect.zw;
    var out: VsOut;
    out.clip = vec4<f32>(atlas_uv.x * 2.0 - 1.0, 1.0 - atlas_uv.y * 2.0, 0.0, 1.0);
    out.uv = in.uv;
    out.screen_uv = atlas_uv;
    return out;
}

// カバレッジ = テクスチャのアルファ。マスク用メッシュの不透明度・色は使わない。
// 担当チャンネルにだけ値を出し、加算合成 (RGBA8 で 1 に飽和) で和集合 min(1, Σ) を作る。
@fragment
fn fs_mask(in: VsOut) -> @location(0) vec4<f32> {
    let coverage = textureSample(tex, tex_sampler, in.uv).a;
    return draw.mask_channel * coverage;
}
