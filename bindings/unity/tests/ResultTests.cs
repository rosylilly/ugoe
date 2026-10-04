// SPDX-License-Identifier: MIT
// UgoeResult が C ABI の結果の値 (crates/ugoe-ffi/include/ugoe.h の UGOE_OK・UGOE_ERROR_*) と揃っているか (T0455)。
using System;
using System.IO;
using System.Linq;
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

        // NULL_POINTER → NullPointer
        private static string Pascal(string snake) =>
            string.Concat(snake.Split('_').Select(w => w[0] + w.Substring(1).ToLowerInvariant()));
    }
}
