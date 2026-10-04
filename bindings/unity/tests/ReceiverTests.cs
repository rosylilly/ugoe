// SPDX-License-Identifier: MIT
// 受け手 (follow)・指揮役の値 (T0454)・トラッキングの優先・本体の物理・評価後のパーツの不透明度 (T0430 で C# に出した C ABI の関数)。
using System.IO;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class ReceiverTests
    {
        private static Puppet PoseSelect() =>
            Puppet.FromJson(File.ReadAllText(Paths.Example("pose-select", "model.json")));

        [Test]
        public void BaseValuesAreWrittenAndChecked()
        {
            using var puppet = PoseSelect();
            var x = puppet.Parameters[0];
            puppet.SetBaseValues(new[] { x.Max + 100 });
            Assert.That(puppet.GetParameter(0), Is.EqualTo(x.Max).Within(1e-5), "clamped to the range");

            var e = Assert.Throws<UgoeException>(() => puppet.SetBaseValues(new float[puppet.Parameters.Count + 1]));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            e = Assert.Throws<UgoeException>(() => puppet.SetBaseValues(new[] { float.NaN }));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            Assert.That(puppet.GetParameter(0), Is.EqualTo(x.Max).Within(1e-5), "a failed call changes nothing");
        }

        [Test]
        public void FollowAppliesValuesAndPartOpacities()
        {
            using var puppet = PoseSelect();
            var hat = puppet.FindPart("Hat");
            var x = puppet.Parameters[0];
            var value = (x.Min + x.Max) / 2;

            puppet.Follow(new[] { value }, null, 0, new[] { new PartOpacityInput(hat, 0.25f, 0.5f) }, 1f / 60);
            Assert.That(puppet.GetParameter(0), Is.EqualTo(value).Within(1e-5));
            Assert.That(puppet.GetPartOpacity(hat).Weight, Is.EqualTo(0.5f).Within(1e-5));
            // 評価後の値は lerp(1, 0.25, 0.5) × 親 (Root = 1)。
            Assert.That(puppet.GetPartEvaluatedOpacity(hat), Is.EqualTo(0.625f).Within(1e-5));
            Assert.That(puppet.GetPartEvaluatedOpacity(puppet.FindPart("Root")), Is.EqualTo(1f).Within(1e-5));

            puppet.Follow(new[] { value }, 0.5f, 1, null, 1f / 60);
            Assert.That(puppet.GetPartOpacity(hat).Weight, Is.EqualTo(0f), "no part opacities, no override");

            var e = Assert.Throws<UgoeException>(() => puppet.Follow(new float[2], null, 0, null, 1f / 60));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            e = Assert.Throws<UgoeException>(() => puppet.Follow(new[] { value }, null, 0, null, -1));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            // 有限でない値は C ABI と同じく断る (値の NaN・音量の NaN・強さと dt の無限大)。
            e = Assert.Throws<UgoeException>(() => puppet.Follow(new[] { float.NaN }, null, 0, null, 1f / 60));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            e = Assert.Throws<UgoeException>(() => puppet.Follow(new[] { value }, float.NaN, 1, null, 1f / 60));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            e = Assert.Throws<UgoeException>(() => puppet.Follow(new[] { value }, 0.5f, float.PositiveInfinity, null, 1f / 60));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            e = Assert.Throws<UgoeException>(() => puppet.Follow(new[] { value }, null, 0, null, float.PositiveInfinity));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            e = Assert.Throws<UgoeException>(() =>
                puppet.Follow(new[] { x.Min }, null, 0, new[] { new PartOpacityInput(hat, 1, float.NegativeInfinity) }, 1f / 60));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            Assert.That(puppet.GetParameter(0), Is.EqualTo(value).Within(1e-5), "a failed call changes nothing");
            // 値の配列が null なら C ABI を呼ばずに断る。
            Assert.Throws<System.ArgumentNullException>(() => puppet.Follow(null, null, 0, null, 1f / 60));
            e = Assert.Throws<UgoeException>(() =>
                puppet.Follow(new[] { value }, null, 0, new[] { new PartOpacityInput(hat, float.NaN, 1) }, 1f / 60));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
            Assert.Throws<UgoeException>(() => puppet.GetPartEvaluatedOpacity(puppet.PartIds.Count));
        }

        [Test]
        public void ConductorValuesDriveAFollower()
        {
            using var conductor = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            using var follower = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            Assert.That(conductor.ValuesBeforePhysics(), Has.Length.EqualTo(conductor.Parameters.Count));
            Assert.That(conductor.LipSyncApplied(), Is.Null, "nothing before the first update");
            Assert.That(conductor.PartOpacitiesApplied(), Is.Empty);

            conductor.SetStageEnabled(UpdateStage.Physics, false);
            conductor.SetParameter(conductor.FindParameter("AngleX"), 20);
            for (var frame = 0; frame < 30; frame++)
            {
                conductor.SetLipSyncLevel(frame % 3 / 2f);
                conductor.Update(1f / 60);
                var values = conductor.ValuesBeforePhysics();
                var lipSync = conductor.LipSyncApplied();
                Assert.That(lipSync.HasValue, Is.True);
                Assert.That(lipSync.Value.Level, Is.EqualTo(frame % 3 / 2f));
                follower.Follow(values, lipSync?.Level, lipSync?.Strength ?? 0, conductor.PartOpacitiesApplied(), 1f / 60);
                // 受け手も同じ値を持つ (さらに中継できる)。
                Assert.That(follower.ValuesBeforePhysics(), Is.EqualTo(values));
                Assert.That(follower.LipSyncApplied(), Is.EqualTo(lipSync));
            }
            Assert.That(conductor.ValuesBeforePhysics()[conductor.FindParameter("AngleX")], Is.EqualTo(20f).Within(1e-5));

            conductor.SetStageEnabled(UpdateStage.LipSync, false);
            conductor.Update(1f / 60);
            Assert.That(conductor.LipSyncApplied(), Is.Null, "the lip sync stage is off");
        }

        [Test]
        public void MotionOverTrackingAndKeptParameters()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            Assert.That(puppet.MotionOverTracking, Is.False);
            puppet.MotionOverTracking = true;
            Assert.That(puppet.MotionOverTracking, Is.True);

            puppet.SetTrackingKept(new[] { puppet.FindParameter("AngleX"), puppet.FindParameter("AngleY") });
            puppet.SetTrackingKept(null);
            puppet.SetTrackingKept(new int[0]);
            var e = Assert.Throws<UgoeException>(() => puppet.SetTrackingKept(new[] { puppet.Parameters.Count }));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.OutOfRange));
            Assert.Throws<System.ArgumentOutOfRangeException>(() => puppet.SetTrackingKept(new[] { -1 }));
        }

        [Test]
        public void BasePhysicsSurvivesDressUp()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            var json = puppet.PhysicsJson();
            Assert.That(json, Is.Not.Null);
            Assert.That(puppet.BasePhysicsJson(), Is.EqualTo(json), "without modules the base is the current setting");

            puppet.SetBasePhysicsJson(null);
            Assert.That(puppet.BasePhysicsJson(), Is.Null);
            Assert.That(puppet.PhysicsJson(), Is.Null);
            puppet.SetBasePhysicsJson(json);
            Assert.That(puppet.BasePhysicsJson(), Is.EqualTo(json));
            puppet.Update(1f / 60);

            var e = Assert.Throws<UgoeException>(() => puppet.SetBasePhysicsJson("{"));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            Assert.That(puppet.BasePhysicsJson(), Is.EqualTo(json));

            using var dressUp = Puppet.FromBundle(Paths.Read("dress-up", "dress-up.ugoe"));
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            var baseJson = dressUp.BasePhysicsJson();
            dressUp.AttachModule(hair);
            Assert.That(dressUp.BasePhysicsJson(), Is.EqualTo(baseJson), "the base does not include the module's rig");
        }
    }
}
