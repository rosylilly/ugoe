// SPDX-License-Identifier: MIT
// 読み込みの上限 (UgoeReadLimits。C ABI の ugoe_*_with_limits、T0152・T0458)。
using System.IO;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class LimitsTests
    {
        [Test]
        public void DefaultMatchesTheNativeLibrary()
        {
            var limits = UgoeReadLimits.Default;
            Assert.That(limits.MaxChunkDecompressed, Is.EqualTo(256UL * 1024 * 1024));
            Assert.That(limits.MaxTotalDecompressed, Is.EqualTo(1024UL * 1024 * 1024));
        }

        [Test]
        public void BundlesOverTheLimitsAreRejected()
        {
            // 見本 (assets/samples) は圧縮していないので、チャンクを zstd で圧縮した公開仕様の例を使う。
            var bytes = File.ReadAllBytes(Paths.Example("bundles", "single-quad-zstd.ugoe"));
            var total = UgoeReadLimits.Default;
            total.MaxTotalDecompressed = 1;
            var e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(bytes, total));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.LimitExceeded), e.Message);
            Assert.That(e.Kind, Is.EqualTo("read_limit_exceeded"));
            Assert.That(e.Fields["limit"], Is.EqualTo("total_decompressed"));
            Assert.That(e.Fields["max"], Is.EqualTo("1"));
            Assert.That(e.Fields["chunk"], Is.Not.Empty);

            // 1 チャンクの上限も同じ種類で、"limit" で見分ける。
            var chunk = UgoeReadLimits.Default;
            chunk.MaxChunkDecompressed = 1;
            e = Assert.Throws<UgoeException>(() => Puppet.FromBundle(bytes, chunk));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.LimitExceeded));
            Assert.That(e.Fields["limit"], Is.EqualTo("chunk_decompressed"));

            // 既定値なら、上限を指定しないときと同じに読める。
            using var puppet = Puppet.FromBundle(bytes, UgoeReadLimits.Default);
            using var plain = Puppet.FromBundle(bytes);
            Assert.That(puppet.Drawables.Count, Is.EqualTo(plain.Drawables.Count));
        }

        [Test]
        public void ModulesOverTheLimitsAreRejected()
        {
            // spec/examples/bundles/short-hair.ugoemod を `ugoe build --compress` で書き直したもの (tests/Fixtures/README.md)。
            var bytes = File.ReadAllBytes(Path.Combine(Paths.Root, "bindings", "unity", "tests", "Fixtures", "short-hair-zstd.ugoemod"));
            var small = UgoeReadLimits.Default;
            small.MaxTotalDecompressed = 1;
            var e = Assert.Throws<UgoeException>(() => Module.FromBytes(bytes, small));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.LimitExceeded), e.Message);
            Assert.That(e.Kind, Is.EqualTo("read_limit_exceeded"));
            Assert.That(e.Fields["limit"], Is.EqualTo("total_decompressed"));

            // 上限の外の失敗は今までどおり Format と "invalid_file"。
            e = Assert.Throws<UgoeException>(() => Module.FromBytes(new byte[] { 1, 2, 3 }, small));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            Assert.That(e.Kind, Is.EqualTo("invalid_file"));

            using var module = Module.FromBytes(bytes, UgoeReadLimits.Default);
            Assert.That(module.Name, Is.Not.Empty);
        }
    }
}
