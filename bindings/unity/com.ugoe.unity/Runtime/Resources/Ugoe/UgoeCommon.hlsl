// SPDX-License-Identifier: MIT
// ugoe の Unity 用シェーダの共通部分。式の正は spec/rendering.md (日本語は docs/spec/rendering.md)。
// 参照シェーダ (crates/ugoe-render/src/shaders/drawable.wgsl) と同じ式を HLSL で書いたもの。
#ifndef UGOE_COMMON_INCLUDED
#define UGOE_COMMON_INCLUDED

// --- マスク (spec/rendering.md の Masks) ---
// 要素が読むマスクの内容のテクスチャ。キャンバス座標の矩形に描かれている (Runtime/Unity/UgoeMaskRenderer.cs)。
//   R: 和集合なら Π(1 − c)、共通部分なら Π c (通常マスク)。G: Π(1 − c) (反転マスク)。
sampler2D _MaskTex;
// xy = 矩形の左上 (キャンバス座標)、zw = 1 / 大きさ
float4 _MaskRect;
// x, y: 通常マスクの結合値 pos = x + y × R、z: mask_strength、w: マスクを持つなら 1
float4 _MaskParams;

// マスク係数 k = 1 − s × (1 − m)、m = pos × (1 − neg)。マスクを持たなければ 1。
float UgoeMaskFactor(float2 canvas)
{
    if (_MaskParams.w < 0.5) return 1.0;
    float2 uv = (canvas - _MaskRect.xy) * _MaskRect.zw;
    float4 v = tex2Dlod(_MaskTex, float4(uv, 0, 0));
    float pos = _MaskParams.x + _MaskParams.y * v.r;
    float m = pos * v.g;
    return 1.0 - _MaskParams.z * (1.0 - m);
}

#endif
