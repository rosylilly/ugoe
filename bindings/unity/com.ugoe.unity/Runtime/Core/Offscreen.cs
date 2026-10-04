// SPDX-License-Identifier: MIT
// 画面の外のテクスチャに描くときの配置と色の変換 (マスクの内容のアトラスと、Linear の色空間での中間のテクスチャ)。
// Unity に依存しない (Runtime/Unity/UgoeMaskRenderer.cs・UgoeIntermediate.cs が使う)。
using System;
using System.Collections.Generic;

namespace Ugoe
{
    /// <summary>キャンバス座標の矩形 (Y 下向き)。</summary>
    public readonly struct CanvasRect
    {
        public readonly float XMin, YMin, XMax, YMax;

        public CanvasRect(float xMin, float yMin, float xMax, float yMax)
        {
            XMin = xMin;
            YMin = yMin;
            XMax = xMax;
            YMax = yMax;
        }

        public float Width => XMax - XMin;
        public float Height => YMax - YMin;
        public bool IsEmpty => !(XMin <= XMax && YMin <= YMax);

        /// <summary>頂点位置 (x, y の組) の外接矩形を広げる。</summary>
        public static CanvasRect Union(CanvasRect r, float[] positions)
        {
            float x0 = r.XMin, y0 = r.YMin, x1 = r.XMax, y1 = r.YMax;
            for (var i = 0; i + 1 < positions.Length; i += 2)
            {
                x0 = Math.Min(x0, positions[i]);
                x1 = Math.Max(x1, positions[i]);
                y0 = Math.Min(y0, positions[i + 1]);
                y1 = Math.Max(y1, positions[i + 1]);
            }
            return new CanvasRect(x0, y0, x1, y1);
        }

        /// <summary>何も含まない矩形 (<see cref="Union"/> の始まり)。</summary>
        public static CanvasRect Empty => new CanvasRect(float.PositiveInfinity, float.PositiveInfinity,
            float.NegativeInfinity, float.NegativeInfinity);
    }

    /// <summary>
    /// キャンバスの矩形を写すテクスチャの中の領域。テクスチャの画素 (px, py) は、キャンバスの
    /// (<see cref="X0"/> + (px − <see cref="Px"/>) / <see cref="Scale"/>, <see cref="Y0"/> + (py − <see cref="Py"/>) / <see cref="Scale"/>)
    /// に当たる (py はテクスチャの v の向き。v = 0 がキャンバスの上)。
    /// </summary>
    public readonly struct CanvasRegion
    {
        /// <summary>領域の左下の画素 (テクスチャの中。ビューポートの原点)。</summary>
        public readonly int Px, Py;
        /// <summary>領域の大きさ (画素)。</summary>
        public readonly int Width, Height;
        /// <summary>領域の画素 (Px, Py) の角に当たるキャンバス座標。</summary>
        public readonly float X0, Y0;
        /// <summary>キャンバスの 1 ピクセルあたりの画素数。</summary>
        public readonly float Scale;

        public CanvasRegion(int px, int py, int width, int height, float x0, float y0, float scale)
        {
            Px = px;
            Py = py;
            Width = width;
            Height = height;
            X0 = x0;
            Y0 = y0;
            Scale = scale;
        }

        /// <summary>領域が写すキャンバスの幅と高さ。</summary>
        public float CanvasWidth => Width / Scale;
        public float CanvasHeight => Height / Scale;

        /// <summary>
        /// キャンバス座標から、大きさ <paramref name="textureWidth"/> x <paramref name="textureHeight"/> のテクスチャの UV への写像
        /// <c>uv = canvas × (x, y) + (z, w)</c> の係数 (シェーダの <c>_MaskRect</c> と頂点属性)。
        /// </summary>
        public (float X, float Y, float Z, float W) UvMap(int textureWidth, int textureHeight) =>
            (Scale / textureWidth, Scale / textureHeight,
                (Px - X0 * Scale) / textureWidth, (Py - Y0 * Scale) / textureHeight);

        /// <summary>
        /// 大きさの上限 <paramref name="maxSize"/> の中で、<paramref name="bounds"/> を写す領域を (0, 0) に作る。
        /// 周りに 1 画素の余白を足し、原点を画素の格子 (1 / <paramref name="scale"/> の倍数) にそろえる
        /// (倍率 1 なら、テクスチャの画素がキャンバスのピクセルと重なる)。収まらなければ倍率を下げる。
        /// </summary>
        public static CanvasRegion Fit(CanvasRect bounds, float scale, int maxSize)
        {
            if (bounds.IsEmpty) bounds = new CanvasRect(0, 0, 1, 1);
            var s = scale;
            for (var attempt = 0; attempt < 64; attempt++)
            {
                var px0 = (int)Math.Floor(bounds.XMin * s) - 1;
                var py0 = (int)Math.Floor(bounds.YMin * s) - 1;
                var px1 = (int)Math.Ceiling(bounds.XMax * s) + 1;
                var py1 = (int)Math.Ceiling(bounds.YMax * s) + 1;
                var w = Math.Max(px1 - px0, 1);
                var h = Math.Max(py1 - py0, 1);
                if (w <= maxSize && h <= maxSize) return new CanvasRegion(0, 0, w, h, px0 / s, py0 / s, s);
                s *= Math.Min((float)maxSize / w, (float)maxSize / h) * 0.999f;
            }
            return new CanvasRegion(0, 0, 1, 1, bounds.XMin, bounds.YMin, s);
        }
    }

    /// <summary>
    /// マスクの内容 (<see cref="MaskContent"/>) を、入れ子の深さごとに 1 枚のテクスチャ (ページ) に並べる配置。
    /// </summary>
    /// <remarks>
    /// 同じページの内容を読む要素は、読む内容が違っても 1 回の描画にまとめられる (<see cref="DrawBatchPlan"/>)。
    /// 入れ子のマスク (マスク用メッシュが自身のマスクを読む) は浅いページを読みながら深いページに描くので、
    /// 書き込み先のテクスチャを同時に読むことはない。
    /// </remarks>
    public sealed class MaskAtlasLayout
    {
        /// <summary>ページの 1 辺の上限 (画素)。</summary>
        public const int MaxSize = 4096;
        /// <summary>内容と内容の間の空き (画素)。バイリニアのサンプリングが隣の内容を読まないように。</summary>
        public const int Gap = 2;

        /// <summary>内容ごとの領域 (<see cref="MaskPlan.Contents"/> と同じ並び)。</summary>
        public CanvasRegion[] Regions { get; }
        /// <summary>内容ごとのページの番号 (= 入れ子の深さ)。</summary>
        public int[] PageOf { get; }
        /// <summary>ページごとの、使う大きさ (画素)。</summary>
        public (int Width, int Height)[] PageSizes { get; }

        private MaskAtlasLayout(CanvasRegion[] regions, int[] pageOf, (int, int)[] sizes)
        {
            Regions = regions;
            PageOf = pageOf;
            PageSizes = sizes;
        }

        /// <summary>
        /// 内容ごとの外接矩形 <paramref name="bounds"/> (キャンバス座標) を、深さ <paramref name="depths"/> のページに詰める。
        /// ページに収まらなければ、そのページの倍率を下げて詰め直す。
        /// </summary>
        /// <param name="scale">キャンバスの 1 ピクセルあたりの画素数。</param>
        public static MaskAtlasLayout Build(IReadOnlyList<CanvasRect> bounds, IReadOnlyList<int> depths, float scale,
            int maxSize = MaxSize)
        {
            var count = bounds.Count;
            var regions = new CanvasRegion[count];
            var pageOf = new int[count];
            var pages = 0;
            for (var i = 0; i < count; i++)
            {
                pageOf[i] = depths[i];
                pages = Math.Max(pages, depths[i] + 1);
            }
            var sizes = new (int, int)[pages];
            for (var page = 0; page < pages; page++)
            {
                var members = new List<int>();
                for (var i = 0; i < count; i++)
                {
                    if (pageOf[i] == page) members.Add(i);
                }
                var s = scale;
                for (var attempt = 0; attempt < 64; attempt++)
                {
                    if (TryPack(members, bounds, s, maxSize, regions, out var size))
                    {
                        sizes[page] = size;
                        break;
                    }
                    s *= 0.8f;
                }
            }
            return new MaskAtlasLayout(regions, pageOf, sizes);
        }

        // 高い順に棚 (shelf) に並べる。幅は最も幅の広い内容と、面積から決める目安の大きい方。
        private static bool TryPack(List<int> members, IReadOnlyList<CanvasRect> bounds, float scale, int maxSize,
            CanvasRegion[] regions, out (int, int) size)
        {
            size = (1, 1);
            var fitted = new CanvasRegion[members.Count];
            long area = 0;
            var widest = 1;
            for (var k = 0; k < members.Count; k++)
            {
                fitted[k] = CanvasRegion.Fit(bounds[members[k]], scale, maxSize);
                if (fitted[k].Scale < scale) return false;
                area += (long)(fitted[k].Width + Gap) * (fitted[k].Height + Gap);
                widest = Math.Max(widest, fitted[k].Width);
            }
            var rowWidth = Math.Min(maxSize, Math.Max(widest, (int)Math.Ceiling(Math.Sqrt(area))));
            var order = new List<int>();
            for (var k = 0; k < members.Count; k++) order.Add(k);
            order.Sort((a, b) => fitted[a].Height != fitted[b].Height ? fitted[b].Height.CompareTo(fitted[a].Height) : a.CompareTo(b));
            int x = 0, y = 0, shelf = 0, width = 1;
            foreach (var k in order)
            {
                var f = fitted[k];
                if (x > 0 && x + f.Width > rowWidth)
                {
                    y += shelf + Gap;
                    x = 0;
                    shelf = 0;
                }
                if (y + f.Height > maxSize) return false;
                regions[members[k]] = new CanvasRegion(x, y, f.Width, f.Height, f.X0, f.Y0, f.Scale);
                width = Math.Max(width, x + f.Width);
                x += f.Width + Gap;
                shelf = Math.Max(shelf, f.Height);
            }
            size = (width, Math.Max(y + shelf, 1));
            return true;
        }
    }

    /// <summary>
    /// sRGB の値と線形の値の変換 (IEC 61966-2-1 の式)。シェーダ (UgoeCommon.hlsl の <c>UgoeSrgbToLinear</c>) と同じ式で、
    /// GPU が sRGB の書き込み先に書くときの変換 (線形 → sRGB) の逆になる。
    /// </summary>
    public static class ColorSpaces
    {
        public static float SrgbToLinear(float c) =>
            c <= 0.04045f ? c / 12.92f : (float)Math.Pow((c + 0.055f) / 1.055f, 2.4f);

        public static float LinearToSrgb(float c) =>
            c <= 0.0031308f ? c * 12.92f : 1.055f * (float)Math.Pow(c, 1 / 2.4f) - 0.055f;

        /// <summary>0〜1 の値を 8 ビットに丸める (書き込み先の量子化)。</summary>
        public static byte ToByte(float c) => (byte)Math.Round(Clamp01(c) * 255f);

        /// <summary>
        /// 中間のテクスチャの画素 (乗算済みアルファ、sRGB の値) を、Linear の色空間の書き込み先に合成する値
        /// (乗算済みアルファ、線形の値) にする。合成のシェーダ (UgoeComposite.shader) と同じ式。
        /// </summary>
        public static (float R, float G, float B, float A) CompositeToLinear(float r, float g, float b, float a)
        {
            if (a <= 0) return (SrgbToLinear(r), SrgbToLinear(g), SrgbToLinear(b), a);
            return (SrgbToLinear(Clamp01(r / a)) * a, SrgbToLinear(Clamp01(g / a)) * a, SrgbToLinear(Clamp01(b / a)) * a, a);
        }

        // Math.Clamp は .NET Standard 2.1 からで、Unity の API 互換レベル「.NET Framework」には無いので使わない。
        private static float Clamp01(float c) => c < 0 ? 0 : c > 1 ? 1 : c;
    }
}
