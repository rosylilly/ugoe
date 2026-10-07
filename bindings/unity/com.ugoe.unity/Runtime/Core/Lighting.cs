// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;

namespace Ugoe
{
    /// <summary>光源の種類 (<c>UGOE_LIGHT_*</c>)。</summary>
    public enum LightKind
    {
        Directional = 0,
        Point = 1,
    }

    /// <summary>
    /// 光源 1 つ (docs/spec/rendering.md#ライティング)。座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (z、手前向き)。
    /// </summary>
    public readonly struct LightSource
    {
        public readonly LightKind Kind;
        /// <summary>平行光源では光が進む向き (長さは問わない)、点光源では位置。</summary>
        public readonly float X, Y, Z;
        /// <summary>色 (1 を超えてもよい)。</summary>
        public readonly float R, G, B;
        /// <summary>点光源の、明るさが 0 になる距離 (正。無限大なら減衰しない)。</summary>
        public readonly float Range;

        public LightSource(LightKind kind, float x, float y, float z, float r, float g, float b, float range = float.PositiveInfinity)
        {
            Kind = kind;
            X = x;
            Y = y;
            Z = z;
            R = r;
            G = g;
            B = b;
            Range = range;
        }

        public static LightSource Directional(float dx, float dy, float dz, float r, float g, float b) =>
            new LightSource(LightKind.Directional, dx, dy, dz, r, g, b);

        public static LightSource Point(float x, float y, float z, float r, float g, float b, float range) =>
            new LightSource(LightKind.Point, x, y, z, r, g, b, range);
    }

    /// <summary>
    /// シェーダに渡す光源の組 (<c>UgoeLightingUniform</c>。参照シェーダの <c>Lighting</c> と同じ中身)。
    /// </summary>
    public sealed class LightingUniform
    {
        /// <summary>参照レンダラが使う光源の数の上限。</summary>
        public const int MaxLights = 8;

        /// <summary>環境光 (rgb、a = 0)。</summary>
        public readonly float[] Ambient = new float[4];

        /// <summary>使う光源の数 (<see cref="Vectors"/> と <see cref="Colors"/> の先頭から)。</summary>
        public int Count;

        /// <summary>光源ごとに 4 つ: 平行光源は xyz = 光源へ向かう単位ベクトル・w = 0、点光源は xyz = 位置・w = 1。</summary>
        public readonly float[] Vectors = new float[MaxLights * 4];

        /// <summary>光源ごとに 4 つ: rgb = 色、a = 点光源の 1 / range (平行光源と減衰しない点光源は 0)。</summary>
        public readonly float[] Colors = new float[MaxLights * 4];

        /// <summary>
        /// 光源の組を検査してシェーダに渡す形にする。環境光の既定は白。向きが 0・有限でない値・range が正でない・負の色は
        /// <see cref="UgoeException"/> (<see cref="UgoeResult.InvalidArgument"/>)。8 個を超える光源は先頭だけを使う。
        /// </summary>
        public static LightingUniform Create(float ambientR, float ambientG, float ambientB, IReadOnlyList<LightSource> lights)
        {
            var count = lights?.Count ?? 0;
            var native = new Native.Light[Math.Max(count, 1)];
            for (var i = 0; i < count; i++)
            {
                var l = lights[i];
                native[i] = new Native.Light
                {
                    Kind = (uint)l.Kind,
                    X = l.X,
                    Y = l.Y,
                    Z = l.Z,
                    R = l.R,
                    G = l.G,
                    B = l.B,
                    Range = l.Range,
                };
            }
            var raw = new float[Native.LightingUniformFloats];
            UgoeException.Check(Native.ugoe_lighting_uniform(new[] { ambientR, ambientG, ambientB }, native, (UIntPtr)count, raw, NativeError.Current));

            var result = new LightingUniform();
            Array.Copy(raw, 0, result.Ambient, 0, 4);
            result.Count = BitConverter.ToInt32(BitConverter.GetBytes(raw[4]), 0);
            for (var i = 0; i < MaxLights; i++)
            {
                Array.Copy(raw, 8 + i * 8, result.Vectors, i * 4, 4);
                Array.Copy(raw, 8 + i * 8 + 4, result.Colors, i * 4, 4);
            }
            return result;
        }

        /// <summary>光源を渡さないとき (環境光が白で光源なし。ライティングを受ける要素も受けない場合と同じ色)。</summary>
        public static LightingUniform Default => Create(1, 1, 1, null);
    }
}
