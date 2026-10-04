// SPDX-License-Identifier: MIT
// 画面の外のテクスチャの配置と色の変換 (Runtime/Core/Offscreen.cs)。
using System;
using System.Linq;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class OffscreenTests
    {
        [Test]
        public void RegionsAlignWithThePixelGrid()
        {
            var r = CanvasRegion.Fit(new CanvasRect(10.3f, -4.5f, 20.2f, 7f), 1, 4096);
            // 1 画素の余白を足し、原点は整数のキャンバス座標 (倍率 1 ならキャンバスのピクセルと重なる)。
            Assert.That((r.X0, r.Y0, r.Width, r.Height), Is.EqualTo((9f, -6f, 13, 14)));
            var half = CanvasRegion.Fit(new CanvasRect(10.3f, -4.5f, 20.2f, 7f), 0.5f, 4096);
            Assert.That((half.X0, half.Y0), Is.EqualTo((8f, -8f)));
            Assert.That(half.X0 * half.Scale, Is.EqualTo(Math.Floor(half.X0 * half.Scale)));

            // 収まらなければ倍率を下げる。
            var big = CanvasRegion.Fit(new CanvasRect(0, 0, 10000, 100), 1, 4096);
            Assert.That(big.Width, Is.LessThanOrEqualTo(4096));
            Assert.That(big.Scale, Is.LessThan(0.41f));
            Assert.That(big.X0 + big.CanvasWidth, Is.GreaterThanOrEqualTo(10000));
        }

        [Test]
        public void UvMapPutsTheCanvasRectInsideItsRegion()
        {
            var region = new CanvasRegion(100, 40, 30, 20, 5, 7, 2);
            var (x, y, z, w) = region.UvMap(256, 128);
            // キャンバスの (X0, Y0) は領域の角 (Px, Py)、そこから 1 ピクセルで Scale 画素進む。
            Assert.That(5 * x + z, Is.EqualTo(100f / 256).Within(1e-6));
            Assert.That(7 * y + w, Is.EqualTo(40f / 128).Within(1e-6));
            Assert.That((5 + 15) * x + z, Is.EqualTo(130f / 256).Within(1e-6));
            Assert.That((7 + 10) * y + w, Is.EqualTo(60f / 128).Within(1e-6));
        }

        [Test]
        public void MaskAtlasPacksContentsWithoutOverlap()
        {
            var bounds = new[]
            {
                new CanvasRect(0, 0, 100, 50), new CanvasRect(-20, 30, 60, 300), new CanvasRect(500, 500, 510, 505),
                new CanvasRect(0, 0, 40, 40), new CanvasRect(10, 10, 30, 30),
            };
            var depths = new[] { 0, 0, 0, 1, 1 };
            var layout = MaskAtlasLayout.Build(bounds, depths, 1);
            Assert.That(layout.PageSizes.Length, Is.EqualTo(2));
            Assert.That(layout.PageOf, Is.EqualTo(depths));
            for (var a = 0; a < bounds.Length; a++)
            {
                var ra = layout.Regions[a];
                var (w, h) = layout.PageSizes[layout.PageOf[a]];
                Assert.That(ra.Px >= 0 && ra.Py >= 0 && ra.Px + ra.Width <= w && ra.Py + ra.Height <= h, Is.True, $"content {a}");
                // 内容の外接矩形は、領域の内側 (余白を残して) に写る。
                Assert.That(ra.X0, Is.LessThan(bounds[a].XMin));
                Assert.That(ra.Y0, Is.LessThan(bounds[a].YMin));
                Assert.That(ra.X0 + ra.CanvasWidth, Is.GreaterThan(bounds[a].XMax));
                Assert.That(ra.Y0 + ra.CanvasHeight, Is.GreaterThan(bounds[a].YMax));
                for (var b = a + 1; b < bounds.Length; b++)
                {
                    if (layout.PageOf[a] != layout.PageOf[b]) continue;
                    var rb = layout.Regions[b];
                    var apart = ra.Px + ra.Width + MaskAtlasLayout.Gap <= rb.Px || rb.Px + rb.Width + MaskAtlasLayout.Gap <= ra.Px
                        || ra.Py + ra.Height + MaskAtlasLayout.Gap <= rb.Py || rb.Py + rb.Height + MaskAtlasLayout.Gap <= ra.Py;
                    Assert.That(apart, Is.True, $"contents {a} and {b} overlap");
                }
            }
        }

        [Test]
        public void MaskAtlasLowersTheScaleWhenAPageIsFull()
        {
            var bounds = Enumerable.Range(0, 6).Select(_ => new CanvasRect(0, 0, 3000, 3000)).ToArray();
            var layout = MaskAtlasLayout.Build(bounds, new int[6], 1, 4096);
            Assert.That(layout.Regions.All(r => r.Scale < 1), Is.True);
            Assert.That(layout.PageSizes[0].Width, Is.LessThanOrEqualTo(4096));
            Assert.That(layout.PageSizes[0].Height, Is.LessThanOrEqualTo(4096));
        }

        // 並べ直した配置は、同じ引数で作った配置と同じになる (前の内容の大きさ・ページの数に左右されない)。
        [Test]
        public void UpdatedMaskAtlasMatchesAFreshBuild()
        {
            var frames = new[]
            {
                (new[]
                {
                    new CanvasRect(0, 0, 100, 50), new CanvasRect(-20, 30, 60, 300), new CanvasRect(500, 500, 510, 505),
                    new CanvasRect(0, 0, 40, 40), new CanvasRect(10, 10, 30, 30),
                }, new[] { 0, 0, 0, 1, 1 }, 1f),
                // 高さの順が入れ替わる。
                (new[]
                {
                    new CanvasRect(0, 0, 100, 400), new CanvasRect(-20, 30, 60, 40), new CanvasRect(500, 500, 700, 505),
                    new CanvasRect(0, 0, 4, 90), new CanvasRect(10, 10, 300, 30),
                }, new[] { 0, 0, 0, 1, 1 }, 1f),
                // ページに収まらず倍率を下げる。
                (Enumerable.Range(0, 5).Select(_ => new CanvasRect(0, 0, 3000, 3000)).ToArray(), new[] { 0, 0, 0, 0, 1 }, 1f),
                // ページの数と内容の数が変わる。
                (new[] { new CanvasRect(0, 0, 10, 10), new CanvasRect(5, 5, 25, 15) }, new[] { 0, 2 }, 2f),
                (new[] { new CanvasRect(0, 0, 10, 10), new CanvasRect(5, 5, 25, 15), new CanvasRect(-5, 0, 0, 5) }, new[] { 1, 0, 0 }, 0.5f),
            };
            var layout = MaskAtlasLayout.Build(frames[0].Item1, frames[0].Item2, frames[0].Item3, 4096);
            foreach (var (bounds, depths, scale) in frames)
            {
                layout.Update(bounds, depths, scale, 4096);
                var fresh = MaskAtlasLayout.Build(bounds, depths, scale, 4096);
                Assert.That(layout.Regions, Is.EqualTo(fresh.Regions));
                Assert.That(layout.PageOf, Is.EqualTo(fresh.PageOf));
                Assert.That(layout.PageSizes, Is.EqualTo(fresh.PageSizes));
            }
        }

        // 内容の数と深さが変わらない間、並べ直しは配列を作り直さず、何も確保しない (UgoeMaskRenderer.Render が毎フレーム呼ぶ)。
        [Test]
        public void UpdatingMaskAtlasDoesNotAllocate()
        {
            var bounds = new[]
            {
                new CanvasRect(0, 0, 100, 50), new CanvasRect(-20, 30, 60, 300), new CanvasRect(500, 500, 510, 505),
                new CanvasRect(0, 0, 40, 40), new CanvasRect(10, 10, 30, 30),
            };
            var depths = new[] { 0, 0, 0, 1, 1 };
            var layout = MaskAtlasLayout.Build(bounds, depths, 1);
            var (regions, pageOf, sizes) = (layout.Regions, layout.PageOf, layout.PageSizes);
            layout.Update(bounds, depths, 1);
            var before = GC.GetAllocatedBytesForCurrentThread();
            for (var frame = 0; frame < 100; frame++)
            {
                bounds[1] = new CanvasRect(-20 - frame, 30, 60, 300 + frame);
                layout.Update(bounds, depths, 1);
            }
            Assert.That(GC.GetAllocatedBytesForCurrentThread() - before, Is.Zero);
            Assert.That(layout.Regions, Is.SameAs(regions));
            Assert.That(layout.PageOf, Is.SameAs(pageOf));
            Assert.That(layout.PageSizes, Is.SameAs(sizes));
        }

        // Linear の色空間の書き込み先は、シェーダの出力 (線形) を sRGB に直して 8 ビットに丸める。合成のシェーダは
        // 中間のテクスチャの値を線形に直して書くので、不透明な画素はすべての値で元に戻る。
        [Test]
        public void OpaqueIntermediatePixelsSurviveTheLinearTarget()
        {
            for (var v = 0; v < 256; v++)
            {
                var c = v / 255f;
                var (r, _, _, a) = ColorSpaces.CompositeToLinear(c, c, c, 1);
                Assert.That(a, Is.EqualTo(1));
                Assert.That(ColorSpaces.ToByte(ColorSpaces.LinearToSrgb(r)), Is.EqualTo(v), $"value {v}");
            }
        }

        // 参照実装の合成 (sRGB の値のまま、乗算済みアルファ) と、Linear の色空間で画面に直接描く合成を比べる。
        // 中間のテクスチャに描けば参照と同じ値になり、直接描くと半透明の重なりでずれる (README の「色空間」)。
        [Test]
        public void IntermediateMatchesTheReferenceWhereDirectDrawingDoesNot()
        {
            // 不透明な赤の上に、半透明 (0.5) の青と、半透明 (0.25) の白を重ねる (乗算済みアルファ)。
            var layers = new[] { (1f, 0f, 0f, 1f), (0f, 0f, 0.5f, 0.5f), (0.25f, 0.25f, 0.25f, 0.25f) };

            // 参照実装と、中間のテクスチャ (sRGB の変換をしない): sRGB の値で src + dst × (1 − src.a)。
            var reference = (0f, 0f, 0f, 0f);
            foreach (var l in layers) reference = Over(l, reference);
            var intermediate = reference;

            // 中間のテクスチャを Linear の書き込み先 (不透明な黒の背景) に写して、8 ビットに丸めた値。
            var shown = Over(ColorSpaces.CompositeToLinear(intermediate.Item1, intermediate.Item2, intermediate.Item3, intermediate.Item4),
                (0f, 0f, 0f, 1f));
            // 直接描く: 各要素を線形に直し、線形の値で合成する。
            var direct = (0f, 0f, 0f, 1f);
            foreach (var l in layers) direct = Over(ColorSpaces.CompositeToLinear(l.Item1, l.Item2, l.Item3, l.Item4), direct);

            var expected = Bytes(reference);
            Assert.That(Bytes(Srgb(shown)), Is.EqualTo(expected));
            var drift = Bytes(Srgb(direct)).Zip(expected, (x, y) => Math.Abs(x - y)).Max();
            Assert.That(drift, Is.GreaterThan(10), "direct drawing should drift (otherwise this test proves nothing)");
        }

        private static (float, float, float, float) Over((float, float, float, float) s, (float, float, float, float) d) =>
            (s.Item1 + d.Item1 * (1 - s.Item4), s.Item2 + d.Item2 * (1 - s.Item4), s.Item3 + d.Item3 * (1 - s.Item4),
                s.Item4 + d.Item4 * (1 - s.Item4));

        private static (float, float, float, float) Srgb((float, float, float, float) c) =>
            (ColorSpaces.LinearToSrgb(c.Item1), ColorSpaces.LinearToSrgb(c.Item2), ColorSpaces.LinearToSrgb(c.Item3), c.Item4);

        private static int[] Bytes((float, float, float, float) c) =>
            new[] { c.Item1, c.Item2, c.Item3 }.Select(v => (int)ColorSpaces.ToByte(v)).ToArray();
    }
}
