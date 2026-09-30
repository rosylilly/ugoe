// SPDX-License-Identifier: MIT
// ugoe の描画可能要素 1 つを描くシェーダ (spec/rendering.md の Mesh color・Blend modes・Masks・Lighting)。
// 出力は乗算済みアルファ。合成モードは _SrcBlend などで材質ごとに決める (Runtime/Unity/UgoePuppet.cs):
//   normal   = One, OneMinusSrcAlpha / One, OneMinusSrcAlpha
//   additive = One, One              / Zero, One
//   multiply = DstColor, OneMinusSrcAlpha / Zero, One
// 裏面カリングは CPU で三角形を選ぶ (向きはキャンバス座標だけで決まるため) ので、ここでは Cull Off。
Shader "Hidden/Ugoe/Drawable"
{
    Properties
    {
        _MainTex ("Texture (premultiplied)", 2D) = "white" {}
        _NormalTex ("Normal map", 2D) = "bump" {}
        _Color ("Multiply color (a = opacity)", Vector) = (1, 1, 1, 1)
        _ScreenColor ("Screen color", Vector) = (0, 0, 0, 0)
        _LitMode ("Lighting (0 = off, 1 = flat normal, 2 = normal map)", Float) = 0
        _MaskTex ("Mask", 2D) = "white" {}
        _MaskRect ("Mask rect", Vector) = (0, 0, 1, 1)
        _MaskParams ("Mask params", Vector) = (1, 0, 0, 0)
        [HideInInspector] _SrcBlend ("Src", Float) = 1
        [HideInInspector] _DstBlend ("Dst", Float) = 10
        [HideInInspector] _SrcBlendAlpha ("Src alpha", Float) = 1
        [HideInInspector] _DstBlendAlpha ("Dst alpha", Float) = 10
    }
    SubShader
    {
        Tags { "Queue" = "Transparent" "RenderType" = "Transparent" "IgnoreProjector" = "True" "PreviewType" = "Plane" }
        Pass
        {
            Blend [_SrcBlend] [_DstBlend], [_SrcBlendAlpha] [_DstBlendAlpha]
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

            #define UGOE_MAX_LIGHTS 8

            sampler2D _MainTex;
            sampler2D _NormalTex;
            float4 _Color;
            float4 _ScreenColor;
            float _LitMode;

            // 光源 (Ugoe.LightingUniform と同じ中身。材質ごとに設定する)
            float4 _UgoeAmbient;
            float _UgoeLightCount;
            float4 _UgoeLightVectors[UGOE_MAX_LIGHTS];
            float4 _UgoeLightColors[UGOE_MAX_LIGHTS];

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;     // Unity の UV (v は上向き)
                float2 canvas : TEXCOORD1; // キャンバス座標 (ピクセル・Y 下向き)
            };

            struct v2f
            {
                float4 pos : SV_POSITION;
                float2 uv : TEXCOORD0;
                float2 canvas : TEXCOORD1;
            };

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                o.canvas = v.canvas;
                return o;
            }

            // 画素の法線 (spec/rendering.md の Normals)。ugoe の UV (v は下向き) とキャンバス座標の画面での微分から、
            // UV → キャンバスの写像のヤコビ行列を求めて回す。画面の y の向きが API で違っても結果は変わらない。
            float3 SurfaceNormal(float2 unityUv, float2 canvas)
            {
                float3 c = tex2D(_NormalTex, unityUv).rgb * 2.0 - 1.0;
                float3 n = _LitMode > 1.5 ? float3(c.x, -c.y, c.z) : float3(0, 0, 1);
                float2 uv = float2(unityUv.x, 1.0 - unityUv.y);
                float2 dpX = ddx(canvas);
                float2 dpY = ddy(canvas);
                float2 duvX = ddx(uv);
                float2 duvY = ddy(uv);
                float s = sign(duvX.x * duvY.y - duvY.x * duvX.y);
                float2 t = (dpX * duvY.y - dpY * duvX.y) * s;
                float2 b = (dpY * duvX.x - dpX * duvY.x) * s;
                if (t.x * b.y - b.x * t.y < 0.0)
                {
                    t = -t;
                    n.x = -n.x;
                }
                float2 r = float2(t.x + b.y, t.y - b.x);
                float rLen = length(r);
                float2 cs = rLen > 0.0 ? r / rLen : float2(1, 0);
                float3 v = float3(cs.x * n.x - cs.y * n.y, cs.y * n.x + cs.x * n.y, n.z);
                float vLen = length(v);
                return vLen > 0.0 ? v / vLen : float3(0, 0, 1);
            }

            // light = ambient + Σ color × max(0, dot(n, l)) × attenuation (spec/rendering.md の Shading)
            float3 LightAt(float2 canvas, float3 n)
            {
                float3 light = _UgoeAmbient.rgb;
                float3 p = float3(canvas, 0);
                int count = (int)min(_UgoeLightCount, UGOE_MAX_LIGHTS);
                for (int i = 0; i < UGOE_MAX_LIGHTS; i++)
                {
                    if (i >= count) break;
                    float4 vec = _UgoeLightVectors[i];
                    float4 col = _UgoeLightColors[i];
                    float3 l = vec.xyz;
                    float attenuation = 1.0;
                    if (vec.w > 0.5)
                    {
                        float3 toLight = vec.xyz - p;
                        float d = length(toLight);
                        l = d > 0.0 ? toLight / d : float3(0, 0, 1);
                        float falloff = saturate(1.0 - d * col.a);
                        attenuation = falloff * falloff;
                    }
                    light += col.rgb * max(0.0, dot(n, l)) * attenuation;
                }
                return light;
            }

            float4 frag(v2f i) : SV_Target
            {
                // テクスチャは読み込み時に乗算済みアルファにしてある (Runtime/Unity/UgoeTextures.cs)。
                float4 c = tex2D(_MainTex, i.uv);
                // 微分を使うので、一様な制御フローで求める。
                float3 normal = SurfaceNormal(i.uv, i.canvas);
                float3 rgb = c.rgb * _Color.rgb;
                rgb = rgb + _ScreenColor.rgb * c.a - rgb * _ScreenColor.rgb;
                if (_LitMode > 0.5)
                {
                    rgb *= LightAt(i.canvas, normal);
                }
                float4 o = float4(rgb, c.a) * _Color.a * UgoeMaskFactor(i.canvas);
            #ifndef UNITY_COLORSPACE_GAMMA
                // リニアの色空間では書き込み先が sRGB で、出力は線形の値として扱われる。ugoe の色は sRGB の値を
                // そのまま使う約束なので、ストレートの色を線形に直して書く (合成は線形で行われるので、半透明の
                // 部分は参照と少しずれる。一致させるにはプロジェクトの色空間を Gamma にする。README を参照)。
                o.rgb = o.a > 0.0 ? GammaToLinearSpace(saturate(o.rgb / o.a)) * o.a : GammaToLinearSpace(o.rgb);
            #endif
                return o;
            }
            ENDHLSL
        }
    }
}
