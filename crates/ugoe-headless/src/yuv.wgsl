// 描画先 (RGBA8) を NV12 か YUY2 に変換する計算シェーダ。式は yuv.rs (CPU の参照実装) と同じ:
// BT.709 の限定範囲。色差は隣り合う画素の R'G'B' を平均してから変換する。1 回の呼び出しで 4 バイト (u32 の 1 語) を書く。

struct Params {
    width: u32,
    height: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

fn luma(c: vec3<f32>) -> f32 {
    return 16.0 + 219.0 * dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn chroma(c: vec3<f32>) -> vec2<f32> {
    let y = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    return vec2<f32>(128.0 + 224.0 * (c.b - y) / 1.8556, 128.0 + 224.0 * (c.r - y) / 1.5748);
}

fn to_byte(v: f32) -> u32 {
    return u32(clamp(floor(v + 0.5), 0.0, 255.0));
}

fn texel(x: u32, y: u32) -> vec3<f32> {
    return textureLoad(source, vec2<i32>(i32(x), i32(y)), 0).rgb;
}

fn pack(a: u32, b: u32, c: u32, d: u32) -> u32 {
    return a | (b << 8u) | (c << 16u) | (d << 24u);
}

// NV12 の Y の面: 横の 4 画素を 1 語に。
@compute @workgroup_size(8, 8)
fn nv12_luma(@builtin(global_invocation_id) id: vec3<u32>) {
    let words = params.width / 4u;
    if (id.x >= words || id.y >= params.height) {
        return;
    }
    let x = id.x * 4u;
    output[id.y * words + id.x] = pack(
        to_byte(luma(texel(x, id.y))),
        to_byte(luma(texel(x + 1u, id.y))),
        to_byte(luma(texel(x + 2u, id.y))),
        to_byte(luma(texel(x + 3u, id.y))),
    );
}

// NV12 の色差の面: 2×2 の画素ごとに Cb Cr。横の 2 組 (4 画素 × 2 行) を 1 語に。
@compute @workgroup_size(8, 8)
fn nv12_chroma(@builtin(global_invocation_id) id: vec3<u32>) {
    let words = params.width / 4u;
    if (id.x >= words || id.y >= params.height / 2u) {
        return;
    }
    let x = id.x * 4u;
    let y = id.y * 2u;
    let c0 = chroma((texel(x, y) + texel(x + 1u, y) + texel(x, y + 1u) + texel(x + 1u, y + 1u)) / 4.0);
    let c1 = chroma(
        (texel(x + 2u, y) + texel(x + 3u, y) + texel(x + 2u, y + 1u) + texel(x + 3u, y + 1u)) / 4.0,
    );
    let luma_words = words * params.height;
    output[luma_words + id.y * words + id.x] = pack(
        to_byte(c0.x), to_byte(c0.y), to_byte(c1.x), to_byte(c1.y),
    );
}

// YUY2: 横の 2 画素ごとに Y0 Cb Y1 Cr。
@compute @workgroup_size(8, 8)
fn yuy2(@builtin(global_invocation_id) id: vec3<u32>) {
    let words = params.width / 2u;
    if (id.x >= words || id.y >= params.height) {
        return;
    }
    let p0 = texel(id.x * 2u, id.y);
    let p1 = texel(id.x * 2u + 1u, id.y);
    let c = chroma((p0 + p1) / 2.0);
    output[id.y * words + id.x] = pack(
        to_byte(luma(p0)), to_byte(c.x), to_byte(luma(p1)), to_byte(c.y),
    );
}
