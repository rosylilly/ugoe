// SPDX-License-Identifier: MIT
// 適合テストの Unity (bindings/unity の Runtime/Core) の実行器 (tests/conformance/README.md)。
//
//   dotnet run --project tests/conformance/unity -- <plan.json> <出力ディレクトリ>
//
// 計画のケースごとに Ugoe.Puppet を作り、モジュールを付け、パラメータを設定して評価し、
// 結果を <出力ディレクトリ>/<ケース>.json に書く (形は tests/conformance/src/frame.rs)。
// 画像は作らない (描画は Unity のメッシュとシェーダで行うので、Unity が要る)。
//
// - steps が空のケースは、すべての段を止めて Update(0) を 1 回呼ぶ (時間を進めない評価)。
// - steps があるケースは、目パチと呼吸を effects のとおりにし、ほかの段を動かして、その dt で順に呼ぶ。
// - follow があるケース (受け手として進める。T0455) は、Update の代わりに Puppet.Follow(値, リップシンクの音量, 強さ,
//   パーツの不透明度, dt) を順に呼ぶ。
// - before は使わない (同期して評価するので、値を変えてから反映するまでの遅れが無い)。
using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Ugoe.Conformance
{
    internal static class Program
    {
        private static readonly UpdateStage[] Stages =
        {
            UpdateStage.Physics, UpdateStage.EyeBlink, UpdateStage.Breath, UpdateStage.LipSync, UpdateStage.Pose,
        };

        private static int Main(string[] args)
        {
            if (args.Length != 2)
            {
                Console.Error.WriteLine("usage: <plan.json> <output directory>");
                return 2;
            }
            var plan = JsonNode.Parse(File.ReadAllText(args[0]));
            Directory.CreateDirectory(args[1]);
            var cases = plan["cases"].AsArray();
            var failed = 0;
            foreach (var c in cases)
            {
                var name = (string)c["name"];
                try
                {
                    var frame = Run(c);
                    File.WriteAllText(Path.Combine(args[1], name + ".json"), frame.ToJsonString());
                }
                catch (Exception e)
                {
                    failed++;
                    Console.Error.WriteLine($"{name}: {e.Message}");
                }
            }
            Console.WriteLine($"unity: evaluated {cases.Count - failed} cases ({failed} failed)");
            return failed == 0 ? 0 : 1;
        }

        private static JsonObject Run(JsonNode c)
        {
            var model = (string)c["model"];
            using var puppet = (bool)c["bundle"]
                ? Puppet.FromBundle(File.ReadAllBytes(model))
                : Puppet.FromJson(File.ReadAllText(model));
            var modules = new List<Module>();
            try
            {
                foreach (var path in c["modules"].AsArray())
                {
                    var module = Module.FromBytes(File.ReadAllBytes((string)path));
                    modules.Add(module);
                    puppet.AttachModule(module);
                }
                foreach (var (id, value) in c["parameters"].AsObject())
                {
                    if (!puppet.SetParameter(id, (float)value)) throw new ArgumentException($"unknown parameter {id}");
                }
                var steps = c["steps"].AsArray();
                if (steps.Count == 0)
                {
                    foreach (var stage in Stages) puppet.SetStageEnabled(stage, false);
                    puppet.Update(0f);
                }
                else
                {
                    var effects = (bool)c["effects"];
                    puppet.SetStageEnabled(UpdateStage.EyeBlink, effects);
                    puppet.SetStageEnabled(UpdateStage.Breath, effects);
                    var follow = c["follow"]?.AsArray();
                    for (var i = 0; i < steps.Count; i++)
                    {
                        var dt = (float)steps[i];
                        if (follow == null) puppet.Update(dt);
                        else Follow(puppet, c, follow, i, dt);
                    }
                }
                return Frame(puppet);
            }
            finally
            {
                foreach (var m in modules) m.Dispose();
            }
        }

        /// <summary>計画の <c>follow</c>・<c>follow_lip_sync</c>・<c>follow_part_opacities</c> の <paramref name="i"/> 番目で受け手として進める。</summary>
        private static void Follow(Puppet puppet, JsonNode c, JsonArray follow, int i, float dt)
        {
            var frame = follow[i].AsArray();
            var values = new float[frame.Count];
            for (var k = 0; k < values.Length; k++) values[k] = (float)frame[k];
            float? level = null;
            var strength = 0f;
            if (c["follow_lip_sync"]?[i] is JsonArray lipSync)
            {
                level = (float)lipSync[0];
                strength = (float)lipSync[1];
            }
            var parts = new List<PartOpacityInput>();
            if (c["follow_part_opacities"]?[i] is JsonArray opacities)
            {
                foreach (var p in opacities) parts.Add(new PartOpacityInput((int)p[0], (float)p[1], (float)p[2]));
            }
            puppet.Follow(values, level, strength, parts, dt);
        }

        /// <summary>評価の結果 (tests/conformance/src/frame.rs の Frame)。</summary>
        private static JsonObject Frame(Puppet puppet)
        {
            var meshes = new JsonObject();
            var drawables = puppet.Drawables;
            for (var i = 0; i < drawables.Count; i++)
            {
                var info = drawables[i];
                var xy = new float[info.VertexCount * 2];
                puppet.CopyPositions(i, xy);
                var positions = new JsonArray();
                for (var v = 0; v < info.VertexCount; v++) positions.Add(new JsonArray(xy[v * 2], xy[v * 2 + 1]));
                var s = puppet.GetDrawableState(i);
                meshes[info.Id] = new JsonObject
                {
                    ["positions"] = positions,
                    ["opacity"] = s.Opacity,
                    ["draw_order"] = s.DrawOrder,
                    ["multiply_color"] = Color(s.MultiplyR, s.MultiplyG, s.MultiplyB),
                    ["screen_color"] = Color(s.ScreenR, s.ScreenG, s.ScreenB),
                    ["mask_strength"] = s.MaskStrength,
                    ["visible"] = s.Visible,
                };
            }
            var order = new int[drawables.Count];
            puppet.CopyRenderOrder(order);
            var renderOrder = new JsonArray();
            foreach (var i in order) renderOrder.Add(drawables[i].Id);
            return new JsonObject { ["meshes"] = meshes, ["render_order"] = renderOrder };
        }

        private static JsonObject Color(float r, float g, float b) =>
            new JsonObject { ["r"] = r, ["g"] = g, ["b"] = b };
    }
}
