// SPDX-License-Identifier: MIT
// 描画のバッチング (Runtime/Core/DrawBatches.cs)。
using System;
using System.IO;
using System.Linq;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class BatchTests
    {
        private static DrawableInfo Mesh(string id, int texture = 0, int vertices = 3, bool maskOnly = false, bool lit = false,
            bool culling = false, params MaskRef[] masks) =>
            new DrawableInfo(id, texture, vertices, 3, culling, maskOnly, MaskMode.Union, masks, lit);

        private static DrawableState State(BlendMode blend = BlendMode.Normal, bool visible = true) =>
            new DrawableState(new Native.DrawableState { Opacity = 1, BlendMode = (uint)blend, Visible = visible ? (byte)1 : (byte)0 });

        private static int[][] Members(DrawBatchPlan plan) => plan.Batches.Select(b => b.Members).ToArray();

        [Test]
        public void JoinsNeighboursWithTheSameMaterial()
        {
            var drawables = new[]
            {
                Mesh("a"), Mesh("b"), Mesh("other-texture", texture: 1), Mesh("c"), Mesh("lit", lit: true),
                Mesh("mask", maskOnly: true), Mesh("add"), Mesh("d"),
            };
            var states = drawables.Select((_, i) => State(i == 6 ? BlendMode.Additive : BlendMode.Normal)).ToArray();
            var order = Enumerable.Range(0, drawables.Length).ToArray();
            var masks = MaskPlan.Build(drawables);
            var plan = DrawBatchPlan.Build(drawables, states, order, masks, batching: true);
            // ライティングを受けるかは頂点属性で渡すので、状態の違いにしない。
            Assert.That(Members(plan), Is.EqualTo(new[] { new[] { 0, 1 }, new[] { 2 }, new[] { 3, 4 }, new[] { 6 }, new[] { 7 } }));
            Assert.That(plan.BatchOf, Is.EqualTo(new[] { 0, 0, 1, 2, 2, -1, 3, 4 }));
            Assert.That(plan.Batches[0].VertexOffsets, Is.EqualTo(new[] { 0, 3 }));
            Assert.That(plan.Batches[3].BlendMode, Is.EqualTo(BlendMode.Additive));

            // 描画順が変われば並びも変わる (マスク専用の要素は間に入っても切らない)。
            var reordered = new[] { 5, 0, 3, 1, 2, 4, 6, 7 };
            Assert.That(Members(DrawBatchPlan.Build(drawables, states, reordered, masks, true)),
                Is.EqualTo(new[] { new[] { 0, 3, 1 }, new[] { 2 }, new[] { 4 }, new[] { 6 }, new[] { 7 } }));

            // 切れば要素ごと。
            Assert.That(DrawBatchPlan.Build(drawables, states, order, masks, batching: false).Batches.Count, Is.EqualTo(7));

            Assert.That(plan.NeedsRebuild(states), Is.False);
            states[1] = State(BlendMode.Multiply);
            Assert.That(plan.NeedsRebuild(states), Is.True);
        }

        [Test]
        public void MaskReadersJoinTheSameContentOnly()
        {
            var drawables = new[]
            {
                Mesh("source", maskOnly: true), Mesh("a"),
                Mesh("reader1", masks: new MaskRef(0, false)), Mesh("reader2", masks: new MaskRef(0, false)), Mesh("b"),
                Mesh("inverted", masks: new MaskRef(0, true)), Mesh("c"), Mesh("reader3", masks: new MaskRef(0, false)),
            };
            var states = drawables.Select(_ => State()).ToArray();
            var masks = MaskPlan.Build(drawables);
            var plan = DrawBatchPlan.Build(drawables, states, Enumerable.Range(0, drawables.Length).ToArray(), masks, true);
            // マスクを読まない要素はどの内容ともまとめる。違う内容 (反転マスク) を読む要素で切る。
            Assert.That(Members(plan), Is.EqualTo(new[] { new[] { 1, 2, 3, 4 }, new[] { 5, 6 }, new[] { 7 } }));
            Assert.That(plan.Batches.Select(b => b.MaskContent), Is.EqualTo(new[] { masks.ContentOf[2], masks.ContentOf[5], masks.ContentOf[7] }));
            Assert.That(masks.ContentOf[2], Is.Not.EqualTo(masks.ContentOf[5]));
        }

        [Test]
        public void SplitsAtTheVertexLimit()
        {
            var drawables = Enumerable.Range(0, 5).Select(i => Mesh($"m{i}", vertices: 30000)).ToArray();
            var states = drawables.Select(_ => State()).ToArray();
            var plan = DrawBatchPlan.Build(drawables, states, Enumerable.Range(0, 5).ToArray(), MaskPlan.Build(drawables), true);
            Assert.That(Members(plan), Is.EqualTo(new[] { new[] { 0, 1 }, new[] { 2, 3 }, new[] { 4 } }));
            Assert.That(plan.Batches.All(b => b.VertexCount <= DrawBatchPlan.MaxVertices), Is.True);
        }

        [Test]
        public void IndicesSkipHiddenAndBackFacingTriangles()
        {
            var drawables = new[] { Mesh("a"), Mesh("hidden"), Mesh("culled", culling: true) };
            var states = new[] { State(), State(visible: false), State() };
            var plan = DrawBatchPlan.Build(drawables, states, new[] { 0, 1, 2 }, MaskPlan.Build(drawables), true);
            var batch = plan.Batches.Single();
            var indices = new[] { new ushort[] { 0, 1, 2 }, new ushort[] { 0, 1, 2 }, new ushort[] { 0, 1, 2 } };
            var front = new float[] { 0, 0, 0, 10, 10, 0 };
            var back = new float[] { 0, 0, 10, 0, 0, 10 };
            var output = new int[batch.IndexCapacity];
            var n = batch.WriteIndices(drawables, indices, new[] { front, front, front }, states, output);
            Assert.That(output.Take(n), Is.EqualTo(new[] { 0, 1, 2, 6, 7, 8 }));
            n = batch.WriteIndices(drawables, indices, new[] { front, front, back }, states, output);
            Assert.That(output.Take(n), Is.EqualTo(new[] { 0, 1, 2 }));
        }

        // 見本のモデルで、描画の呼び出しがどれだけ減るか (README の数と合わせる)。見本はレイヤーごとのテクスチャなので
        // そのままでは減らない。`ugoe build --atlas` で 1 枚のアトラスにまとめたとき (すべての要素のテクスチャが 0 になる) を真似る
        // (法線マップのあるテクスチャはアトラスにまとめないので、avatar-lit.ugoe は除く)。
        [TestCase(2, "avatar.ugoe")]
        [TestCase(2, "dress-up", "dress-up.ugoe")]
        public void SamplesNeedFewerDrawCallsWithAnAtlas(int expected, params string[] path)
        {
            using var puppet = Puppet.FromBundle(Paths.Read(path));
            puppet.Update(0);
            var sync = new FrameSync(puppet);
            sync.Pull();
            var drawables = puppet.Drawables;
            var masks = MaskPlan.Build(drawables);
            var single = DrawBatchPlan.Build(drawables, sync.States, sync.RenderOrder, masks, false);
            var asIs = DrawBatchPlan.Build(drawables, sync.States, sync.RenderOrder, masks, true);
            var atlas = drawables.Select(d => new DrawableInfo(d.Id, 0, d.VertexCount, d.IndexCount, d.Culling, d.MaskOnly,
                d.MaskMode, d.Masks, d.Lit)).ToArray();
            var atlased = DrawBatchPlan.Build(atlas, sync.States, sync.RenderOrder, masks, true);
            TestContext.Out.WriteLine($"{Path.Combine(path)}: {drawables.Count} drawables, {single.Batches.Count} draw calls " +
                $"without batching, {asIs.Batches.Count} with batching, {atlased.Batches.Count} with batching and an atlas");
            Assert.That(asIs.Batches.Count, Is.LessThanOrEqualTo(single.Batches.Count));
            Assert.That(atlased.Batches.Count, Is.EqualTo(expected));
            // まとめても描く要素と順は変わらない。
            var order = single.Batches.SelectMany(b => b.Members).ToArray();
            Assert.That(asIs.Batches.SelectMany(b => b.Members), Is.EqualTo(order));
            Assert.That(atlased.Batches.SelectMany(b => b.Members), Is.EqualTo(order));
        }
    }
}
