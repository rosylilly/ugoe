// SPDX-License-Identifier: MIT
// UgoeResult が C ABI の結果の値 (crates/ugoe-ffi/include/ugoe.h の UGOE_OK・UGOE_ERROR_*) と揃っているか (T0455)。
// UgoeException が読む補足の値のキーが crates/ugoe-wasm/codes.json と揃っているか (T0458)。
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
        public void FieldKeysMatchTheCodes()
        {
            using var codes = JsonDocument.Parse(File.ReadAllText(Path.Combine(Paths.Root, "crates", "ugoe-wasm", "codes.json")));
            // 失敗の種類 (着せ替えと読み込み) のキー。読み込みの警告 (load_warning) は Puppet.GetLoadWarningField で読む。
            var keys = new[] { "dress_up_error", "load_error" }
                .SelectMany(group => codes.RootElement.GetProperty(group).EnumerateObject())
                .SelectMany(kind => kind.Value.EnumerateArray().Select(k => k.GetString()))
                .Distinct()
                .ToArray();
            Assert.That(keys, Does.Contain("mask").And.Contain("limit"));
            Assert.That(UgoeException.FieldKeys, Is.EquivalentTo(keys),
                "UgoeException.FieldKeys has every field key of dress_up_error and load_error in codes.json and nothing else");
        }

        // NULL_POINTER → NullPointer
        private static string Pascal(string snake) =>
            string.Concat(snake.Split('_').Select(w => w[0] + w.Substring(1).ToLowerInvariant()));
    }
}
