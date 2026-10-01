// SPDX-License-Identifier: MIT
// ugoe の描画可能要素を描くシェーダ (spec/rendering.md の Mesh color・Blend modes・Masks・Lighting)。
// 描画順で隣り合う要素を 1 つのメッシュにまとめて描く (docs/spec/rendering.md#描画の最適化、Runtime/Core/DrawBatches.cs)ので、
// 要素ごとの値 (乗算色と不透明度・スクリーン色・マスクの効き具合・ライティングの種類) は頂点属性で受け取る。
// テクスチャ・合成モード・読むマスクの内容はバッチごと (MaterialPropertyBlock と材質)。
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
                float4 color : COLOR;      // 乗算色 (rgb) と不透明度 (a)
                float4 screen : TEXCOORD2; // スクリーン色 (rgb)
                float2 extra : TEXCOORD3;  // x = mask_strength (マスクを読まなければ 0)、y = ライティング (0 = なし、1 = 法線 (0, 0, 1)、2 = 法線マップ)
            };

            struct v2f
            {
                float4 pos : SV_POSITION;
                float2 uv : TEXCOORD0;
                float2 canvas : TEXCOORD1;
                // 1 つの三角形の頂点はすべて同じ要素なので、補間しても値は変わらない。
                float4 color : TEXCOORD2;
                float4 screen : TEXCOORD3;
                float2 extra : TEXCOORD4;
            };

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                o.canvas = v.canvas;
                o.color = v.color;
                o.screen = v.screen;
                o.extra = v.extra;
                return o;
            }

            // 画素の法線 (spec/rendering.md の Normals)。ugoe の UV (v は下向き) とキャンバス座標の画面での微分から、
            // UV → キャンバスの写像のヤコビ行列を求めて回す。画面の y の向きが API で違っても結果は変わらない。
            float3 SurfaceNormal(float2 unityUv, float2 canvas, float litMode)
            {
                float3 c = tex2D(_NormalTex, unityUv).rgb * 2.0 - 1.0;
                float3 n = litMode > 1.5 ? float3(c.x, -c.y, c.z) : float3(0, 0, 1);
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
                float3 normal = SurfaceNormal(i.uv, i.canvas, i.extra.y);
                float3 rgb = c.rgb * i.color.rgb;
                rgb = rgb + i.screen.rgb * c.a - rgb * i.screen.rgb;
                if (i.extra.y > 0.5)
                {
                    rgb *= LightAt(i.canvas, normal);
                }
                float4 o = float4(rgb, c.a) * i.color.a * UgoeMaskFactorWith(i.canvas, i.extra.x);
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
