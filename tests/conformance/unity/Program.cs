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
// - conduct があるケース (T0454) は、同じモデルで指揮役の Puppet をもう 1 つ作って Update で進め、指揮役の口
//   (ValuesBeforePhysics・LipSyncApplied・PartOpacitiesApplied) で読んだ値で受け手を Follow で進める。
// - motions があるケース (T0462) は、その番号の Update の直前に StartMotion で始め、各 Update の後に発火したイベントと
//   出どころ (MotionEvents の Value・Time・Motion・Number) を結果の motion_events に書く。
// - before は使わない (同期して評価するので、値を変えてから反映するまでの遅れが無い)。
// - テクスチャを持つバンドルのケース (KTX2 は T0466、PNG は T0473) は、すべてのテクスチャと法線マップを EncodedTexture.Decode
//   (C ABI の ugoe_image_decode。Unity の UgoeTextures がテクスチャを読む経路) でデコードし、結果を textures に書く。
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

        /// <summary>計画のケースのモデルから Puppet を作り、モジュールを付け、パラメータを設定する。</summary>
        private static Puppet Load(JsonNode c, List<Module> modules)
        {
            var model = (string)c["model"];
            var puppet = (bool)c["bundle"]
                ? Puppet.FromBundle(File.ReadAllBytes(model))
                : Puppet.FromJson(File.ReadAllText(model));
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
                return puppet;
            }
            catch
            {
                puppet.Dispose();
                throw;
            }
        }

        private static JsonObject Run(JsonNode c)
        {
            var modules = new List<Module>();
            var events = new JsonArray();
            Puppet conductor = null;
            try
            {
                using var puppet = Load(c, modules);
                var conduct = c["conduct"];
                if (conduct != null) conductor = Conductor(c, conduct, modules);
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
                    var motions = c["motions"]?.AsArray();
                    for (var i = 0; i < steps.Count; i++)
                    {
                        var dt = (float)steps[i];
                        if (conductor != null) Lead(conductor, puppet, conduct, i, dt);
                        else if (follow != null) Follow(puppet, c, follow, i, dt);
                        else if (motions == null) puppet.Update(dt);
                        else UpdateWithMotions(puppet, motions, events, i, dt);
                    }
                }
                var frame = Frame(puppet);
                if (c["motions"] != null) frame["motion_events"] = events;
                var textures = Textures(puppet);
                if (textures != null) frame["textures"] = textures;
                return frame;
            }
            finally
            {
                conductor?.Dispose();
                foreach (var m in modules) m.Dispose();
            }
        }

        /// <summary>計画の <c>conduct</c> の指揮役を作る (T0454): 効果の設定を書き、物理の段を止め、目パチと呼吸は効果の設定があるときだけ動かし、モーションを再生する。</summary>
        private static Puppet Conductor(JsonNode c, JsonNode conduct, List<Module> modules)
        {
            var conductor = Load(c, modules);
            var effects = (string)conduct["effects"];
            if (effects != null) conductor.SetEffectsJson(effects);
            conductor.SetStageEnabled(UpdateStage.Physics, false);
            conductor.SetStageEnabled(UpdateStage.EyeBlink, effects != null);
            conductor.SetStageEnabled(UpdateStage.Breath, effects != null);
            var motion = (string)conduct["motion"];
            if (motion != null && !conductor.StartMotion(motion, MotionPriority.Normal))
            {
                conductor.Dispose();
                throw new InvalidOperationException($"motion {motion} did not start");
            }
            return conductor;
        }

        /// <summary>指揮役を <paramref name="dt"/> 秒進め、指揮役の口で読んだ値で受け手を進める (T0454)。</summary>
        private static void Lead(Puppet conductor, Puppet puppet, JsonNode conduct, int i, float dt)
        {
            if (conduct["lip_sync"] is JsonArray levels && i < levels.Count) conductor.SetLipSyncLevel((float)levels[i]);
            conductor.Update(dt);
            var lipSync = conductor.LipSyncApplied();
            puppet.Follow(conductor.ValuesBeforePhysics(), lipSync?.Level, lipSync?.Strength ?? 0, conductor.PartOpacitiesApplied(), dt);
        }

        /// <summary>
        /// 計画の <c>motions</c> のうち <paramref name="i"/> 番目の Update の直前のものを始めて進め、発火したイベントと出どころを
        /// <paramref name="events"/> に足す (T0462)。
        /// </summary>
        private static void UpdateWithMotions(Puppet puppet, JsonArray motions, JsonArray events, int i, float dt)
        {
            foreach (var m in motions)
            {
                if ((int)m["step"] != i) continue;
                var name = (string)m["name"];
                var priority = (string)m["priority"] switch
                {
                    "idle" => MotionPriority.Idle,
                    "force" => MotionPriority.Force,
                    _ => MotionPriority.Normal,
                };
                var started = puppet.StartMotion(name, priority);
                if (started != (bool)m["accepted"])
                    throw new InvalidOperationException($"starting {name} at step {i} gave {started}");
            }
            puppet.Update(dt);
            foreach (var e in puppet.MotionEvents())
            {
                events.Add(new JsonObject
                {
                    ["step"] = i,
                    ["value"] = e.Value,
                    ["time"] = e.Time,
                    ["motion"] = e.Motion,
                    ["number"] = e.Number,
                });
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

        /// <summary>
        /// テクスチャの番号ごとのデコードの結果 (tests/conformance/src/frame.rs の TextureFrame)。PNG も KTX2 も (T0473)。テクスチャが
        /// 無ければ null (参照も書かない)。描画するテクスチャは乗算済み、法線マップはアルファを掛けずにデコードする (UgoeTextures と同じ)。
        /// </summary>
        private static JsonArray Textures(Puppet puppet)
        {
            var count = puppet.TextureCount;
            var textures = new List<(EncodedTexture Color, EncodedTexture Normal)>();
            var any = false;
            for (var i = 0; i < count; i++)
            {
                var color = puppet.GetTexture(i);
                var normal = puppet.GetNormalMap(i);
                any |= color.Format != EncodedTextureFormat.None || normal.Format != EncodedTextureFormat.None;
                textures.Add((color, normal));
            }
            if (!any) return null;
            var result = new JsonArray();
            foreach (var (color, normal) in textures)
            {
                result.Add(new JsonObject
                {
                    ["color"] = Digest(color, true),
                    ["normal_map"] = Digest(normal, false),
                });
            }
            return result;
        }

        /// <summary>デコードした画像の <c>"幅x高さ:画素の FNV-1a 64"</c>、読めなければ種類 (frame.rs の image_digest)。画像が無ければ null。</summary>
        private static string Digest(EncodedTexture texture, bool premultiply)
        {
            if (texture.Format == EncodedTextureFormat.None) return null;
            DecodedImage image;
            try
            {
                image = texture.Decode(premultiply);
            }
            catch (UgoeException e)
            {
                return e.Kind ?? e.Result.ToString();
            }
            var hash = 0xcbf29ce484222325UL;
            foreach (var b in image.Pixels)
            {
                hash ^= b;
                hash = unchecked(hash * 0x100000001b3UL);
            }
            return $"{image.Width}x{image.Height}:{hash:x16}";
        }

        private static JsonObject Color(float r, float g, float b) =>
            new JsonObject { ["r"] = r, ["g"] = g, ["b"] = b };
    }
}
