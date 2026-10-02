// SPDX-License-Identifier: MIT
// ugoe のマスクの内容を描くシェーダ (spec/rendering.md の Masks)。Runtime/Unity/UgoeMaskRenderer.cs が
// CommandBuffer で使う。書き込み先は内容を並べたページ (ARGB32、1 で初期化) で、内容ごとにその領域をビューポートにして
// キャンバス座標の矩形を写す。
//
// Pass 0 (CoverR) / 1 (CoverG): マスク用メッシュのカバレッジ c = テクスチャのアルファ × 自身のマスク係数を、
//   乗算合成で dst × (1 − c) にする。三角形が重なれば 1 − Π(1 − a) の分だけ掛かる。不透明度・色・カリングは使わない。
// Pass 2 (MultiplyR): 作業用のテクスチャ (_UgoeScratch の R = 1 − c_i。ページと同じ大きさと配置) を読み、R に c_i を掛ける (共通部分)。
Shader "Hidden/Ugoe/Mask"
{
    Properties
    {
        _MainTex ("Texture (premultiplied)", 2D) = "white" {}
        _MaskTex ("Mask", 2D) = "white" {}
        _MaskRect ("Mask rect", Vector) = (0, 0, 1, 1)
        _MaskParams ("Mask params", Vector) = (1, 0, 0, 0)
        _QuadRect ("Quad rect", Vector) = (0, 0, 1, 1)
        _QuadMap ("Quad map", Vector) = (1, 1, 0, 0)
    }
    SubShader
    {
        Tags { "RenderType" = "Transparent" "IgnoreProjector" = "True" }
        ZWrite Off
        ZTest Always
        Cull Off

        HLSLINCLUDE
        #include "UnityCG.cginc"
        #include "UgoeCommon.hlsl"

        sampler2D _MainTex;

        struct appdata
        {
            float4 vertex : POSITION;
            float2 uv : TEXCOORD0;
            float2 canvas : TEXCOORD1;
        };

        struct v2f
        {
            float4 pos : SV_POSITION;
            float2 uv : TEXCOORD0;
            float2 canvas : TEXCOORD1;
        };

        // 位置はキャンバス座標で、ビュー・射影 (キャンバスの矩形 → クリップ) は CommandBuffer が設定する。
        v2f vertCover(appdata v)
        {
            v2f o;
            o.pos = mul(UNITY_MATRIX_VP, float4(v.canvas, 0, 1));
            o.uv = v.uv;
            o.canvas = v.canvas;
            return o;
        }

        float4 fragCover(v2f i) : SV_Target
        {
            float c = tex2D(_MainTex, i.uv).a * UgoeMaskFactor(i.canvas);
            return float4(c, c, c, c);
        }
        ENDHLSL

        Pass
        {
            Name "CoverR"
            Blend Zero OneMinusSrcColor
            ColorMask R
            HLSLPROGRAM
            #pragma vertex vertCover
            #pragma fragment fragCover
            ENDHLSL
        }

        Pass
        {
            Name "CoverG"
            Blend Zero OneMinusSrcColor
            ColorMask G
            HLSLPROGRAM
            #pragma vertex vertCover
            #pragma fragment fragCover
            ENDHLSL
        }

        Pass
        {
            Name "MultiplyR"
            Blend DstColor Zero
            ColorMask R
            HLSLPROGRAM
            #pragma vertex vertQuad
            #pragma fragment fragQuad

            sampler2D _UgoeScratch;
            // xy = 矩形の左上 (キャンバス座標)、zw = 大きさ
            float4 _QuadRect;
            // キャンバス座標から作業用のテクスチャの UV への写像: uv = canvas × xy + zw
            float4 _QuadMap;

            v2f vertQuad(appdata v)
            {
                v2f o;
                float2 canvas = _QuadRect.xy + v.uv * _QuadRect.zw;
                o.pos = mul(UNITY_MATRIX_VP, float4(canvas, 0, 1));
                o.uv = canvas * _QuadMap.xy + _QuadMap.zw;
                o.canvas = canvas;
                return o;
            }

            float4 fragQuad(v2f i) : SV_Target
            {
                // 作業用のテクスチャは書き込み先のページと同じ大きさと配置なので、同じ写像で読める。
                float c = 1.0 - tex2D(_UgoeScratch, i.uv).r;
                return float4(c, c, c, c);
            }
            ENDHLSL
        }
    }
}
