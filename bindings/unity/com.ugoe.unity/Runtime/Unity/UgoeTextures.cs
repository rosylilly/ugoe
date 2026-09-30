// SPDX-License-Identifier: MIT
using System.Collections.Generic;
using UnityEngine;

namespace Ugoe.Unity
{
    /// <summary>
    /// モデルのテクスチャと法線マップを Unity の <see cref="Texture2D"/> にする (spec/rendering.md の Textures)。
    /// </summary>
    /// <remarks>
    /// - 画像は乗算済みアルファにしてから渡す (参照実装と同じ整数の丸め)。バイリニア・端で止める・ミップマップなし。
    /// - 色は sRGB の値をそのまま使う約束なので、sRGB のデコードをしない (<c>linear: true</c>) で作る。
    /// - 法線マップはアルファを掛けずに、sRGB のデコードもしない。
    /// - KTX2 は Unity の標準ではデコードできないので、白 (法線マップなら平ら) で代用して警告を出す。
    /// </remarks>
    internal sealed class UgoeTextures
    {
        private readonly List<Texture2D> colors = new List<Texture2D>();
        private readonly List<Texture2D> normals = new List<Texture2D>();
        private static Texture2D flatNormal;

        public Texture2D GetColor(int index) => index < colors.Count && colors[index] != null ? colors[index] : Texture2D.whiteTexture;

        /// <summary>法線マップ。無ければ null (シェーダは法線 (0, 0, 1) として扱う)。</summary>
        public Texture2D GetNormal(int index) => index < normals.Count ? normals[index] : null;

        /// <summary>法線マップの代わりに束縛する平らな法線 (0.5, 0.5, 1)。</summary>
        public static Texture2D FlatNormal
        {
            get
            {
                if (flatNormal == null)
                {
                    flatNormal = new Texture2D(1, 1, TextureFormat.RGBA32, false, true) { name = "ugoe flat normal", hideFlags = HideFlags.HideAndDontSave };
                    flatNormal.SetPixel(0, 0, new Color32(128, 128, 255, 255));
                    flatNormal.Apply(false, true);
                }
                return flatNormal;
            }
        }

        /// <summary>すべて読み込む (作った直後)。</summary>
        public void LoadAll(Puppet puppet, string owner)
        {
            Clear();
            for (var i = 0; i < puppet.TextureCount; i++) Load(puppet, i, owner);
        }

        /// <summary>着せ替えの後: 変わった番号だけ読み直し、使われなくなった番号を捨てる。</summary>
        public void Refresh(Puppet puppet, string owner)
        {
            var count = puppet.TextureCount;
            foreach (var index in puppet.ChangedTextures)
            {
                if (index < count) Load(puppet, index, owner);
            }
            for (var i = colors.Count - 1; i >= count; i--)
            {
                Destroy(colors[i]);
                Destroy(normals[i]);
                colors.RemoveAt(i);
                normals.RemoveAt(i);
            }
        }

        public void Clear()
        {
            foreach (var t in colors) Destroy(t);
            foreach (var t in normals) Destroy(t);
            colors.Clear();
            normals.Clear();
        }

        private void Load(Puppet puppet, int index, string owner)
        {
            while (colors.Count <= index)
            {
                colors.Add(null);
                normals.Add(null);
            }
            Destroy(colors[index]);
            Destroy(normals[index]);
            colors[index] = Decode(puppet.GetTexture(index), true, $"{owner} texture {index}");
            normals[index] = Decode(puppet.GetNormalMap(index), false, $"{owner} normal map {index}");
        }

        private static Texture2D Decode(EncodedTexture encoded, bool premultiply, string name)
        {
            switch (encoded.Format)
            {
                case EncodedTextureFormat.None:
                    return null;
                case EncodedTextureFormat.Png:
                    break;
                default:
                    Debug.LogWarning($"ugoe: {name} is {encoded.Format}, which this package cannot decode; drawing it as white");
                    return null;
            }
            var texture = new Texture2D(2, 2, TextureFormat.RGBA32, false, true)
            {
                name = name,
                hideFlags = HideFlags.DontSave,
            };
            if (!texture.LoadImage(encoded.Data, false))
            {
                Debug.LogWarning($"ugoe: could not decode {name}");
                Destroy(texture);
                return null;
            }
            if (premultiply)
            {
                var pixels = texture.GetPixels32();
                for (var i = 0; i < pixels.Length; i++)
                {
                    var p = pixels[i];
                    if (p.a == 255) continue;
                    pixels[i] = new Color32(
                        TextureConversion.Premultiply(p.r, p.a),
                        TextureConversion.Premultiply(p.g, p.a),
                        TextureConversion.Premultiply(p.b, p.a),
                        p.a);
                }
                texture.SetPixels32(pixels);
            }
            texture.filterMode = FilterMode.Bilinear;
            texture.wrapMode = TextureWrapMode.Clamp;
            texture.anisoLevel = 0;
            texture.Apply(false, true);
            return texture;
        }

        private static void Destroy(Object o)
        {
            if (o == null) return;
            if (Application.isPlaying) Object.Destroy(o);
            else Object.DestroyImmediate(o);
        }
    }
}
