// SPDX-License-Identifier: MIT
using System.IO;

namespace Ugoe.Tests
{
    internal static class Paths
    {
        /// <summary>リポジトリのルート (AGENTS.md のあるフォルダ)。</summary>
        internal static readonly string Root = FindRoot();

        internal static string Sample(params string[] parts) =>
            Path.Combine(Root, "assets", "samples", Path.Combine(parts));

        internal static byte[] Read(params string[] parts) => File.ReadAllBytes(Sample(parts));

        /// <summary>公開仕様の例 (spec/examples/)。期待値は各例の timeline.json から写す。</summary>
        internal static string Example(params string[] parts) =>
            Path.Combine(Root, "spec", "examples", Path.Combine(parts));

        private static string FindRoot()
        {
            var dir = new DirectoryInfo(System.AppContext.BaseDirectory);
            while (dir != null && !File.Exists(Path.Combine(dir.FullName, "AGENTS.md"))) dir = dir.Parent;
            return dir?.FullName ?? throw new DirectoryNotFoundException("repository root not found");
        }
    }
}
