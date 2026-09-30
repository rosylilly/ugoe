// SPDX-License-Identifier: MIT
using System.Linq;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class RenderHelperTests
    {
        [Test]
        public void PremultiplyMatchesTheReference()
        {
            // (c × a + 127) div 255
            Assert.That(TextureConversion.Premultiply(255, 128), Is.EqualTo(128));
            Assert.That(TextureConversion.Premultiply(200, 0), Is.EqualTo(0));
            Assert.That(TextureConversion.Premultiply(1, 127), Is.EqualTo(0));
            Assert.That(TextureConversion.Premultiply(1, 128), Is.EqualTo(1));
            var rgba = new byte[] { 255, 100, 10, 255, 255, 100, 10, 128 };
            TextureConversion.PremultiplyRgba32(rgba);
            Assert.That(rgba, Is.EqualTo(new byte[] { 255, 100, 10, 255, 128, 50, 5, 128 }));
        }

        [Test]
        public void CullingUsesCanvasCoordinates()
        {
            // キャンバス座標 (Y 下向き) で反時計回りに見える三角形が表。
            var positions = new float[] { 0, 0, 0, 10, 10, 0 };
            var output = new int[6];
            Assert.That(Culling.FrontFacing(positions, new ushort[] { 0, 1, 2 }, output), Is.EqualTo(3));
            Assert.That(output.Take(3), Is.EqualTo(new[] { 0, 1, 2 }));
            Assert.That(Culling.FrontFacing(positions, new ushort[] { 0, 2, 1 }, output), Is.EqualTo(0));
            // 潰れた三角形 (値が 0) は裏として扱う。
            Assert.That(Culling.FrontFacing(new float[] { 0, 0, 1, 1, 2, 2 }, new ushort[] { 0, 1, 2 }, output), Is.EqualTo(0));
        }

        private static DrawableInfo Mesh(string id, MaskMode mode = MaskMode.Union, params MaskRef[] masks) =>
            new DrawableInfo(id, 0, 3, 3, false, false, mode, masks, false);

        [Test]
        public void MaskPlanSharesContentsAndOrdersNesting()
        {
            var drawables = new[]
            {
                Mesh("clip"),                                                   // 0
                Mesh("inner", MaskMode.Union, new MaskRef(0, false)),           // 1: 0 で切られ、2 のマスクになる
                Mesh("a", MaskMode.Union, new MaskRef(1, false)),               // 2: 1 を読む (入れ子の深さ 1)
                Mesh("b", MaskMode.Union, new MaskRef(1, false), new MaskRef(1, false)), // 3: 2 と同じ内容 (重複は 1 つ)
                Mesh("c", MaskMode.Intersection, new MaskRef(0, false), new MaskRef(1, false), new MaskRef(6, true)), // 4
                Mesh("d", MaskMode.Intersection, new MaskRef(0, false)),        // 5: 通常マスク 1 つなら和集合と同じ → 1 と共有
                Mesh("free"),                                                   // 6
            };
            var plan = MaskPlan.Build(drawables);
            Assert.That(plan.Contents.Count, Is.EqualTo(3));
            Assert.That(plan.ContentOf[6], Is.EqualTo(-1));
            Assert.That(plan.ContentOf[2], Is.EqualTo(plan.ContentOf[3]));
            Assert.That(plan.ContentOf[1], Is.EqualTo(plan.ContentOf[5]));

            var outer = plan.Contents[plan.ContentOf[1]];
            var nested = plan.Contents[plan.ContentOf[2]];
            var both = plan.Contents[plan.ContentOf[4]];
            Assert.That(outer.Depth, Is.EqualTo(0));
            Assert.That(nested.Depth, Is.EqualTo(1));
            Assert.That(both.Depth, Is.EqualTo(1));
            // 深さの小さい順に並ぶ
            Assert.That(plan.Contents.Select(c => c.Depth), Is.Ordered);
            Assert.That(outer.Readers, Is.EqualTo(new[] { 1, 5 }));

            Assert.That(both.Positive, Is.EqualTo(new[] { 0, 1 }));
            Assert.That(both.Negative, Is.EqualTo(new[] { 6 }));
            Assert.That(both.Mode, Is.EqualTo(MaskMode.Intersection));
            Assert.That((both.PositiveOffset, both.PositiveScale), Is.EqualTo((0f, 1f)));
            Assert.That((outer.PositiveOffset, outer.PositiveScale), Is.EqualTo((1f, -1f)));

            Assert.That(plan.IsMaskSource[0], Is.True);
            Assert.That(plan.IsMaskSource[1], Is.True);
            Assert.That(plan.IsMaskSource[2], Is.False);
        }

        // マスクの内容を描く手順 (UgoeMaskRenderer) を CPU でなぞり、spec/rendering.md の式と比べる。
        private static (float r, float g) Channels(MaskContent content, float[] coverage)
        {
            float r = 1, g = 1;
            foreach (var i in content.Positive)
            {
                // 和集合: R ×= 1 − c。共通部分: 作業用に 1 − c を作り、R ×= 1 − (1 − c)。
                r *= content.Mode == MaskMode.Intersection && content.Positive.Length > 1 ? 1 - (1 - coverage[i]) : 1 - coverage[i];
            }
            foreach (var i in content.Negative) g *= 1 - coverage[i];
            return (r, g);
        }

        private static float SpecFactor(MaskMode mode, int[] pos, int[] neg, float[] c, float s)
        {
            float p;
            if (pos.Length == 0) p = 1;
            else if (mode == MaskMode.Union) p = 1 - pos.Aggregate(1f, (acc, i) => acc * (1 - c[i]));
            else p = pos.Aggregate(1f, (acc, i) => acc * c[i]);
            var n = neg.Length == 0 ? 0 : 1 - neg.Aggregate(1f, (acc, i) => acc * (1 - c[i]));
            var m = p * (1 - n);
            return 1 - s * (1 - m);
        }

        [Test]
        public void MaskChannelsReproduceTheSpecFormula()
        {
            var coverage = new[] { 0.25f, 0.8f, 0.5f, 0.1f };
            var cases = new[]
            {
                (MaskMode.Union, new[] { new MaskRef(0, false), new MaskRef(1, false) }),
                (MaskMode.Intersection, new[] { new MaskRef(0, false), new MaskRef(1, false) }),
                (MaskMode.Intersection, new[] { new MaskRef(0, false), new MaskRef(1, false), new MaskRef(2, true), new MaskRef(3, true) }),
                (MaskMode.Union, new[] { new MaskRef(2, true) }),
                (MaskMode.Intersection, new[] { new MaskRef(1, false) }),
            };
            foreach (var (mode, masks) in cases)
            {
                var drawables = new[] { Mesh("0"), Mesh("1"), Mesh("2"), Mesh("3"), Mesh("x", mode, masks) };
                var content = MaskPlan.Build(drawables).Contents[0];
                var (r, g) = Channels(content, coverage);
                var pos = masks.Where(m => !m.Invert).Select(m => m.Drawable).ToArray();
                var neg = masks.Where(m => m.Invert).Select(m => m.Drawable).ToArray();
                foreach (var s in new[] { 0f, 0.5f, 1f })
                {
                    Assert.That(content.Factor(r, g, s), Is.EqualTo(SpecFactor(mode, pos, neg, coverage, s)).Within(1e-6),
                        $"{mode} {masks.Length} masks, strength {s}");
                }
            }
        }

        [Test]
        public void MaskPlansOfTheSamples()
        {
            foreach (var path in new[] { new[] { "avatar.ugoe" }, new[] { "avatar-lit.ugoe" }, new[] { "dress-up", "dress-up.ugoe" } })
            {
                using var puppet = Puppet.FromBundle(Paths.Read(path));
                var plan = MaskPlan.Build(puppet.Drawables);
                for (var i = 0; i < puppet.Drawables.Count; i++)
                {
                    Assert.That(plan.ContentOf[i] >= 0, Is.EqualTo(puppet.Drawables[i].Masks.Length > 0));
                }
            }
        }

        [Test]
        public void MaskPlanWithOnlyInvertedMasks()
        {
            var plan = MaskPlan.Build(new[] { Mesh("m"), Mesh("x", MaskMode.Union, new MaskRef(0, true)) });
            var c = plan.Contents[0];
            Assert.That(c.Positive, Is.Empty);
            Assert.That((c.PositiveOffset, c.PositiveScale), Is.EqualTo((1f, 0f)));
        }
    }
}
