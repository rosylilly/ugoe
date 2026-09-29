// ugoe 参照シェーダ: 本描画 (vs_main / fs_main) と、マスクパス (vs_mask / fs_mask, vs_fill / fs_fill)。
// 式の正は docs/spec/rendering.md。エンジン移植時は naga で HLSL/GLSL/MSL/SPIR-V に変換して使う。
//
// 出力は乗算済みアルファ (premultiplied alpha)。

struct Globals {
    // キャンバス座標 → クリップ座標
    view_proj: mat3x3<f32>,
};

// 要素自身のマスクの読み方 (マスクパスでは、マスク用メッシュ自身のマスク = 入れ子の継承)。
struct MaskRead {
    // 画面 UV → マスクアトラス UV の変換: atlas_uv = xy + screen_uv * zw
    rect: vec4<f32>,
    // マスクの係数 m = params.x + dot(アトラスの値, channel)
    // (channel は符号付きの one-hot。和集合として読むときは x = 1, チャンネル = -1)
    channel: vec4<f32>,
    // x: m のオフセット, y: mask_strength (マスクを持たなければ 0), zw: 未使用
    params: vec4<f32>,
};

// マスクパスでマスク用メッシュを 1 回描く分のユニフォーム。
struct DrawUniforms {
    multiply_color: vec4<f32>, // 未使用
    screen_color: vec4<f32>,   // 未使用
    mask: MaskRead,
    // --- 書き込み先 ---
    // 画面 UV → 書き込み先アトラス UV の変換 (内容の外接矩形を担当セルに写す)
    target_rect: vec4<f32>,
    // 書き込むチャンネル (one-hot。A は作業領域)
    target_channel: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
// 本描画の要素ごとの値 (docs/spec/rendering.md#描画の最適化)。1 要素 = ELEMENT_TEXELS 個の texel を横に並べ、
// 1 行に textureDimensions().x / ELEMENT_TEXELS 要素を詰める。
// texel: 0 = 乗算色 (a = 不透明度), 1 = スクリーン色, 2..4 = マスクの読み方 (rect, channel, params)
@group(0) @binding(1) var element_data: texture_2d<f32>;
@group(1) @binding(0) var<uniform> draw: DrawUniforms;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var tex_sampler: sampler;
@group(3) @binding(0) var mask_tex: texture_2d<f32>;
@group(3) @binding(1) var mask_sampler: sampler;

const ELEMENT_TEXELS: u32 = 5u;

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

// 本描画の頂点入力。複数の要素を 1 回で描くので、頂点ごとに要素の番号を持つ。
struct MainIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) element: u32,
};

// 本描画の頂点出力。要素ごとの値は三角形の中で一定なので flat で渡す。
struct MainOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) screen_uv: vec2<f32>,
    @location(2) @interpolate(flat) multiply_color: vec4<f32>, // rgb, a = opacity
    @location(3) @interpolate(flat) screen_color: vec4<f32>,   // rgb, a = 未使用
    @location(4) @interpolate(flat) mask_rect: vec4<f32>,
    @location(5) @interpolate(flat) mask_channel: vec4<f32>,
    @location(6) @interpolate(flat) mask_params: vec4<f32>,
};

fn element_texel(element: u32, texel: u32) -> vec4<f32> {
    let per_row = textureDimensions(element_data).x / ELEMENT_TEXELS;
    let x = (element % per_row) * ELEMENT_TEXELS + texel;
    return textureLoad(element_data, vec2<u32>(x, element / per_row), 0);
}

fn to_screen_uv(clip: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(clip.x * 0.5 + 0.5, 0.5 - clip.y * 0.5);
}

// マスク係数 k = 1 − s × (1 − m) (docs/spec/rendering.md#マスク)。マスクを持たなければ s = 0 で 1。
fn mask_factor(screen_uv: vec2<f32>, mask: MaskRead) -> f32 {
    let mask_uv = mask.rect.xy + screen_uv * mask.rect.zw;
    let m = mask.params.x + dot(textureSample(mask_tex, mask_sampler, mask_uv), mask.channel);
    return 1.0 - mask.params.y * (1.0 - m);
}

@vertex
fn vs_main(in: MainIn) -> MainOut {
    let p = globals.view_proj * vec3<f32>(in.position, 1.0);
    var out: MainOut;
    out.clip = vec4<f32>(p.xy, 0.0, 1.0);
    out.uv = in.uv;
    out.screen_uv = to_screen_uv(p.xy);
    out.multiply_color = element_texel(in.element, 0u);
    out.screen_color = element_texel(in.element, 1u);
    out.mask_rect = element_texel(in.element, 2u);
    out.mask_channel = element_texel(in.element, 3u);
    out.mask_params = element_texel(in.element, 4u);
    return out;
}

@fragment
fn fs_main(in: MainOut) -> @location(0) vec4<f32> {
    // テクスチャは乗算済みアルファで格納されている前提
    var c = textureSample(tex, tex_sampler, in.uv);
    // 乗算色 (rgb のみ)
    var rgb = c.rgb * in.multiply_color.rgb;
    // スクリーン色: s + d - s*d を乗算済み空間で適用
    rgb = rgb + in.screen_color.rgb * c.a - rgb * in.screen_color.rgb;
    c = vec4<f32>(rgb, c.a) * in.multiply_color.a;
    return c * mask_factor(in.screen_uv, MaskRead(in.mask_rect, in.mask_channel, in.mask_params));
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
    let coverage = textureSample(tex, tex_sampler, in.uv).a * mask_factor(in.screen_uv, draw.mask);
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
