// ugoe 参照シェーダ: 描画可能要素 1 つ分 (vs_main / fs_main) と、マスクパス (vs_mask / fs_mask, vs_fill / fs_fill)。
// 式の正は docs/spec/rendering.md。エンジン移植時は naga で HLSL/GLSL/MSL/SPIR-V に変換して使う。
//
// 出力は乗算済みアルファ (premultiplied alpha)。

struct Globals {
    // キャンバス座標 → クリップ座標
    view_proj: mat3x3<f32>,
};

// 本描画では描画可能要素 1 つ分、マスクパスではマスク用メッシュ 1 回の描画分。
struct DrawUniforms {
    multiply_color: vec4<f32>, // rgb, a = opacity (マスクパスでは未使用)
    screen_color: vec4<f32>,   // rgb, a = 未使用 (マスクパスでは未使用)
    // --- この要素自身のマスク (マスクパスでは、マスク用メッシュ自身のマスク = 入れ子の継承) ---
    // 画面 UV → マスクアトラス UV の変換: atlas_uv = xy + screen_uv * zw
    mask_rect: vec4<f32>,
    // マスクの係数 m = mask_params.x + dot(アトラスの値, mask_channel)
    // (mask_channel は符号付きの one-hot。和集合として読むときは x = 1, チャンネル = -1)
    mask_channel: vec4<f32>,
    // x: m のオフセット, y: mask_strength (マスクを持たなければ 0), zw: 未使用
    mask_params: vec4<f32>,
    // --- マスクパスだけ: 書き込み先 ---
    // 画面 UV → 書き込み先アトラス UV の変換 (内容の外接矩形を担当セルに写す)
    target_rect: vec4<f32>,
    // 書き込むチャンネル (one-hot。A は作業領域)
    target_channel: vec4<f32>,
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

// マスク係数 k = 1 − s × (1 − m) (docs/spec/rendering.md#マスク)。マスクを持たなければ s = 0 で 1。
fn mask_factor(screen_uv: vec2<f32>) -> f32 {
    let mask_uv = draw.mask_rect.xy + screen_uv * draw.mask_rect.zw;
    let m = draw.mask_params.x + dot(textureSample(mask_tex, mask_sampler, mask_uv), draw.mask_channel);
    return 1.0 - draw.mask_params.y * (1.0 - m);
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
    return c * mask_factor(in.screen_uv);
}

// マスクパス: マスク用メッシュを、画面上の位置のままアトラスの担当セルに写して描く。
@vertex
fn vs_mask(in: VsIn) -> VsOut {
    let p = globals.view_proj * vec3<f32>(in.position, 1.0);
    let screen_uv = to_screen_uv(p.xy);
    let atlas_uv = draw.target_rect.xy + screen_uv * draw.target_rect.zw;
    var out: VsOut;
    out.clip = vec4<f32>(atlas_uv.x * 2.0 - 1.0, 1.0 - atlas_uv.y * 2.0, 0.0, 1.0);
    out.uv = in.uv;
    out.screen_uv = screen_uv;
    return out;
}

// カバレッジ c = テクスチャのアルファ × 自身のマスク係数。マスク用メッシュの不透明度・色は使わない。
// 乗算合成 (dst × src) で、書き込み先のチャンネルにだけ 1 − c を掛ける (他のチャンネルは × 1)。
@fragment
fn fs_mask(in: VsOut) -> @location(0) vec4<f32> {
    let coverage = textureSample(tex, tex_sampler, in.uv).a * mask_factor(in.screen_uv);
    return vec4<f32>(1.0) - draw.target_channel * coverage;
}

// マスクパスの全面描画 (シザー矩形でセルに限る)。作業領域を埋める段と、作業領域をチャンネルに掛ける段で使う。
// 値は合成の設定と書き込みマスクで決まる。
@vertex
fn vs_fill(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    // 画面全体を覆う 1 つの三角形
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_fill() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0);
}
