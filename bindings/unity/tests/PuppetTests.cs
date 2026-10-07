// SPDX-License-Identifier: MIT
// Runtime/Core を、ホストの libugoe_ffi と assets/samples のモデルで検査する。
using System;
using System.Linq;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class PuppetTests
    {
        [Test]
        public void VersionAndAbi()
        {
            Assert.That(UgoeRuntime.AbiVersion, Is.EqualTo(2u));
            Assert.That(UgoeRuntime.Version, Does.Match(@"^\d+\.\d+\.\d+"));
        }

        [Test]
        public void LoadsTheSampleAvatar()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            Assert.That(puppet.Canvas.Width, Is.GreaterThan(0));
            Assert.That(puppet.Canvas.Height, Is.GreaterThan(0));
            Assert.That(puppet.Drawables.Count, Is.GreaterThan(0));
            Assert.That(puppet.Parameters.Select(p => p.Id), Does.Contain("AngleX"));
            Assert.That(puppet.TextureCount, Is.GreaterThan(0));
            Assert.That(puppet.LoadWarnings, Is.Empty);

            foreach (var d in puppet.Drawables)
            {
                Assert.That(d.Id, Is.Not.Empty);
                Assert.That(d.Texture, Is.LessThan(puppet.TextureCount));
                Assert.That(d.IndexCount % 3, Is.EqualTo(0));
                Assert.That(puppet.GetUvs(Array.IndexOf(puppet.Drawables.ToArray(), d)).Length, Is.EqualTo(d.VertexCount * 2));
            }

            var texture = puppet.GetTexture(0);
            Assert.That(texture.Format, Is.EqualTo(EncodedTextureFormat.Png));
            // PNG の署名
            Assert.That(texture.Data.Take(4), Is.EqualTo(new byte[] { 0x89, 0x50, 0x4e, 0x47 }));
        }

        [Test]
        public void ParametersClampAndMovePositions()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            var angle = puppet.FindParameter("AngleX");
            var info = puppet.Parameters[angle];
            Assert.That(puppet.FindParameter("NoSuchParameter"), Is.EqualTo(-1));

            puppet.SetParameter(angle, info.Max + 100);
            Assert.That(puppet.GetParameter(angle), Is.EqualTo(info.Max).Within(1e-5));
            Assert.That(puppet.SetParameter("NoSuchParameter", 1), Is.False);

            var sync = new FrameSync(puppet);
            puppet.SetParameter(angle, info.Min);
            puppet.Update(0);
            sync.Pull();
            var before = sync.Positions.Select(p => (float[])p.Clone()).ToArray();
            puppet.SetParameter(angle, info.Max);
            puppet.Update(0);
            sync.Pull();
            var moved = Enumerable.Range(0, before.Length).Count(i => !before[i].SequenceEqual(sync.Positions[i]));
            Assert.That(moved, Is.GreaterThan(0), "AngleX should move some meshes");

            var applied = puppet.SetParameters(new[] { "AngleX", "Missing", "AngleY" }, new[] { 12f, 1f, 5f });
            Assert.That(applied, Is.EqualTo(2));
            Assert.That(puppet.GetParameter(angle), Is.EqualTo(12f).Within(1e-5));
        }

        [Test]
        public void ChangeFlagsSkipUnchangedTransfers()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            // 時間で動く段 (物理・目パチ・呼吸) を止めると、パラメータを変えない限り何も変わらない。
            foreach (UpdateStage stage in Enum.GetValues(typeof(UpdateStage))) puppet.SetStageEnabled(stage, false);
            var sync = new FrameSync(puppet);

            puppet.Update(0);
            sync.Pull();
            Assert.That(sync.PositionsCopied, Is.EqualTo(puppet.Drawables.Count), "the first update transfers everything");
            Assert.That(sync.RenderOrderChanged, Is.True);
            Assert.That(sync.RenderOrder.OrderBy(i => i), Is.EqualTo(Enumerable.Range(0, puppet.Drawables.Count)));

            puppet.Update(1f / 60);
            sync.Pull();
            Assert.That(sync.PositionsCopied, Is.EqualTo(0));
            Assert.That(sync.RenderOrderChanged, Is.False);
            Assert.That(sync.States.All(s => s.Changes == DrawableChanges.None), Is.True);

            // 変更フラグによらずすべてを取り出しても、同じ値になる (新しく作った FrameSync でも)。
            var all = new FrameSync(puppet);
            all.PullAll();
            Assert.That(all.PositionsCopied, Is.EqualTo(puppet.Drawables.Count));
            Assert.That(all.PositionsChanged.All(c => c), Is.True);
            for (var i = 0; i < puppet.Drawables.Count; i++) Assert.That(all.Positions[i], Is.EqualTo(sync.Positions[i]));
            Assert.That(all.RenderOrder, Is.EqualTo(sync.RenderOrder));

            puppet.SetParameter("AngleX", 20);
            puppet.Update(1f / 60);
            sync.Pull();
            Assert.That(sync.PositionsCopied, Is.GreaterThan(0));
            Assert.That(sync.PositionsCopied, Is.LessThan(puppet.Drawables.Count), "only the meshes that AngleX moves");
        }

        [Test]
        public void MotionsAndExpressions()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            if (puppet.MotionNames.Count > 0)
            {
                Assert.That(puppet.StartMotion(0, MotionPriority.Force), Is.True);
                puppet.Update(0.1f);
            }
            if (puppet.ExpressionNames.Count > 0)
            {
                puppet.SetExpression(puppet.ExpressionNames[0]);
                puppet.Update(0.1f);
                puppet.StopExpressions();
            }
            Assert.Throws<ArgumentException>(() => puppet.SetExpression("no-such-expression"));
            puppet.SetLipSyncLevel(0.5f);
            puppet.SetLipSyncLevel(null);
            puppet.Update(0.1f);
        }

        [Test]
        public void PhysicsJsonRoundTripsAndResets()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            var json = puppet.PhysicsJson();
            Assert.That(json, Is.Not.Null.And.Not.Empty);
            puppet.Update(1f / 60);
            puppet.SetPhysicsJson(json);
            Assert.That(puppet.PhysicsJson(), Is.EqualTo(json));
            puppet.ResetPhysics();
            puppet.Update(1f / 60);

            var e = Assert.Throws<UgoeException>(() => puppet.SetPhysicsJson("{"));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            Assert.That(puppet.PhysicsJson(), Is.EqualTo(json), "a failed replacement changes nothing");

            puppet.SetPhysicsJson(null);
            Assert.That(puppet.PhysicsJson(), Is.Null);
        }

        [Test]
        public void MotionEventsAreEmptyWithoutEvents()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            Assert.That(puppet.MotionEvents(), Is.Empty);
            if (puppet.MotionNames.Count > 0) puppet.StartMotion(0, MotionPriority.Force);
            puppet.Update(0.5f);
            Assert.That(puppet.MotionEvents().All(ev => ev.Value != null), Is.True);
        }

        [Test]
        public void BrokenFilesThrowFormatErrors()
        {
            var e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(new byte[] { 1, 2, 3 }));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            Assert.That(e.Message, Is.Not.Empty);

            var m = Assert.Throws<UgoeException>(() => Module.FromBytes(new byte[] { 1, 2, 3 }));
            Assert.That(m.Result, Is.EqualTo(UgoeResult.Format));
            Assert.That(m.Kind, Is.EqualTo("invalid_file"));
        }

        [Test]
        public void DressUpAttachesDetachesAndReportsTextures()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("dress-up", "dress-up.ugoe"));
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            using var twin = Module.FromBytes(Paths.Read("dress-up", "twin-tails.ugoemod"));
            using var sailor = Module.FromBytes(Paths.Read("dress-up", "sailor.ugoemod"));
            Assert.That(puppet.SlotIds, Does.Contain(hair.Slot));
            var baseDrawables = puppet.Drawables.Count;
            var baseTextures = puppet.TextureCount;

            puppet.AttachModule(hair);
            Assert.That(puppet.DressUpVersion, Is.EqualTo(1));
            Assert.That(puppet.AttachedModuleNames, Is.EqualTo(new[] { hair.Name }));
            Assert.That(puppet.Drawables.Count, Is.GreaterThan(baseDrawables));
            Assert.That(puppet.ChangedTextures, Is.Not.Empty);
            Assert.That(puppet.TextureCount, Is.GreaterThan(baseTextures));
            puppet.Update(0);

            // 同じスロットのモジュールは差し替える。
            Assert.That(twin.Slot, Is.EqualTo(hair.Slot));
            puppet.AttachModule(twin);
            Assert.That(puppet.AttachedModuleNames, Is.EqualTo(new[] { twin.Name }));

            puppet.SetModules(new[] { hair, sailor });
            Assert.That(puppet.AttachedModuleNames, Is.EqualTo(new[] { hair.Name, sailor.Name }));
            puppet.Update(1f / 60);

            puppet.DetachModule(hair.Name);
            Assert.That(puppet.AttachedModuleNames, Is.EqualTo(new[] { sailor.Name }));
            var e = Assert.Throws<UgoeException>(() => puppet.DetachModule(hair.Name));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.DressUp));
            Assert.That(e.Kind, Is.EqualTo("not_attached"));
            Assert.That(e.Fields["module"], Is.EqualTo(hair.Name));

            puppet.SetModules(Array.Empty<Module>());
            Assert.That(puppet.AttachedModuleNames, Is.Empty);
            Assert.That(puppet.Drawables.Count, Is.EqualTo(baseDrawables));
            Assert.That(puppet.TextureCount, Is.EqualTo(baseTextures));
        }

        // UgoePuppet が覚えておくモジュールの並び (読み直しで SetModules に渡す) は、C ABI の付けた順と同じでなければならない。
        // 同じスロットのモジュールは、合成の順の同じ位置で差し替わる。
        [Test]
        public void RememberedModuleOrderMatchesTheNativeOrder()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("dress-up", "dress-up.ugoe"));
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            using var twin = Module.FromBytes(Paths.Read("dress-up", "twin-tails.ugoemod"));
            using var sailor = Module.FromBytes(Paths.Read("dress-up", "sailor.ugoemod"));
            using var ears = Module.FromBytes(Paths.Read("dress-up", "cat-ears.ugoemod"));
            var remembered = new System.Collections.Generic.List<Module> { hair, sailor };
            puppet.SetModules(remembered);
            foreach (var next in new[] { twin, ears, hair })
            {
                puppet.AttachModule(next);
                ModuleOrder.Attach(remembered, next, m => m.Name, m => m.Slot);
                Assert.That(remembered.Select(m => m.Name), Is.EqualTo(puppet.AttachedModuleNames), $"after attaching {next.Name}");
            }
            Assert.That(puppet.AttachedModuleNames[0], Is.EqualTo(hair.Name), "the replacement keeps the first position");
        }

        [Test]
        public void ModuleOutlivesItsHandle()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("dress-up", "dress-up.ugoe"));
            var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            puppet.AttachModule(hair);
            hair.Dispose();
            puppet.Update(1f / 60);
            Assert.That(puppet.AttachedModuleNames, Has.Count.EqualTo(1));
            var t = puppet.GetTexture(puppet.TextureCount - 1);
            Assert.That(t.Format, Is.EqualTo(EncodedTextureFormat.Png));
        }

        [Test]
        public void DressUpFailuresCarryKindAndFields()
        {
            using var avatar = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            var e = Assert.Throws<UgoeException>(() => avatar.AttachModule(hair));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.DressUp));
            Assert.That(e.Kind, Is.Not.Null);
            Assert.That(e.Fields["module"], Is.EqualTo(hair.Name));
            Assert.That(avatar.AttachedModuleNames, Is.Empty);
        }

        [Test]
        public void LightingModelHasNormalMaps()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar-lit.ugoe"));
            Assert.That(puppet.UsesLighting, Is.True);
            Assert.That(puppet.Drawables.Any(d => d.Lit), Is.True);
            var normals = Enumerable.Range(0, puppet.TextureCount).Select(puppet.GetNormalMap).ToArray();
            Assert.That(normals.Any(n => n.Format == EncodedTextureFormat.Png), Is.True);

            using var plain = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            Assert.That(plain.UsesLighting, Is.False);
            Assert.That(plain.GetNormalMap(0).Format, Is.EqualTo(EncodedTextureFormat.None));
        }

        [Test]
        public void LightingUniformLayout()
        {
            var u = LightingUniform.Create(0.1f, 0.2f, 0.3f, new[]
            {
                LightSource.Directional(0, 0, -2, 1, 1, 1),
                LightSource.Point(1, 2, 3, 2, 1, 0.5f, 4),
            });
            Assert.That(u.Ambient, Is.EqualTo(new[] { 0.1f, 0.2f, 0.3f, 0f }));
            Assert.That(u.Count, Is.EqualTo(2));
            // 平行光源は光源へ向かう単位ベクトル。
            Assert.That(u.Vectors.Take(4), Is.EqualTo(new[] { 0f, 0f, 1f, 0f }));
            Assert.That(u.Vectors.Skip(4).Take(4), Is.EqualTo(new[] { 1f, 2f, 3f, 1f }));
            Assert.That(u.Colors.Skip(4).Take(4), Is.EqualTo(new[] { 2f, 1f, 0.5f, 0.25f }));

            var d = LightingUniform.Default;
            Assert.That(d.Ambient, Is.EqualTo(new[] { 1f, 1f, 1f, 0f }));
            Assert.That(d.Count, Is.EqualTo(0));

            var e = Assert.Throws<UgoeException>(() =>
                LightingUniform.Create(1, 1, 1, new[] { LightSource.Directional(0, 0, 0, 1, 1, 1) }));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));

            var many = Enumerable.Range(0, 10).Select(i => LightSource.Directional(1, 0, 0, 1, 1, 1)).ToArray();
            Assert.That(LightingUniform.Create(1, 1, 1, many).Count, Is.EqualTo(LightingUniform.MaxLights));
        }

        [Test]
        public void HitAreas()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            puppet.Update(0);
            var c = puppet.Canvas;
            var hits = puppet.HitAreasAt(c.Width / 2, c.Height / 2);
            Assert.That(hits.All(i => i >= 0 && i < puppet.HitAreaIds.Count), Is.True);
            Assert.That(puppet.HitAreasAt(-1e6f, -1e6f), Is.Empty);
            foreach (var id in puppet.HitAreaIds) Assert.That(puppet.HitTest(id, -1e6f, -1e6f), Is.False);
            var e = Assert.Throws<UgoeException>(() => puppet.HitTest("NoSuchArea", 0, 0));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.NotFound));
            Assert.That(puppet.StandardHitArea("NoSuchName"), Is.EqualTo(-1));
            Assert.That(puppet.HitTestStandard("NoSuchName", 0, 0), Is.False);
        }

        [Test]
        public void TrackingWritesParameters()
        {
            Assert.That(TrackingInput.BlendShapeName(0), Is.Not.Empty);
            Assert.That(TrackingInput.BlendShapeIndex("eyeBlinkLeft"), Is.GreaterThanOrEqualTo(0));
            Assert.That(TrackingInput.BlendShapeIndex("_neutral"), Is.EqualTo(-1));

            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            var targets = puppet.TrackingTargets();
            var angle = puppet.FindParameter("AngleX");
            Assert.That(targets, Does.Contain(angle));
            Assert.That(targets, Is.Ordered);

            var input = new TrackingInput { HeadYaw = 10 };
            Assert.That(input.SetBlendShape("jawOpen", 0.5f), Is.True);
            puppet.SetTracking(input);
            puppet.Update(1f / 60);
            Assert.That(puppet.GetParameter(angle), Is.EqualTo(10).Within(1e-3));
            puppet.SetTracking(null);

            var bad = new TrackingInput { HeadYaw = float.NaN };
            var e = Assert.Throws<UgoeException>(() => puppet.SetTracking(bad));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
        }

        [Test]
        public void BrokenJsonThrows()
        {
            var e = Assert.Throws<UgoeException>(() => Puppet.FromJson("{"));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
        }
    }
}
