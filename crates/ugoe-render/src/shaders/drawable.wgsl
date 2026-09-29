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
    // x: m のオフセット, y: mask_strength (マスクを持たなければ 0),
    // z: ライティング (本描画のみ。0 = 受けない, 1 = 法線 (0, 0, 1), 2 = 法線マップを読む), w: 未使用
    params: vec4<f32>,
};

// 光源 1 つ (docs/spec/rendering.md#ライティング)。座標はキャンバス座標 + 奥行き (z は手前向き)。
struct Light {
    // 平行光源: xyz = 光源へ向かう単位ベクトル, w = 0。点光源: xyz = 位置, w = 1
    vector: vec4<f32>,
    // rgb = 色, a = 点光源の 1 / range (平行光源と減衰しない点光源は 0)
    color: vec4<f32>,
};

struct Lighting {
    // rgb = 環境光
    ambient: vec4<f32>,
    // x = 光源の数
    count: vec4<u32>,
    lights: array<Light, MAX_LIGHTS>,
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
@group(0) @binding(2) var<uniform> lighting: Lighting;
@group(1) @binding(0) var<uniform> draw: DrawUniforms;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var tex_sampler: sampler;
// テクスチャの法線マップ (RGB = 接空間法線 n = 2c − 1。G は画像の上向き)。ないときはダミーを束縛する。
@group(2) @binding(2) var normal_tex: texture_2d<f32>;
@group(3) @binding(0) var mask_tex: texture_2d<f32>;
@group(3) @binding(1) var mask_sampler: sampler;

const ELEMENT_TEXELS: u32 = 5u;
const MAX_LIGHTS: u32 = 8u;

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
    // キャンバス座標 (ライティングと、法線を回すヤコビ行列に使う)
    @location(7) canvas: vec2<f32>,
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
    out.canvas = in.position;
    return out;
}

// 画素の法線 (キャンバス座標 + 奥行き、単位ベクトル)。docs/spec/rendering.md#ライティング
// `mode` はライティングの種類 (MaskRead::params.z)。微分とサンプリングを使うので、一様な制御フローで呼ぶ。
fn surface_normal(uv: vec2<f32>, canvas: vec2<f32>, mode: f32) -> vec3<f32> {
    let c = textureSample(normal_tex, tex_sampler, uv).rgb * 2.0 - 1.0;
    // 法線マップの G は画像の上向きなので、UV 空間 (v は下向き) では y の符号が逆になる。
    var n = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(c.x, -c.y, c.z), mode > 1.5);
    // UV 空間 → キャンバス座標の写像のヤコビ行列の列 t = ∂P/∂u, b = ∂P/∂v。
    // 画面での微分から求める (三角形の中で一定)。正の倍数は向きに効かないので det(∂uv) の符号だけを掛ける。
    let dp_x = dpdx(canvas);
    let dp_y = dpdy(canvas);
    let duv_x = dpdx(uv);
    let duv_y = dpdy(uv);
    let s = sign(duv_x.x * duv_y.y - duv_y.x * duv_x.y);
    var t = (dp_x * duv_y.y - dp_y * duv_x.y) * s;
    let b = (dp_y * duv_x.x - dp_x * duv_y.x) * s;
    // 反転していれば u 軸を反転した写像の回転成分を使い、法線の x の符号を反転する。
    if t.x * b.y - b.x * t.y < 0.0 {
        t = -t;
        n.x = -n.x;
    }
    // 回転成分 (極分解の回転) の cos, sin。写像が潰れていれば回さない。
    let r = vec2<f32>(t.x + b.y, t.y - b.x);
    let r_len = length(r);
    let cs = select(vec2<f32>(1.0, 0.0), r / r_len, r_len > 0.0);
    let v = vec3<f32>(cs.x * n.x - cs.y * n.y, cs.y * n.x + cs.x * n.y, n.z);
    let v_len = length(v);
    return select(vec3<f32>(0.0, 0.0, 1.0), v / v_len, v_len > 0.0);
}

// light = ambient + Σ color_i × max(0, dot(n, l_i)) × attenuation_i (docs/spec/rendering.md#ライティング)
fn light_at(canvas: vec2<f32>, n: vec3<f32>) -> vec3<f32> {
    var light = lighting.ambient.rgb;
    let p = vec3<f32>(canvas, 0.0);
    let count = min(lighting.count.x, MAX_LIGHTS);
    for (var i = 0u; i < count; i++) {
        let source = lighting.lights[i];
        var l = source.vector.xyz;
        var attenuation = 1.0;
        if source.vector.w > 0.5 {
            let to_light = source.vector.xyz - p;
            let d = length(to_light);
            l = select(vec3<f32>(0.0, 0.0, 1.0), to_light / d, d > 0.0);
            let falloff = clamp(1.0 - d * source.color.a, 0.0, 1.0);
            attenuation = falloff * falloff;
        }
        light += source.color.rgb * max(0.0, dot(n, l)) * attenuation;
    }
    return light;
}

@fragment
fn fs_main(in: MainOut) -> @location(0) vec4<f32> {
    // テクスチャは乗算済みアルファで格納されている前提
    var c = textureSample(tex, tex_sampler, in.uv);
    let lit = in.mask_params.z;
    let normal = surface_normal(in.uv, in.canvas, lit);
    // 乗算色 (rgb のみ)
    var rgb = c.rgb * in.multiply_color.rgb;
    // スクリーン色: s + d - s*d を乗算済み空間で適用
    rgb = rgb + in.screen_color.rgb * c.a - rgb * in.screen_color.rgb;
    // ライティング (乗算済みのまま。乗算色・スクリーン色の後、不透明度とマスクの前)
    if lit > 0.5 {
        rgb = rgb * light_at(in.canvas, normal);
    }
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
