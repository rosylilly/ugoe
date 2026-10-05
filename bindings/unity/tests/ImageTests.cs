// SPDX-License-Identifier: MIT
// テクスチャのデコード (EncodedTexture.Decode。C ABI の ugoe_image_decode、T0466・T0473)。
using System;
using System.IO;
using System.IO.Compression;
using System.Text;
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

        /// <summary>
        /// 1 枚の PNG (<paramref name="width"/>×<paramref name="height"/>、色の種類 <paramref name="colorType"/>・ビット深度
        /// <paramref name="depth"/>)。<paramref name="rows"/> は各行の画素 (行頭のフィルタの種類は付けない)。
        /// <paramref name="extra"/> は IHDR と IDAT の間に置くチャンク (種類と中身)。
        /// </summary>
        private static byte[] Png(uint width, uint height, byte colorType, byte depth, byte[][] rows, params (string Type, byte[] Data)[] extra)
        {
            using var stream = new MemoryStream();
            stream.Write(new byte[] { 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A });
            var ihdr = new byte[13];
            BigEndian(ihdr, 0, width);
            BigEndian(ihdr, 4, height);
            ihdr[8] = depth;
            ihdr[9] = colorType;
            Chunk(stream, "IHDR", ihdr);
            foreach (var (type, data) in extra) Chunk(stream, type, data);
            using var raw = new MemoryStream();
            using (var z = new ZLibStream(raw, CompressionLevel.Optimal, true))
            {
                foreach (var row in rows)
                {
                    z.WriteByte(0);
                    z.Write(row);
                }
            }
            Chunk(stream, "IDAT", raw.ToArray());
            Chunk(stream, "IEND", Array.Empty<byte>());
            return stream.ToArray();
        }

        private static void Chunk(Stream stream, string type, byte[] data)
        {
            var header = new byte[4];
            BigEndian(header, 0, (uint)data.Length);
            stream.Write(header);
            var body = new byte[4 + data.Length];
            Encoding.ASCII.GetBytes(type).CopyTo(body, 0);
            data.CopyTo(body, 4);
            stream.Write(body);
            BigEndian(header, 0, Crc32(body));
            stream.Write(header);
        }

        private static void BigEndian(byte[] buffer, int offset, uint value)
        {
            buffer[offset] = (byte)(value >> 24);
            buffer[offset + 1] = (byte)(value >> 16);
            buffer[offset + 2] = (byte)(value >> 8);
            buffer[offset + 3] = (byte)value;
        }

        private static uint Crc32(byte[] data)
        {
            var crc = 0xFFFFFFFFu;
            foreach (var b in data)
            {
                crc ^= b;
                for (var k = 0; k < 8; k++) crc = (crc & 1) != 0 ? 0xEDB88320u ^ (crc >> 1) : crc >> 1;
            }
            return ~crc;
        }

        /// <summary>
        /// PNG も C ABI でデコードする (T0473)。上の行が先で、乗算は C ABI の側 (参照と同じ <c>(c × a + 127) div 255</c>)。
        /// </summary>
        [Test]
        public void DecodesRgbaPngTopRowFirst()
        {
            // 1×2 の RGBA8: 上が半透明の白、下が不透明の赤。
            var png = new EncodedTexture(EncodedTextureFormat.Png,
                Png(1, 2, 6, 8, new[] { new byte[] { 255, 255, 255, 128 }, new byte[] { 255, 0, 0, 255 } }));
            var image = png.Decode(true);
            Assert.That((image.Width, image.Height), Is.EqualTo((1, 2)));
            Assert.That(image.Pixels, Is.EqualTo(new byte[] { 128, 128, 128, 128, 255, 0, 0, 255 }));
            Assert.That(png.Decode(false).Pixels, Is.EqualTo(new byte[] { 255, 255, 255, 128, 255, 0, 0, 255 }));
            // Unity の Texture2D に渡すときは KTX2 と同じく行を入れ替える。
            Assert.That(TextureConversion.FlipRows(image.Pixels, image.Width, image.Height),
                Is.EqualTo(new byte[] { 255, 0, 0, 255, 128, 128, 128, 128 }));
        }

        /// <summary>
        /// 16 ビットは上位の 8 ビットを使い、ガンマのチャンク (gAMA) は値を変えない (参照の ugoe_format::image と同じ。
        /// ugoe-format の sixteen_bit_is_reduced_to_eight_bit と同じ画素)。
        /// </summary>
        [Test]
        public void SixteenBitPngUsesTheHighByteAndIgnoresGamma()
        {
            var gamma = new byte[4];
            BigEndian(gamma, 0, 100000); // gAMA = 1.0 (補正するデコーダなら値が変わる)
            var bytes = Png(1, 1, 6, 16, new[] { new byte[] { 0x12, 0x34, 0xab, 0xcd, 0xff, 0x00, 0x80, 0x01 } }, ("gAMA", gamma));
            var png = new EncodedTexture(EncodedTextureFormat.Png, bytes);
            Assert.That(png.Decode(false).Pixels, Is.EqualTo(new byte[] { 0x12, 0xab, 0xff, 0x80 }));
            Assert.That(png.Decode(true).Pixels, Is.EqualTo(new byte[] {
                TextureConversion.Premultiply(0x12, 0x80),
                TextureConversion.Premultiply(0xab, 0x80),
                TextureConversion.Premultiply(0xff, 0x80),
                0x80,
            }));
        }

        /// <summary>パレット (PLTE と tRNS) とグレースケールは RGBA8 に展開する。</summary>
        [Test]
        public void PaletteAndGrayscalePngAreExpanded()
        {
            var palette = Png(3, 1, 3, 8, new[] { new byte[] { 0, 1, 2 } },
                ("PLTE", new byte[] { 10, 20, 30, 40, 50, 60, 255, 0, 0 }),
                ("tRNS", new byte[] { 255, 0 }));
            // tRNS の無い番号 (2) は不透明。番号 1 は透明なので、乗算すると黒になる。
            Assert.That(new EncodedTexture(EncodedTextureFormat.Png, palette).Decode(false).Pixels,
                Is.EqualTo(new byte[] { 10, 20, 30, 255, 40, 50, 60, 0, 255, 0, 0, 255 }));
            Assert.That(new EncodedTexture(EncodedTextureFormat.Png, palette).Decode(true).Pixels,
                Is.EqualTo(new byte[] { 10, 20, 30, 255, 0, 0, 0, 0, 255, 0, 0, 255 }));

            var grayAlpha = Png(2, 1, 4, 8, new[] { new byte[] { 200, 255, 100, 50 } });
            Assert.That(new EncodedTexture(EncodedTextureFormat.Png, grayAlpha).Decode(false).Pixels,
                Is.EqualTo(new byte[] { 200, 200, 200, 255, 100, 100, 100, 50 }));
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
