// SPDX-License-Identifier: MIT
// テクスチャのデコード (EncodedTexture.Decode。C ABI の ugoe_image_decode、T0466)。
using System;
using System.IO;
using NUnit.Framework;

namespace Ugoe.Tests
{
    public class ImageTests
    {
        /// <summary>1 段だけの KTX2 (<paramref name="vkFormat"/>・<paramref name="width"/>×<paramref name="height"/>・中身は <paramref name="level"/>)。</summary>
        private static byte[] Ktx2(uint vkFormat, uint width, uint height, byte[] level)
        {
            using var stream = new MemoryStream();
            using var w = new BinaryWriter(stream);
            w.Write(new byte[] { 0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A });
            // vkFormat・typeSize・幅・高さ・奥行き・層・面・段・超圧縮
            foreach (var v in new uint[] { vkFormat, 1, width, height, 0, 0, 1, 1, 0 }) w.Write(v);
            // データ形式の記述・キーと値 (なし)
            foreach (var v in new uint[] { 0, 0, 0, 0 }) w.Write(v);
            // 超圧縮の大域データ (なし)
            w.Write(0UL);
            w.Write(0UL);
            // 段の索引: 位置・長さ・展開後の長さ
            w.Write(80UL + 24);
            w.Write((ulong)level.Length);
            w.Write((ulong)level.Length);
            w.Write(level);
            return stream.ToArray();
        }

        /// <summary>
        /// VK_FORMAT_BC1_RGB_UNORM_BLOCK (131) の 4×4。color0 = 赤 (565 の 0xF800)、color1 = 青 (0x001F)。
        /// 上の行の 4 画素だけ番号 1 (青)、ほかは番号 0 (赤)。
        /// </summary>
        private static byte[] Bc1() => Ktx2(131, 4, 4, new byte[] { 0x00, 0xF8, 0x1F, 0x00, 0b0101_0101, 0, 0, 0 });

        [Test]
        public void DecodesBc1Ktx2TopRowFirst()
        {
            var image = new EncodedTexture(EncodedTextureFormat.Ktx2, Bc1()).Decode(true);
            Assert.That(image.Width, Is.EqualTo(4));
            Assert.That(image.Height, Is.EqualTo(4));
            Assert.That(image.Pixels.Length, Is.EqualTo(4 * 4 * 4));
            Assert.That(image.Pixels[..4], Is.EqualTo(new byte[] { 0, 0, 255, 255 }));
            Assert.That(image.Pixels[16..20], Is.EqualTo(new byte[] { 255, 0, 0, 255 }));

            // Unity の Texture2D (下の行が先) に渡すときは行を入れ替える。
            var flipped = TextureConversion.FlipRows(image.Pixels, image.Width, image.Height);
            Assert.That(flipped[..4], Is.EqualTo(new byte[] { 255, 0, 0, 255 }));
            Assert.That(flipped[48..52], Is.EqualTo(new byte[] { 0, 0, 255, 255 }));
        }

        [Test]
        public void DecodesUncompressedKtx2PremultipliedOrStraight()
        {
            // VK_FORMAT_R8G8B8A8_UNORM (37) の 1×1、半透明の白。
            var texture = new EncodedTexture(EncodedTextureFormat.Ktx2, Ktx2(37, 1, 1, new byte[] { 255, 255, 255, 128 }));
            Assert.That(texture.Decode(true).Pixels, Is.EqualTo(new byte[] { 128, 128, 128, 128 }));
            Assert.That(texture.Decode(false).Pixels, Is.EqualTo(new byte[] { 255, 255, 255, 128 }));
        }

        [Test]
        public void DecodesTheSampleTexturesLikeTheirSize()
        {
            using var puppet = Puppet.FromBundle(Paths.Read("avatar.ugoe"));
            var image = puppet.GetTexture(0).Decode(true);
            Assert.That(image.Width, Is.GreaterThan(0));
            Assert.That(image.Pixels.Length, Is.EqualTo(image.Width * image.Height * 4));
        }

        [Test]
        public void UnreadableImagesHaveKinds()
        {
            // ETC2 (VK_FORMAT_ETC2_R8G8B8A8_UNORM_BLOCK = 151) は読めるがデコードしない (白で代用する)。
            var e = Assert.Throws<UgoeException>(() =>
                new EncodedTexture(EncodedTextureFormat.Ktx2, Ktx2(151, 4, 4, new byte[16])).Decode(true));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format), e.Message);
            Assert.That(e.Kind, Is.EqualTo("unsupported_texture"));

            e = Assert.Throws<UgoeException>(() =>
                new EncodedTexture(EncodedTextureFormat.Png, new byte[] { 1, 2, 3 }).Decode(true));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.Format));
            Assert.That(e.Kind, Is.EqualTo("invalid_texture"));

            e = Assert.Throws<UgoeException>(() => new EncodedTexture(EncodedTextureFormat.None, null).Decode(true));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.InvalidArgument));
        }

        [Test]
        public void ImagesOverTheLimitAreRejected()
        {
            Assert.That(UgoeImageLimits.Default.MaxSide, Is.EqualTo(16384u));
            var limits = UgoeImageLimits.Default;
            limits.MaxSide = 2;
            var e = Assert.Throws<UgoeException>(() =>
                new EncodedTexture(EncodedTextureFormat.Ktx2, Bc1()).Decode(true, limits));
            Assert.That(e.Result, Is.EqualTo(UgoeResult.LimitExceeded), e.Message);
            Assert.That(e.Kind, Is.EqualTo("image_limit_exceeded"));
            Assert.That(e.Fields["width"], Is.EqualTo("4"));
            Assert.That(e.Fields["height"], Is.EqualTo("4"));
            Assert.That(e.Fields["max"], Is.EqualTo("2"));
        }

        [Test]
        public void FlipRowsChecksTheSize()
        {
            Assert.Throws<ArgumentException>(() => TextureConversion.FlipRows(new byte[8], 1, 1));
            Assert.That(TextureConversion.FlipRows(new byte[] { 1, 1, 1, 1, 2, 2, 2, 2 }, 1, 2),
                Is.EqualTo(new byte[] { 2, 2, 2, 2, 1, 1, 1, 1 }));
        }
    }
}
