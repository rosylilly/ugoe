// SPDX-License-Identifier: MIT
// Linear の色空間で、中間のテクスチャ (Runtime/Unity/UgoeIntermediate.cs) を画面に写すシェーダ。
// 中間のテクスチャには、sRGB の値のまま (参照実装と同じく) 合成した結果が乗算済みアルファで入っている。
// ストレートの色を線形に直してアルファを掛け直し、書き込み先 (sRGB) に乗算済みアルファで合成する。
// 不透明な画素は書き込み先の変換 (線形 → sRGB) で中間のテクスチャの値に戻る。
Shader "Hidden/Ugoe/Composite"
{
    Properties
    {
        _MainTex ("Intermediate (premultiplied, sRGB values)", 2D) = "clear" {}
    }
    SubShader
    {
        Tags { "Queue" = "Transparent" "RenderType" = "Transparent" "IgnoreProjector" = "True" "PreviewType" = "Plane" }
        Pass
        {
            Blend One OneMinusSrcAlpha
            ZWrite Off
            ZTest LEqual
            Cull Off
            Lighting Off

            HLSLPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 3.0
            #include "UnityCG.cginc"
            #include "UgoeCommon.hlsl"

            sampler2D _MainTex;

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
            };

            struct v2f
            {
                float4 pos : SV_POSITION;
                float2 uv : TEXCOORD0;
            };

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                return o;
            }

            float4 frag(v2f i) : SV_Target
            {
                float4 c = tex2D(_MainTex, i.uv);
            #ifdef UNITY_COLORSPACE_GAMMA
                return c;
            #else
                return UgoeToLinearPremultiplied(c);
            #endif
            }
            ENDHLSL
        }
    }
}
