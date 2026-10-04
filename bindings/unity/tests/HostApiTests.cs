// SPDX-License-Identifier: MIT
// T0412・T0414 で C ABI に足したホストの操作 (パーツ・ポーズの表示の選択・効果の差し替え・モーションのイベント) を、
// 公開仕様の例 (spec/examples/) の期待値と比べる。
using System.IO;
using System.Linq;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class HostApiTests
    {
        private static Puppet FromExample(string name) =>
            Puppet.FromJson(File.ReadAllText(Paths.Example(name, "model.json")));

        private static void AssertPart(Puppet puppet, string id, float value, float weight)
        {
            var o = puppet.GetPartOpacity(puppet.FindPart(id));
            Assert.That(o.Weight, Is.EqualTo(weight).Within(1e-4), $"{id} weight");
            // 重み 0 (上書きなし) の値は使われない。
            if (weight > 0) Assert.That(o.Value, Is.EqualTo(value).Within(1e-4), $"{id} value");
        }

        [Test]
        public void PartsAreListedInModelOrder()
        {
            using var puppet = FromExample("pose-select");
            Assert.That(puppet.PartIds, Is.EqualTo(new[] { "Root", "ArmA", "ArmB", "SleeveB", "Hat" }));
            for (var i = 0; i < puppet.PartIds.Count; i++) Assert.That(puppet.FindPart(puppet.PartIds[i]), Is.EqualTo(i));
            Assert.That(puppet.FindPart("Missing"), Is.EqualTo(-1));
            Assert.That(puppet.FindPart(null), Is.EqualTo(-1));
            // C の文字列は NUL で終わるので、途中に NUL のある ID を渡すと手前だけで引いてしまう。渡す前に断る。
            Assert.Throws<System.ArgumentException>(() => puppet.FindPart("Root\0Hat"));
            var e = Assert.Throws<UgoeException>(() => puppet.GetPartOpacity(puppet.PartIds.Count));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.OutOfRange));
        }

        // spec/examples/pose-select/timeline.json のフレーム 0〜5 (モーションを使わない部分)。
        [Test]
        public void ShowPoseFadesLikeThePoseSelectExample()
        {
            using var puppet = FromExample("pose-select");
            // 最初の update の前は上書きが無い。
            Assert.That(puppet.GetPartOpacity(puppet.FindPart("ArmA")).Weight, Is.EqualTo(0));

            puppet.Update(0.125f);
            AssertPart(puppet, "ArmA", 1, 1);
            AssertPart(puppet, "ArmB", 0, 1);
            AssertPart(puppet, "Hat", 0, 0);

            Assert.That(puppet.ShowPose("ArmB"), Is.True);
            puppet.Update(0.125f);
            AssertPart(puppet, "ArmA", 0.75f, 1);
            AssertPart(puppet, "ArmB", 0.25f, 1);
            AssertPart(puppet, "SleeveB", 0.25f, 1);

            puppet.Update(0.125f);
            AssertPart(puppet, "ArmA", 0.5f, 1);
            AssertPart(puppet, "ArmB", 0.5f, 1);

            Assert.That(puppet.ShowPose("ArmA"), Is.True);
            puppet.Update(0.125f);
            AssertPart(puppet, "ArmA", 0.75f, 1);
            puppet.Update(0.125f);
            AssertPart(puppet, "ArmA", 1, 1);

            // リンクのパーツと、ポーズの外のパーツは何も変えない。
            Assert.That(puppet.ShowPose("SleeveB"), Is.False);
            Assert.That(puppet.ShowPose("Hat"), Is.False);
            Assert.That(puppet.ShowPose("Missing"), Is.False);
            puppet.Update(0.125f);
            AssertPart(puppet, "ArmA", 1, 1);
            AssertPart(puppet, "ArmB", 0, 1);
        }

        private static void AssertParameters(Puppet puppet, float eye, float breath, float mouth, float mouth2)
        {
            Assert.That(puppet.GetParameter(puppet.FindParameter("EyeL")), Is.EqualTo(eye).Within(1e-3), "EyeL");
            Assert.That(puppet.GetParameter(puppet.FindParameter("Breath")), Is.EqualTo(breath).Within(1e-3), "Breath");
            Assert.That(puppet.GetParameter(puppet.FindParameter("Mouth")), Is.EqualTo(mouth).Within(1e-3), "Mouth");
            Assert.That(puppet.GetParameter(puppet.FindParameter("Mouth2")), Is.EqualTo(mouth2).Within(1e-3), "Mouth2");
        }

        // spec/examples/effects-replace/timeline.json。
        [Test]
        public void EffectsReplacementMatchesTheExample()
        {
            const string effects = @"{
                ""eye_blink"": { ""parameters"": [""EyeL""], ""interval"": 0.4, ""closing"": 0.1, ""closed"": 0.05,
                                 ""opening"": 0.15, ""seed"": 42 },
                ""breath"": { ""parameters"": [{ ""parameter"": ""Breath"", ""offset"": 0.1, ""peak"": 0.25,
                                                 ""cycle"": 1.0, ""weight"": 1.0 }] },
                ""lip_sync"": { ""parameters"": [""Mouth2""] }
            }";
            using var puppet = FromExample("effects-replace");
            puppet.SetLipSyncLevel(0.6f);
            puppet.Update(0.1f);
            AssertParameters(puppet, 1, 0.1545f, 0.6f, 0);
            puppet.Update(0.1f);
            AssertParameters(puppet, 1, 0.2939f, 0.6f, 0);

            // 2 回目の差し替えは同じ設定でも初めからになるので、同じ値を繰り返す。
            for (var round = 0; round < 2; round++)
            {
                puppet.SetEffectsJson(effects);
                puppet.Update(0.1f);
                AssertParameters(puppet, 1, 0.2469f, 0, 0.6f);
                puppet.Update(0.1f);
                puppet.Update(0.1f);
                AssertParameters(puppet, 1, 0.3378f, 0, 0.6f);
                puppet.Update(0.1f);
                puppet.Update(0.1f);
                AssertParameters(puppet, 0.9663f, 0.1f, 0, 0.6f);
                puppet.Update(0.1f);
                AssertParameters(puppet, 0, -0.0469f, 0, 0.6f);
            }

            puppet.SetEffectsJson("{}");
            puppet.Update(0.1f);
            AssertParameters(puppet, 1, 0, 0, 0);

            var e = Assert.Throws<UgoeException>(() => puppet.SetEffectsJson("{"));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            Assert.Throws<System.ArgumentNullException>(() => puppet.SetEffectsJson(null));
        }

        // spec/examples/motion-crossfade/timeline.json のフレーム 0〜3 (バンドルの motion-crossfade.ugoe)。
        [Test]
        public void MotionEventsFireLikeTheCrossfadeExample()
        {
            using var puppet = Puppet.FromBundle(File.ReadAllBytes(Paths.Example("bundles", "motion-crossfade.ugoe")));
            Assert.That(puppet.StartMotion("idle", MotionPriority.Idle), Is.True);
            puppet.Update(0.25f);
            Assert.That(puppet.MotionEvents().Select(e => e.Value), Is.EqualTo(new[] { "idle-start" }));
            puppet.Update(0.25f);
            var half = puppet.MotionEvents();
            Assert.That(half.Select(e => e.Value), Is.EqualTo(new[] { "idle-half" }));
            Assert.That(half[0].Time, Is.EqualTo(0.5f).Within(1e-5));
            puppet.Update(0.25f);
            Assert.That(puppet.MotionEvents(), Is.Empty);
            // 長いフレームで過ぎたループの分もすべて発火する。
            puppet.Update(2.5f);
            Assert.That(puppet.MotionEvents().Select(e => e.Value),
                Is.EqualTo(new[] { "idle-start", "idle-half", "idle-start", "idle-half", "idle-start" }));
        }
    }
}
