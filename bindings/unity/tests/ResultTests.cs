// SPDX-License-Identifier: MIT
// UgoeResult が C ABI の結果の値 (crates/ugoe-ffi/include/ugoe.h の UGOE_OK・UGOE_ERROR_*) と揃っているか (T0455)。
// 実際の失敗 (モデルの読み込み・着せ替え) の UgoeException の種類と補足の値が、crates/ugoe-wasm/codes.json と揃っているか
// (T0458・T0464。補足の値は C ABI が列挙するものをすべて読むので、キーの一覧は C# に写さない)。
using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class ResultTests
    {
        [Test]
        public void ResultsMatchTheHeader()
        {
            var header = File.ReadAllText(Path.Combine(Paths.Root, "crates", "ugoe-ffi", "include", "ugoe.h"));
            var defines = Regex.Matches(header, @"^#define UGOE_(OK|ERROR_[A-Z_]+) (\d+)$", RegexOptions.Multiline)
                .Cast<Match>()
                .ToDictionary(m => Pascal(m.Groups[1].Value.Replace("ERROR_", "")), m => int.Parse(m.Groups[2].Value));
            Assert.That(defines, Has.Count.GreaterThan(10));
            var names = Enum.GetNames(typeof(UgoeResult));
            Assert.That(names, Is.EquivalentTo(defines.Keys), "UgoeResult has every UGOE_ERROR_* and nothing else");
            foreach (var (name, value) in defines)
            {
                Assert.That((int)Enum.Parse<UgoeResult>(name), Is.EqualTo(value), name);
            }
        }

        [Test]
        public void BundleLoadFailuresCarryKinds()
        {
            // 壊れたバンドル (チャンクの表が無い。spec/examples/bundles/bundles.json)。
            var e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(File.ReadAllBytes(Paths.Example("bundles", "empty-chunk-header.ugoe"))));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format), e.Message);
            AssertCodes("load_error", e, "invalid_file");
            Assert.That(e.Fields, Is.Empty);

            e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(new byte[] { 1, 2, 3 }));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            AssertCodes("load_error", e, "invalid_file");

            // 知らない必須の機能。
            e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(File.ReadAllBytes(Paths.Example("bundles", "required-feature.ugoe"))));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format), e.Message);
            AssertCodes("load_error", e, "unsupported_features");
            Assert.That(e.Fields["features"], Is.Not.Empty);

            // 新しいスキーマの版 (assets/compat/future。docs/testing.md#互換性テスト)。
            var newer = File.ReadAllBytes(Path.Combine(Paths.Root, "assets", "compat", "future", "newer-schema.ugoe"));
            e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(newer));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format), e.Message);
            AssertCodes("load_error", e, "unsupported_version");
            Assert.That(uint.Parse(e.Fields["found"]), Is.GreaterThan(uint.Parse(e.Fields["supported"])));

            // 読むときの検査を通らないモデルは、結果は Format のまま、種類は invalid_model。
            e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(File.ReadAllBytes(Paths.Example("bundles", "invalid-motion.ugoe"))));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format), e.Message);
            AssertCodes("load_error", e, "invalid_model");
            Assert.That(int.Parse(e.Fields["errors"]), Is.GreaterThan(0));

            // 上限の超過 (Kind と Fields の値は tests/LimitsTests.cs が確かめる)。
            var limits = UgoeReadLimits.Default;
            limits.MaxTotalDecompressed = 1;
            e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(File.ReadAllBytes(Paths.Example("bundles", "single-quad-zstd.ugoe")), limits));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.LimitExceeded), e.Message);
            AssertCodes("load_error", e, "read_limit_exceeded");
        }

        [Test]
        public void JsonLoadFailuresCarryKinds()
        {
            var e = Assert.Throws<UgoeException>(() => Puppet.FromJson("{\"schema_version\": 999999}"));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format), e.Message);
            AssertCodes("load_error", e, "unsupported_version");
            Assert.That(e.Fields["found"], Is.EqualTo("999999"));

            e = Assert.Throws<UgoeException>(() => Puppet.FromJson("{}"));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            AssertCodes("load_error", e, "invalid_file");
        }

        [Test]
        public void UnknownMaskFillsTheMaskField()
        {
            // 見本の LongHair (assets/samples/dress-up) は本体の Skin でマスクする。互換性テストの本体 (assets/compat/dev) の
            // スロット Hair がマスクに使わせるのは Face だけなので、付けると unknown_mask で、新しいキー mask (T0458) が入る。
            using var puppet = Puppet.FromBundle(File.ReadAllBytes(Path.Combine(Paths.Root, "assets", "compat", "dev", "dress-up-base.ugoe")));
            using var hair = Module.FromBytes(Paths.Read("dress-up", "long-hair.ugoemod"));
            var e = Assert.Throws<UgoeException>(() => puppet.AttachModule(hair));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.DressUp), e.Message);
            AssertCodes("dress_up_error", e, "unknown_mask");
            Assert.That(e.Fields["module"], Is.EqualTo(hair.Name));
            Assert.That(e.Fields["mesh"], Is.EqualTo("Back"));
            Assert.That(e.Fields["mask"], Is.EqualTo("Skin"));
            Assert.That(puppet.AttachedModuleNames, Is.Empty);
        }

        /// <summary>
        /// 種類が <paramref name="kind"/> で、補足の値のキーが codes.json のその種類のキーと揃っている (値を持たないキーは無くてよい)。
        /// </summary>
        private static void AssertCodes(string group, UgoeException e, string kind)
        {
            Assert.That(e.Kind, Is.EqualTo(kind), e.Message);
            using var codes = JsonDocument.Parse(File.ReadAllText(Path.Combine(Paths.Root, "crates", "ugoe-wasm", "codes.json")));
            var keys = codes.RootElement.GetProperty(group).GetProperty(kind).EnumerateArray().Select(k => k.GetString()).ToArray();
            Assert.That(e.Fields.Keys, Is.SubsetOf(keys), $"{group}.{kind} in codes.json");
        }

        // NULL_POINTER → NullPointer
        private static string Pascal(string snake) =>
            string.Concat(snake.Split('_').Select(w => w[0] + w.Substring(1).ToLowerInvariant()));
    }
}
