// SPDX-License-Identifier: MIT
// UgoePuppet の描き直しの印と LateUpdate の順 (Runtime/Core/ComponentState.cs の FrameState)。
using System.Collections.Generic;
using System.Globalization;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class FrameStateTests
    {
        // UgoePuppet の代わり。呼ばれた順を記録し、UgoePuppet と同じ時に FrameState の出来事を知らせる。
        private sealed class Host : IFrameHost
        {
            public readonly FrameState Frame = new FrameState();
            public readonly List<string> Calls = new List<string>();
            public bool Puppet;
            public bool LoadSucceeds = true;
            public bool Reload;
            public bool Lost;
            public bool Run;
            public float Dt = 0.5f;
            public bool Materials = true;
            public bool UseIntermediate;
            public bool IntermediateLost;
            public bool IntermediateExists;
            public int IntermediateDraws;
            // 評価で見た目が変わったか (UgoePuppet では FrameSync.PullIfChanged)。
            public bool Shape = true;

            public bool HasPuppet => Puppet;
            public bool MasksLost => Lost;
            public bool Running => Run;
            public float DeltaTime => Dt;

            public bool NeedsReload()
            {
                Calls.Add("NeedsReload");
                return Reload;
            }

            public void ApplySavedParameters() => Calls.Add("ApplySavedParameters");

            // UgoePuppet.Load: 印を下ろし、読めたら描画を作って (Built) 0 秒進める。
            public void Load()
            {
                Calls.Add("Load");
                Frame.Loaded();
                Puppet = LoadSucceeds;
                if (!Puppet) return;
                Frame.Built();
                Advance(0);
            }

            public void Advance(float dt)
            {
                Calls.Add($"Advance({dt.ToString(CultureInfo.InvariantCulture)})");
                Frame.Evaluated();
                if (Shape) Frame.Sent();
            }

            public void ApplyLights()
            {
                Calls.Add("ApplyLights");
                Frame.LightsApplied(Materials);
            }

            // UgoePuppet.RenderIntermediate と同じ手順。
            public void RenderIntermediate()
            {
                Calls.Add("RenderIntermediate");
                var use = UseIntermediate && Puppet;
                Frame.SetIntermediateUsed(use);
                if (!use) return;
                if (!IntermediateExists)
                {
                    IntermediateExists = true;
                    Frame.IntermediateCreated();
                }
                if (Frame.TakeIntermediate(IntermediateLost)) IntermediateDraws++;
            }

            // 1 フレーム分を回し、呼ばれた順を返す。
            public string[] Frame1()
            {
                Calls.Clear();
                Frame.LateUpdate(this);
                return Calls.ToArray();
            }
        }

        // 読み込みを済ませて、何も変わっていない状態にする。
        private static Host Loaded()
        {
            var host = new Host();
            host.Frame1();
            return host;
        }

        // --- 読み込み ---

        [Test]
        public void FirstFrameLoadsEvaluatesAndAppliesLights()
        {
            var host = new Host();
            Assert.That(host.Frame.NeedsLoad, Is.True);
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "Load", "Advance(0)", "ApplyLights", "RenderIntermediate" }));
            Assert.That(host.Frame.NeedsLoad, Is.False);
            Assert.That(host.Frame.Dirty, Is.False);
            Assert.That(host.Frame.LightsDirty, Is.False);
        }

        [Test]
        public void FailedLoadStopsTheFrame()
        {
            var host = new Host { LoadSucceeds = false };
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "Load" }));
            // 読み直すのは、何かが変わってから (毎フレーム読み直さない)。
            Assert.That(host.Frame1(), Is.Empty);
        }

        [Test]
        public void EnablingAndDisablingReload()
        {
            var host = Loaded();
            host.Frame.Disabled();
            Assert.That(host.Frame.NeedsLoad, Is.True);
            host.Frame1();
            host.Frame.Enabled();
            Assert.That(host.Frame1()[0], Is.EqualTo("Load"));
        }

        [Test]
        public void PixelScaleChangeReloadsOnlyALoadedPuppet()
        {
            var host = new Host();
            host.Frame.PixelScaleChanged(false);
            host.Frame1();
            host.Frame.PixelScaleChanged(false);
            Assert.That(host.Frame.NeedsLoad, Is.False, "nothing loaded yet");
            host.Frame.PixelScaleChanged(true);
            Assert.That(host.Frame1()[0], Is.EqualTo("Load"));
        }

        // --- 評価 ---

        [Test]
        public void StoppedAndUnchangedDoesNotEvaluate()
        {
            var host = Loaded();
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "RenderIntermediate" }));
        }

        [Test]
        public void ParameterChangeEvaluatesOnceWhileStopped()
        {
            var host = Loaded();
            host.Frame.Changed();
            Assert.That(host.Frame.Dirty, Is.True);
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "Advance(0)", "RenderIntermediate" }));
            Assert.That(host.Frame.Dirty, Is.False);
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "RenderIntermediate" }));
        }

        [Test]
        public void RunningAdvancesEveryFrame()
        {
            var host = Loaded();
            host.Run = true;
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "Advance(0.5)", "RenderIntermediate" }));
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "Advance(0.5)", "RenderIntermediate" }));
        }

        [Test]
        public void NegativeOrNonFiniteTimeAdvancesByZero()
        {
            Assert.That(FrameState.ClampDeltaTime(-1), Is.EqualTo(0));
            Assert.That(FrameState.ClampDeltaTime(float.NaN), Is.EqualTo(0));
            Assert.That(FrameState.ClampDeltaTime(float.PositiveInfinity), Is.EqualTo(0));
            Assert.That(FrameState.ClampDeltaTime(float.NegativeInfinity), Is.EqualTo(0));
            Assert.That(FrameState.ClampDeltaTime(0.25f), Is.EqualTo(0.25f));
            var host = Loaded();
            host.Run = true;
            host.Dt = -0.5f;
            Assert.That(host.Frame1()[0], Is.EqualTo("Advance(0)"));
            host.Dt = float.NaN;
            Assert.That(host.Frame1()[0], Is.EqualTo("Advance(0)"));
        }

        [Test]
        public void LostMasksEvaluateWhileStopped()
        {
            var host = Loaded();
            host.Lost = true;
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "Advance(0)", "RenderIntermediate" }));
        }

        [Test]
        public void EvaluationMarksTheIntermediate()
        {
            var host = Loaded();
            Assert.That(host.Frame.IntermediateDirty, Is.True, "not drawn because it is not used");
            host.Frame.TakeIntermediate(false);
            host.Frame.Changed();
            host.Frame1();
            Assert.That(host.Frame.IntermediateDirty, Is.True);
        }

        [Test]
        public void UnchangedEvaluationKeepsTheIntermediate()
        {
            var host = Loaded();
            host.Frame.TakeIntermediate(false);
            host.Shape = false;
            host.Frame.Changed();
            host.Frame1();
            Assert.That(host.Frame.Dirty, Is.False, "evaluated even though nothing was sent");
            Assert.That(host.Frame.IntermediateDirty, Is.False);
        }

        // --- 光源 ---

        [Test]
        public void LightChangeAppliesLightsWithoutEvaluating()
        {
            var host = Loaded();
            host.Frame.LightsChanged();
            Assert.That(host.Frame.LightsDirty, Is.True);
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "ApplyLights", "RenderIntermediate" }));
            Assert.That(host.Frame.LightsDirty, Is.False);
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "RenderIntermediate" }));
        }

        [Test]
        public void AppliedLightsMarkTheIntermediateOnlyWithMaterials()
        {
            var frame = new FrameState();
            frame.LightsApplied(false);
            Assert.That(frame.LightsDirty, Is.False);
            Assert.That(frame.IntermediateDirty, Is.False, "no materials yet");
            frame.LightsApplied(true);
            Assert.That(frame.IntermediateDirty, Is.True);
        }

        [Test]
        public void LightsAreAppliedAfterEvaluation()
        {
            var host = Loaded();
            host.Frame.Changed();
            host.Frame.LightsChanged();
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "Advance(0)", "ApplyLights", "RenderIntermediate" }));
        }

        // --- インスペクタでの変更 ---

        [Test]
        public void InspectorChangeReappliesParametersAndLights()
        {
            var host = Loaded();
            host.Frame.InspectorChanged();
            Assert.That(host.Frame1(), Is.EqualTo(new[]
            {
                "NeedsReload", "ApplySavedParameters", "Advance(0)", "ApplyLights", "RenderIntermediate",
            }));
            Assert.That(host.Frame.Validated, Is.False);
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "RenderIntermediate" }));
        }

        [Test]
        public void InspectorChangeOfTheSourceReloads()
        {
            var host = Loaded();
            host.Reload = true;
            host.Frame.InspectorChanged();
            Assert.That(host.Frame1(), Is.EqualTo(new[]
            {
                "NeedsReload", "Load", "Advance(0)", "ApplyLights", "RenderIntermediate",
            }));
        }

        [Test]
        public void InspectorChangeWithoutAPuppetDoesNothingElse()
        {
            var host = new Host { LoadSucceeds = false };
            host.Frame1();
            host.Frame.InspectorChanged();
            Assert.That(host.Frame1(), Is.EqualTo(new[] { "NeedsReload" }), "no puppet to set parameters on");
        }

        // --- 中間のテクスチャ ---

        [Test]
        public void IntermediateIsDrawnOnlyWhenMarked()
        {
            var host = Loaded();
            host.UseIntermediate = true;
            host.Frame1();
            Assert.That(host.Frame.IntermediateUsed, Is.True);
            Assert.That(host.IntermediateDraws, Is.EqualTo(1), "switching draws");
            host.Frame1();
            Assert.That(host.IntermediateDraws, Is.EqualTo(1), "unchanged");
            host.Frame.Changed();
            host.Frame1();
            Assert.That(host.IntermediateDraws, Is.EqualTo(2), "evaluated");
            host.Frame.LightsChanged();
            host.Frame1();
            Assert.That(host.IntermediateDraws, Is.EqualTo(3), "lights applied");
            host.IntermediateLost = true;
            host.Frame1();
            Assert.That(host.IntermediateDraws, Is.EqualTo(4), "texture lost");
        }

        [Test]
        public void SwitchingTheIntermediateMarksIt()
        {
            var frame = new FrameState();
            Assert.That(frame.SetIntermediateUsed(false), Is.False);
            Assert.That(frame.SetIntermediateUsed(true), Is.True);
            Assert.That(frame.TakeIntermediate(false), Is.True);
            Assert.That(frame.TakeIntermediate(false), Is.False);
            Assert.That(frame.SetIntermediateUsed(true), Is.False);
            Assert.That(frame.SetIntermediateUsed(false), Is.True);
            Assert.That(frame.IntermediateUsed, Is.False);
            Assert.That(frame.IntermediateDirty, Is.True, "drawn again when switched back");
            frame.TakeIntermediate(false);
            frame.IntermediateCreated();
            Assert.That(frame.TakeIntermediate(false), Is.True, "a new texture is empty");
        }
    }
}
