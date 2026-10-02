// SPDX-License-Identifier: MIT
// ugoe の Unity 用シェーダの共通部分。式の正は spec/rendering.md (日本語は docs/spec/rendering.md)。
// 参照シェーダ (crates/ugoe-render/src/shaders/drawable.wgsl) と同じ式を HLSL で書いたもの。
#ifndef UGOE_COMMON_INCLUDED
#define UGOE_COMMON_INCLUDED

// --- マスク (spec/rendering.md の Masks) ---
// マスクの内容のテクスチャ。内容は入れ子の深さごとに 1 枚 (ページ) に並べて描く (Runtime/Unity/UgoeMaskRenderer.cs、
// Runtime/Core/Offscreen.cs の MaskAtlasLayout)。各内容は、それを読む要素のキャンバス座標の矩形を写す。
//   R: 和集合なら Π(1 − c)、共通部分なら Π c (通常マスク)。G: Π(1 − c) (反転マスク)。
sampler2D _MaskTex;
// キャンバス座標からページの UV への写像: uv = canvas × xy + zw (マスクの内容を描くとき。要素を描くときは頂点属性)
float4 _MaskRect;
// x, y: 通常マスクの結合値 pos = x + y × R、z: mask_strength、w: マスクを読むなら 1 (マスクの内容を描くとき)
float4 _MaskParams;

// マスク係数 k = 1 − s × (1 − m)、m = pos × (1 − neg)。s は mask_strength (マスクを読まなければ 0 にする)。
// map は uv = canvas × map.xy + map.zw、positive は pos = positive.x + positive.y × R の係数。
float UgoeMaskFactorAt(float2 canvas, float4 map, float2 positive, float strength)
{
    float2 uv = canvas * map.xy + map.zw;
    float4 v = tex2Dlod(_MaskTex, float4(uv, 0, 0));
    float pos = positive.x + positive.y * v.r;
    float m = pos * v.g;
    return 1.0 - strength * (1.0 - m);
}

// _MaskRect と _MaskParams から読む (マスクの内容を描くとき。マスク用メッシュが自身のマスクを読む)。マスクを持たなければ 1。
float UgoeMaskFactor(float2 canvas)
{
    if (_MaskParams.w < 0.5) return 1.0;
    return UgoeMaskFactorAt(canvas, _MaskRect, _MaskParams.xy, _MaskParams.z);
}

// --- 色空間 ---
// sRGB の値を線形の値にする (IEC 61966-2-1 の式。Runtime/Core/Offscreen.cs の ColorSpaces と同じ)。
// UnityCG の GammaToLinearSpace は近似 (と half の精度) なので、書き込み先の線形 → sRGB の変換で元の値に戻らない。
float3 UgoeSrgbToLinear(float3 c)
{
    float3 low = c / 12.92;
    float3 high = pow((max(c, 0.0) + 0.055) / 1.055, 2.4);
    return float3(c.r <= 0.04045 ? low.r : high.r, c.g <= 0.04045 ? low.g : high.g, c.b <= 0.04045 ? low.b : high.b);
}

// 乗算済みアルファの sRGB の値を、乗算済みアルファの線形の値にする (ストレートの色を直してからアルファを掛け直す)。
float4 UgoeToLinearPremultiplied(float4 o)
{
    float3 straight = o.a > 0.0 ? saturate(o.rgb / o.a) : o.rgb;
    return float4(UgoeSrgbToLinear(straight) * o.a, o.a);
}

#endif
