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

        private static string FindRoot()
        {
            var dir = new DirectoryInfo(System.AppContext.BaseDirectory);
            while (dir != null && !File.Exists(Path.Combine(dir.FullName, "AGENTS.md"))) dir = dir.Parent;
            return dir?.FullName ?? throw new DirectoryNotFoundException("repository root not found");
        }
    }
}
