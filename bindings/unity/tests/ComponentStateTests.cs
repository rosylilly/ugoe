// SPDX-License-Identifier: MIT
// UgoePuppet の状態の扱い (Runtime/Core/ComponentState.cs)。T0426 で見つけた不具合を再現する入力を含む。
using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class ComponentStateTests
    {
        // UgoeModel・UgoeModule の代わり (参照で比べる)
        private sealed class Asset
        {
            public readonly string Name;
            public readonly string Slot;

            public Asset(string name, string slot = null)
            {
                Name = name;
                Slot = slot ?? name;
            }

            public override string ToString() => Name;
        }

        private static DrawableInfo Mesh(string id, bool culling = false, params MaskRef[] masks) =>
            new DrawableInfo(id, 0, 3, 3, culling, false, MaskMode.Union, masks, false);

        private static DrawableState State(DrawableChanges changes = DrawableChanges.None, BlendMode blend = BlendMode.Normal) =>
            new DrawableState(new Native.DrawableState { Opacity = 1, BlendMode = (uint)blend, Visible = 1, Changes = (byte)changes });

        // --- pixelsPerUnit ---

        [Test]
        public void PixelsPerUnitStaysPositiveAndFinite()
        {
            Assert.That(PixelScale.Sanitize(50, 100), Is.EqualTo(50));
            Assert.That(PixelScale.Sanitize(0, 100), Is.EqualTo(PixelScale.Min));
            Assert.That(PixelScale.Sanitize(-3, 100), Is.EqualTo(PixelScale.Min));
            Assert.That(PixelScale.Sanitize(float.NaN, 80), Is.EqualTo(80));
            Assert.That(PixelScale.Sanitize(float.PositiveInfinity, 80), Is.EqualTo(80));
            Assert.That(PixelScale.Sanitize(float.NegativeInfinity, 80), Is.EqualTo(80));
            Assert.That(PixelScale.Sanitize(float.NaN, float.NaN), Is.EqualTo(PixelScale.Default));
            Assert.That(PixelScale.Sanitize(float.NaN, 0), Is.EqualTo(PixelScale.Default));
        }

        // --- 読み直すか ---

        [Test]
        public void ReloadsBeforeTheFirstLoad()
        {
            var loaded = new LoadedSource<Asset, Asset>();
            Assert.That(loaded.NeedsReload(null, new Asset[0], 100), Is.True);
        }

        [Test]
        public void UnchangedSourceDoesNotReload()
        {
            var model = new Asset("model");
            var hair = new Asset("hair");
            var loaded = new LoadedSource<Asset, Asset>();
            loaded.Remember(model, new[] { hair }, 100);
            Assert.That(loaded.NeedsReload(model, new List<Asset> { hair }, 100), Is.False);
        }

        // T0426: インスペクタで pixelsPerUnit を変えても読み直さず、頂点の動いたメッシュだけが新しい大きさになった。
        [Test]
        public void InspectorPixelsPerUnitChangeReloads()
        {
            var model = new Asset("model");
            var loaded = new LoadedSource<Asset, Asset>();
            loaded.Remember(model, new Asset[0], 100);
            Assert.That(loaded.NeedsReload(model, new Asset[0], 50), Is.True);
            loaded.Remember(model, new Asset[0], 50);
            Assert.That(loaded.NeedsReload(model, new Asset[0], 50), Is.False);
        }

        [Test]
        public void ModelOrModuleChangesReload()
        {
            var model = new Asset("model");
            var hair = new Asset("hair");
            var ears = new Asset("ears");
            var loaded = new LoadedSource<Asset, Asset>();
            loaded.Remember(model, new[] { hair, ears }, 100);
            Assert.That(loaded.NeedsReload(new Asset("model"), new[] { hair, ears }, 100), Is.True, "another model");
            Assert.That(loaded.NeedsReload(null, new[] { hair, ears }, 100), Is.True, "model removed");
            Assert.That(loaded.NeedsReload(model, new[] { ears, hair }, 100), Is.True, "composition order");
            Assert.That(loaded.NeedsReload(model, new[] { hair }, 100), Is.True, "module removed");
            Assert.That(loaded.NeedsReload(model, new[] { hair, ears, null }, 100), Is.True, "empty slot added");
        }

        // 着せ替えで覚えた並びは、インスペクタに映っても読み直さない。
        [Test]
        public void DressUpUpdatesTheRememberedModules()
        {
            var model = new Asset("model");
            var hair = new Asset("hair", "hair-slot");
            var twin = new Asset("twin", "hair-slot");
            var sailor = new Asset("sailor");
            var modules = new List<Asset> { hair, sailor };
            var loaded = new LoadedSource<Asset, Asset>();
            loaded.Remember(model, modules, 100);
            ModuleOrder.Attach(modules, twin, m => m.Name, m => m.Slot);
            Assert.That(loaded.NeedsReload(model, modules, 100), Is.True);
            loaded.RememberModules(modules);
            Assert.That(loaded.NeedsReload(model, modules, 100), Is.False);
        }

        // T0426: 同じスロットのモジュールの差し替えを並びの末尾に加えていて、読み直すと合成の順が変わった。
        // C ABI の順との突き合わせは PuppetTests.RememberedModuleOrderMatchesTheNativeOrder。
        [Test]
        public void SlotReplacementKeepsThePosition()
        {
            var hair = new Asset("hair", "hair-slot");
            var sailor = new Asset("sailor");
            var twin = new Asset("twin", "hair-slot");
            var ears = new Asset("ears");
            var modules = new List<Asset> { hair, sailor };
            ModuleOrder.Attach(modules, twin, m => m.Name, m => m.Slot);
            Assert.That(modules, Is.EqualTo(new[] { twin, sailor }));
            ModuleOrder.Attach(modules, ears, m => m.Name, m => m.Slot);
            Assert.That(modules, Is.EqualTo(new[] { twin, sailor, ears }));
            // 同じ名前で別のスロットのものと、同じスロットのものの両方があれば、最初の位置に入れてほかを除く。
            var sailorInHair = new Asset("sailor", "hair-slot");
            ModuleOrder.Attach(modules, sailorInHair, m => m.Name, m => m.Slot);
            Assert.That(modules, Is.EqualTo(new[] { sailorInHair, ears }));
        }

        // --- 読み込みの途中の失敗 ---

        private sealed class Log
        {
            public readonly List<(LoadStage Stage, int Index)> Errors = new List<(LoadStage, int)>();
            public void Add(LoadStage stage, int index, Exception e) => Errors.Add((stage, index));
        }

        private static Func<Puppet> DressUpBody => () => Puppet.FromBundle(Paths.Read("dress-up", "dress-up.ugoe"));

        [Test]
        public void LoadsTheBodyAndModules()
        {
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            using var sailor = Module.FromBytes(Paths.Read("dress-up", "sailor.ugoemod"));
            var log = new Log();
            var finished = 0;
            using var puppet = PuppetLoad.Run(DressUpBody, new[] { hair, sailor }, m => m, _ => finished++, log.Add);
            Assert.That(puppet, Is.Not.Null);
            Assert.That(finished, Is.EqualTo(1));
            Assert.That(log.Errors, Is.Empty);
            Assert.That(puppet.AttachedModuleNames, Is.EqualTo(new[] { hair.Name, sailor.Name }));
        }

        [Test]
        public void UnreadableBodyLoadsNothing()
        {
            var log = new Log();
            var finished = false;
            var puppet = PuppetLoad.Run(() => Puppet.FromBundle(new byte[] { 1, 2, 3 }), new Module[0], m => m,
                _ => finished = true, log.Add);
            Assert.That(puppet, Is.Null);
            Assert.That(finished, Is.False);
            Assert.That(log.Errors, Is.EqualTo(new[] { (LoadStage.Create, -1) }));
        }

        // 読めないモジュール (壊れた .ugoemod) と空の欄は飛ばして、ほかのモジュールと本体を表示する。
        [Test]
        public void UnreadableModuleIsSkipped()
        {
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            using var sailor = Module.FromBytes(Paths.Read("dress-up", "sailor.ugoemod"));
            var sources = new[] { "hair", "broken", null, "sailor" };
            var log = new Log();
            using var puppet = PuppetLoad.Run(DressUpBody, sources, s => s switch
            {
                null => null,
                "hair" => hair,
                "sailor" => sailor,
                _ => Module.FromBytes(new byte[] { 1, 2, 3 }),
            }, _ => { }, log.Add);
            Assert.That(puppet, Is.Not.Null);
            Assert.That(log.Errors, Is.EqualTo(new[] { (LoadStage.ReadModule, 1) }));
            Assert.That(puppet.AttachedModuleNames, Is.EqualTo(new[] { hair.Name, sailor.Name }));
        }

        // 本体に合わないモジュールは付けず、本体だけを表示する。
        [Test]
        public void ModulesThatDoNotFitLeaveTheBody()
        {
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            var log = new Log();
            using var puppet = PuppetLoad.Run(() => Puppet.FromBundle(Paths.Read("avatar.ugoe")), new[] { hair }, m => m,
                _ => { }, log.Add);
            Assert.That(puppet, Is.Not.Null);
            Assert.That(log.Errors, Is.EqualTo(new[] { (LoadStage.AttachModules, -1) }));
            Assert.That(puppet.AttachedModuleNames, Is.Empty);
        }

        // T0426: テクスチャや描画の準備の途中で失敗すると作りかけの Puppet が残り、LateUpdate が毎フレーム例外を出した。
        [Test]
        public void FailureWhilePreparingLeavesNoPuppet()
        {
            Puppet seen = null;
            var log = new Log();
            var puppet = PuppetLoad.Run(DressUpBody, new Module[0], m => m, p =>
            {
                seen = p;
                throw new InvalidOperationException("texture decode failed");
            }, log.Add);
            Assert.That(puppet, Is.Null, "a half-loaded puppet would be evaluated every frame");
            Assert.That(seen, Is.Not.Null);
            Assert.That(seen.IsDisposed, Is.True);
            Assert.That(log.Errors, Is.EqualTo(new[] { (LoadStage.Finish, -1) }));
        }

        // --- 送り直す範囲 ---

        private static DrawBatch OneBatch(DrawableInfo[] drawables, DrawableState[] states)
        {
            var order = Enumerable.Range(0, drawables.Length).ToArray();
            var plan = DrawBatchPlan.Build(drawables, states, order, MaskPlan.Build(drawables), true);
            Assert.That(plan.Batches, Has.Count.EqualTo(1));
            return plan.Batches[0];
        }

        [Test]
        public void FullUploadSendsEverything()
        {
            var drawables = new[] { Mesh("a"), Mesh("b") };
            var states = new[] { State(), State() };
            var upload = new BatchUpload();
            upload.Plan(OneBatch(drawables, states), drawables, new[] { false, false }, states, true, false);
            Assert.That((upload.Vertices, upload.Colors, upload.Indices), Is.EqualTo((true, true, true)));
            Assert.That(upload.MemberPositions, Is.EqualTo(new[] { true, true }));
            Assert.That(upload.MemberColors, Is.EqualTo(new[] { true, true }));
        }

        [Test]
        public void NothingChangedSendsNothing()
        {
            var drawables = new[] { Mesh("a"), Mesh("b", culling: true) };
            var states = new[] { State(DrawableChanges.DrawOrder), State() };
            var upload = new BatchUpload();
            upload.Plan(OneBatch(drawables, states), drawables, new[] { false, false }, states, false, false);
            Assert.That((upload.Vertices, upload.Colors, upload.Indices), Is.EqualTo((false, false, false)));
            Assert.That(upload.MemberPositions, Is.EqualTo(new[] { false, false }));
            Assert.That(upload.MemberColors, Is.EqualTo(new[] { false, false }));
        }

        [Test]
        public void MovedVerticesAreSentOnlyForTheMovedMember()
        {
            var drawables = new[] { Mesh("a"), Mesh("culled", culling: true) };
            var states = new[] { State(DrawableChanges.Positions), State() };
            var batch = OneBatch(drawables, states);
            var upload = new BatchUpload();
            upload.Plan(batch, drawables, new[] { true, false }, states, false, false);
            Assert.That((upload.Vertices, upload.Colors, upload.Indices), Is.EqualTo((true, false, false)));
            Assert.That(upload.MemberPositions, Is.EqualTo(new[] { true, false }));

            // 裏面カリングする要素が動くと、表を向く三角形が変わる。
            upload.Plan(batch, drawables, new[] { false, true }, states, false, false);
            Assert.That((upload.Vertices, upload.Colors, upload.Indices), Is.EqualTo((true, false, true)));
            Assert.That(upload.MemberPositions, Is.EqualTo(new[] { false, true }));
        }

        [Test]
        public void ColorChangesAreSentOnlyForTheChangedMember()
        {
            var drawables = new[] { Mesh("a"), Mesh("b"), Mesh("c"), Mesh("d"), Mesh("e") };
            var changes = new[]
            {
                DrawableChanges.Opacity, DrawableChanges.MultiplyColor, DrawableChanges.ScreenColor, DrawableChanges.MaskStrength,
                DrawableChanges.None,
            };
            var states = changes.Select(c => State(c)).ToArray();
            var upload = new BatchUpload();
            upload.Plan(OneBatch(drawables, states), drawables, new bool[5], states, false, false);
            Assert.That((upload.Vertices, upload.Colors, upload.Indices), Is.EqualTo((false, true, false)));
            Assert.That(upload.MemberColors, Is.EqualTo(new[] { true, true, true, true, false }));
        }

        [Test]
        public void VisibilityRewritesTheTriangles()
        {
            var drawables = new[] { Mesh("a"), Mesh("b") };
            var states = new[] { State(), State(DrawableChanges.Visibility) };
            var upload = new BatchUpload();
            upload.Plan(OneBatch(drawables, states), drawables, new[] { false, false }, states, false, false);
            Assert.That((upload.Vertices, upload.Colors, upload.Indices), Is.EqualTo((false, false, true)));
        }

        // マスクのページの配置が変わると、すべての要素の頂点属性 (ページの中の位置) を送り直す。
        [Test]
        public void AllColorsSendsEveryMembersAttributes()
        {
            var drawables = new[] { Mesh("a"), Mesh("b") };
            var states = new[] { State(), State() };
            var upload = new BatchUpload();
            upload.Plan(OneBatch(drawables, states), drawables, new[] { false, false }, states, false, true);
            Assert.That((upload.Vertices, upload.Colors, upload.Indices), Is.EqualTo((false, true, false)));
            Assert.That(upload.MemberColors, Is.EqualTo(new[] { true, true }));
        }

        [Test]
        public void UploadFollowsTheBatchSize()
        {
            var upload = new BatchUpload();
            var two = new[] { Mesh("a"), Mesh("b") };
            var twoStates = new[] { State(), State() };
            upload.Plan(OneBatch(two, twoStates), two, new[] { true, false }, twoStates, false, false);
            var three = new[] { Mesh("a"), Mesh("b"), Mesh("c") };
            var threeStates = new[] { State(), State(), State() };
            upload.Plan(OneBatch(three, threeStates), three, new[] { false, false, true }, threeStates, false, false);
            Assert.That(upload.MemberPositions, Is.EqualTo(new[] { false, false, true }));
        }

        // --- バッチを作り直す条件 ---

        [Test]
        public void BatchRebuildConditions()
        {
            var drawables = new[] { Mesh("a"), Mesh("b") };
            var states = new[] { State(), State() };
            var plan = DrawBatchPlan.Build(drawables, states, new[] { 0, 1 }, MaskPlan.Build(drawables), true);
            Assert.That(SyncRules.NeedsBatchRebuild(null, false, true, true, states), Is.True, "no plan yet");
            Assert.That(SyncRules.NeedsBatchRebuild(plan, false, true, true, states), Is.False);
            Assert.That(SyncRules.NeedsBatchRebuild(plan, true, true, true, states), Is.True, "render order");
            Assert.That(SyncRules.NeedsBatchRebuild(plan, false, false, true, states), Is.True, "batching turned off");
            var additive = new[] { State(), State(blend: BlendMode.Additive) };
            Assert.That(SyncRules.NeedsBatchRebuild(plan, false, true, true, additive), Is.True, "blend mode");
        }

        // --- マスクを描き直す条件 ---

        // 0: マスク用メッシュ、1: 0 を読む、2: マスクに関わらない、3: 1 を読む (1 はマスクを読むマスク用メッシュ)
        private static MaskPlan NestedMasks() => MaskPlan.Build(new[]
        {
            Mesh("mask"), Mesh("nested", masks: new MaskRef(0, false)), Mesh("plain"), Mesh("reader", masks: new MaskRef(1, false)),
        });

        [Test]
        public void MaskContentFollowsSourcesAndReaders()
        {
            var plan = NestedMasks();
            var still = new[] { State(), State(), State(), State() };
            bool Moved(int i) => SyncRules.MaskContentChanged(plan, Enumerable.Range(0, 4).Select(k => k == i).ToArray(), still);
            Assert.That(Moved(0), Is.True, "the mask source moved");
            Assert.That(Moved(3), Is.True, "a reader moved (the content is drawn over the reader)");
            Assert.That(Moved(2), Is.False, "an unrelated mesh moved");
            Assert.That(SyncRules.MaskContentChanged(plan, new bool[4], still), Is.False);
        }

        [Test]
        public void MaskStrengthMattersOnlyForNestedSources()
        {
            var plan = NestedMasks();
            var none = new bool[4];
            DrawableState[] Strength(int i) =>
                Enumerable.Range(0, 4).Select(k => State(k == i ? DrawableChanges.MaskStrength : DrawableChanges.None)).ToArray();
            Assert.That(SyncRules.MaskContentChanged(plan, none, Strength(1)), Is.True, "a source that reads a mask");
            // 読むだけの要素の効き具合は頂点属性で渡すので、内容は変わらない。
            Assert.That(SyncRules.MaskContentChanged(plan, none, Strength(3)), Is.False);
            Assert.That(SyncRules.MaskContentChanged(plan, none, Strength(0)), Is.False);
        }

        [Test]
        public void MaskRedrawConditions()
        {
            Assert.That(SyncRules.NeedsMaskRedraw(false, true, 2, 1, true), Is.False, "no masks");
            Assert.That(SyncRules.NeedsMaskRedraw(true, false, 1, 1, false), Is.False);
            Assert.That(SyncRules.NeedsMaskRedraw(true, true, 1, 1, false), Is.True, "content changed");
            Assert.That(SyncRules.NeedsMaskRedraw(true, false, 1, 1, true), Is.True, "pages lost");
            Assert.That(SyncRules.NeedsMaskRedraw(true, false, 1.1892071f, 1, false), Is.True, "resolution changed");
            Assert.That(SyncRules.NeedsMaskRedraw(true, false, 1, 0, false), Is.True, "never drawn");
            Assert.That(SyncRules.NeedsMaskRedraw(true, false, 1.0000001f, 1, false), Is.False, "rounding noise");
        }
    }
}
