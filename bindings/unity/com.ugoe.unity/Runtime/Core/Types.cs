// SPDX-License-Identifier: MIT
// C ABI の値を、Unity に依存しない C# の型にしたもの。
using System;
using System.Runtime.InteropServices;

namespace Ugoe
{
    /// <summary>
    /// 読み込みの上限 (C ABI の <c>UgoeReadLimits</c>。docs/spec/file-formats.md#読み込みの上限、T0152)。
    /// 値は <see cref="Default"/> から始めて変え、<see cref="Puppet.FromBundle(byte[], UgoeReadLimits)"/> と
    /// <see cref="Module.FromBytes(byte[], UgoeReadLimits)"/> に渡す。超えたら <see cref="UgoeResult.LimitExceeded"/>
    /// (種類 <c>"read_limit_exceeded"</c>、補足の値 <c>"limit"</c>・<c>"chunk"</c>・<c>"max"</c>)。
    /// 数えるのは圧縮したチャンクを展開したバイトだけ。上限はファイル形式の決まりではなく、読む側の方針。
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct UgoeReadLimits
    {
        /// <summary>圧縮したチャンク 1 つを展開した後の大きさの上限 (バイト)。既定は 256 MiB。</summary>
        public ulong MaxChunkDecompressed;

        /// <summary>1 つのファイルで、圧縮したチャンクを展開したバイトの合計の上限。既定は 1 GiB。</summary>
        public ulong MaxTotalDecompressed;

        /// <summary>既定の上限 (上限を指定しない <see cref="Puppet.FromBundle(byte[])"/> などが使う値)。ネイティブライブラリから読む。</summary>
        public static UgoeReadLimits Default
        {
            get
            {
                UgoeRuntime.EnsureCompatible();
                return Native.ugoe_read_limits_default();
            }
        }
    }

    /// <summary>モデルのキャンバス (ピクセル単位・Y 下向き。docs/spec/model.md)。</summary>
    public readonly struct CanvasInfo
    {
        public readonly float Width;
        public readonly float Height;
        /// <summary>表示時の原点 (足元の中央など)。</summary>
        public readonly float OriginX;
        public readonly float OriginY;

        public CanvasInfo(float width, float height, float originX, float originY)
        {
            Width = width;
            Height = height;
            OriginX = originX;
            OriginY = originY;
        }
    }

    /// <summary>パラメータの範囲と既定値。</summary>
    public readonly struct ParameterInfo
    {
        public readonly string Id;
        public readonly float Min;
        public readonly float Max;
        public readonly float Default;

        public ParameterInfo(string id, float min, float max, float defaultValue)
        {
            Id = id;
            Min = min;
            Max = max;
            Default = defaultValue;
        }
    }

    /// <summary>合成モード (docs/spec/rendering.md の Blend modes)。</summary>
    public enum BlendMode
    {
        Normal = 0,
        Additive = 1,
        Multiply = 2,
    }

    /// <summary>通常マスクの結合方法。</summary>
    public enum MaskMode
    {
        /// <summary>和集合 <c>1 − Π(1 − c)</c>。</summary>
        Union = 0,
        /// <summary>共通部分 <c>Π c</c>。</summary>
        Intersection = 1,
    }

    /// <summary><see cref="DrawableState.Changes"/> のビット (<c>UGOE_CHANGED_*</c>)。</summary>
    [Flags]
    public enum DrawableChanges : byte
    {
        None = 0,
        Positions = 1 << 0,
        Opacity = 1 << 1,
        DrawOrder = 1 << 2,
        Visibility = 1 << 3,
        MultiplyColor = 1 << 4,
        ScreenColor = 1 << 5,
        MaskStrength = 1 << 6,
    }

    /// <summary>マスクの参照 1 つ。</summary>
    public readonly struct MaskRef
    {
        /// <summary>マスクとして使う描画可能要素の番号。</summary>
        public readonly int Drawable;
        /// <summary>true なら反転マスク (その要素の内側を隠す)。</summary>
        public readonly bool Invert;

        public MaskRef(int drawable, bool invert)
        {
            Drawable = drawable;
            Invert = invert;
        }
    }

    /// <summary>描画可能要素の、評価で変わらない情報。着せ替えで描画可能要素の並びが変わったら取り直す。</summary>
    public sealed class DrawableInfo
    {
        public string Id { get; }
        /// <summary>使うテクスチャの番号 (<see cref="Puppet.GetTexture"/>)。</summary>
        public int Texture { get; }
        public int VertexCount { get; }
        public int IndexCount { get; }
        /// <summary>裏向きの三角形を描かないか (docs/spec/rendering.md の Back-face culling)。</summary>
        public bool Culling { get; }
        /// <summary>マスク専用で、色としては描かないか。</summary>
        public bool MaskOnly { get; }
        public MaskMode MaskMode { get; }
        public MaskRef[] Masks { get; }
        /// <summary>ライティングを受けるか (モデルの lighting と、メッシュの lit の両方が true)。</summary>
        public bool Lit { get; }

        public DrawableInfo(string id, int texture, int vertexCount, int indexCount, bool culling, bool maskOnly,
            MaskMode maskMode, MaskRef[] masks, bool lit)
        {
            Id = id;
            Texture = texture;
            VertexCount = vertexCount;
            IndexCount = indexCount;
            Culling = culling;
            MaskOnly = maskOnly;
            MaskMode = maskMode;
            Masks = masks;
            Lit = lit;
        }
    }

    /// <summary>描画可能要素の、直前の <see cref="Puppet.Update"/> で評価した状態。</summary>
    public readonly struct DrawableState
    {
        public readonly float Opacity;
        /// <summary>描画順の値 (大きいほど手前)。並べた結果は <see cref="Puppet.CopyRenderOrder"/>。</summary>
        public readonly float DrawOrder;
        public readonly float MultiplyR, MultiplyG, MultiplyB;
        public readonly float ScreenR, ScreenG, ScreenB;
        /// <summary>知らない合成モードは通常の合成に置き換え済み。</summary>
        public readonly BlendMode BlendMode;
        /// <summary>マスクの効き具合 (0〜1)。</summary>
        public readonly float MaskStrength;
        /// <summary>色として描くか。不透明度 0 やマスク専用の要素は false。</summary>
        public readonly bool Visible;
        /// <summary>前の <see cref="Puppet.Update"/> から変わったもの。作った直後と最初の update の後はすべて立つ。</summary>
        public readonly DrawableChanges Changes;

        internal DrawableState(in Native.DrawableState s)
        {
            Opacity = s.Opacity;
            DrawOrder = s.DrawOrder;
            MultiplyR = s.MultiplyR;
            MultiplyG = s.MultiplyG;
            MultiplyB = s.MultiplyB;
            ScreenR = s.ScreenR;
            ScreenG = s.ScreenG;
            ScreenB = s.ScreenB;
            BlendMode = s.BlendMode <= 2 ? (BlendMode)s.BlendMode : BlendMode.Normal;
            MaskStrength = s.MaskStrength;
            Visible = s.Visible != 0;
            Changes = (DrawableChanges)s.Changes;
        }
    }

    /// <summary>画像の形式 (<c>UGOE_TEXTURE_*</c>)。</summary>
    public enum EncodedTextureFormat
    {
        /// <summary>画像がない (JSON から作ったとき)。白で代用する。法線マップなら法線 (0, 0, 1)。</summary>
        None = 0,
        /// <summary>PNG (乗算していないアルファ)。</summary>
        Png = 1,
        Ktx2 = 2,
    }

    /// <summary>エンコードされたままの画像。デコードはホストが行う。</summary>
    public readonly struct EncodedTexture
    {
        public readonly EncodedTextureFormat Format;
        /// <summary>ファイルの中身 (コピー)。<see cref="EncodedTextureFormat.None"/> なら null。</summary>
        public readonly byte[] Data;

        public EncodedTexture(EncodedTextureFormat format, byte[] data)
        {
            Format = format;
            Data = data;
        }

        /// <summary>既定の上限 (<see cref="UgoeImageLimits.Default"/>) で <see cref="Decode(bool, UgoeImageLimits)"/> する。</summary>
        public DecodedImage Decode(bool premultiply) => Decode(premultiply, UgoeImageLimits.Default);

        /// <summary>
        /// C ABI のデコーダ (<c>ugoe_image_decode</c>。参照の <c>ugoe render</c>・wasm と同じ) で 8bit RGBA にする (T0466)。
        /// PNG と、KTX2 の圧縮していない 8bit の RGB / RGBA と BC1〜BC5・BC7 を読み、参照と同じ画素になる。
        /// </summary>
        /// <param name="premultiply">true なら乗算済みアルファにする (描画するテクスチャ)。法線マップは false。</param>
        /// <param name="limits">デコードの上限。</param>
        /// <exception cref="UgoeException">
        /// 読めない。<see cref="UgoeException.Kind"/> は <c>"unsupported_texture"</c> (読める KTX2 だがデコードしない形式。ETC2・ASTC など)・
        /// <c>"invalid_texture"</c> (壊れている)・<c>"image_limit_exceeded"</c> (<see cref="UgoeResult.LimitExceeded"/>。一辺が上限を超える)。
        /// <see cref="EncodedTextureFormat.None"/> は <see cref="UgoeResult.InvalidArgument"/>。
        /// </exception>
        public DecodedImage Decode(bool premultiply, UgoeImageLimits limits)
        {
            UgoeRuntime.EnsureCompatible();
            var data = Data ?? Array.Empty<byte>();
            UgoeException.Check(Native.ugoe_image_decode((uint)Format, data, (UIntPtr)data.Length, ref limits, premultiply ? (byte)1 : (byte)0, out var image));
            try
            {
                UgoeException.Check(Native.ugoe_image_size(image, out var width, out var height));
                UgoeException.Check(Native.ugoe_image_pixels(image, out var pixels, out var len));
                var copy = new byte[checked((int)len.ToUInt64())];
                if (copy.Length > 0) Marshal.Copy(pixels, copy, 0, copy.Length);
                return new DecodedImage((int)width, (int)height, copy);
            }
            finally
            {
                Native.ugoe_image_destroy(image);
            }
        }
    }

    /// <summary>
    /// 画像のデコードの上限 (C ABI の <c>UgoeImageLimits</c>。docs/spec/file-formats.md#読み込みの上限)。
    /// 値は <see cref="Default"/> から始めて変え、<see cref="EncodedTexture.Decode(bool, UgoeImageLimits)"/> に渡す。
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct UgoeImageLimits
    {
        /// <summary>画像の幅と高さそれぞれの上限 (画素)。既定は 16384。</summary>
        public uint MaxSide;

        /// <summary>既定の上限。ネイティブライブラリから読む。</summary>
        public static UgoeImageLimits Default
        {
            get
            {
                UgoeRuntime.EnsureCompatible();
                return Native.ugoe_image_limits_default();
            }
        }
    }

    /// <summary>デコードした 8bit RGBA の画像 (<see cref="EncodedTexture.Decode(bool)"/>)。</summary>
    public sealed class DecodedImage
    {
        public int Width { get; }
        public int Height { get; }

        /// <summary>
        /// 画素 (1 画素 4 バイトで R・G・B・A、<see cref="Width"/> × <see cref="Height"/> 画素)。**上の行が先**
        /// (Unity の <c>Texture2D</c> は下の行が先なので、<see cref="TextureConversion.FlipRows"/> で並べ替えて渡す)。
        /// </summary>
        public byte[] Pixels { get; }

        public DecodedImage(int width, int height, byte[] pixels)
        {
            Width = width;
            Height = height;
            Pixels = pixels;
        }
    }

    /// <summary>読み込みの警告 (<c>code</c> は wasm の <c>load_warnings_json</c> と同じ)。</summary>
    public sealed class LoadWarning
    {
        /// <summary><c>"texture_missing"</c>・<c>"unknown_optional_feature"</c>・<c>"unknown_value"</c> など。</summary>
        public string Code { get; }
        /// <summary>英語の文。安定した値として扱わない。</summary>
        public string Message { get; }

        public LoadWarning(string code, string message)
        {
            Code = code;
            Message = message;
        }

        public override string ToString() => $"[{Code}] {Message}";
    }

    /// <summary>モーションのイベント (直前の <see cref="Puppet.Update"/> で発火したもの)。</summary>
    public readonly struct MotionEvent
    {
        /// <summary>モーションの中の時刻 (秒)。</summary>
        public readonly float Time;
        public readonly string Value;
        /// <summary>
        /// イベントを発火したモーションの名前 (T0462)。クロスフェードでフェードアウト中のモーションのこともある。
        /// </summary>
        public readonly string Motion;
        /// <summary>
        /// イベントを発火したモーションを始めた番号 (この <see cref="Puppet"/> で開始したモーションに 1 から振る。開始しなかった要求では
        /// 進まない。T0462)。同じ名前のモーションを始め直したものを見分けるのに使う。
        /// </summary>
        public readonly ulong Number;

        public MotionEvent(float time, string value, string motion = null, ulong number = 0)
        {
            Time = time;
            Value = value;
            Motion = motion;
            Number = number;
        }

        public override string ToString() => $"{Value} @ {Time} ({Motion} #{Number})";
    }

    /// <summary>
    /// パーツの不透明度の上書き (直前の <see cref="Puppet.Update"/> がモーションのパーツの不透明度のカーブとポーズから
    /// 書いたもの。docs/spec/motion.md#フレーム内の適用順)。キーフォームと親を掛けた評価後の値ではない
    /// (それは <see cref="Puppet.GetPartEvaluatedOpacity"/>)。
    /// </summary>
    public readonly struct PartOpacity
    {
        public readonly float Value;
        /// <summary>上書きの重み (0〜1)。上書きしていなければ 0。</summary>
        public readonly float Weight;

        public PartOpacity(float value, float weight)
        {
            Value = value;
            Weight = weight;
        }

        public override string ToString() => $"{Value} (weight {Weight})";
    }

    /// <summary>
    /// 指揮役のリップシンクの段が当てた音量と強さ (<c>ugoe::LipSyncInput</c>。<see cref="Puppet.LipSyncApplied"/> が返し、
    /// 受け手が <see cref="Puppet.Follow"/> に渡す。T0454)。
    /// </summary>
    public readonly struct LipSyncInput
    {
        /// <summary>音量 (0〜1)。</summary>
        public readonly float Level;
        /// <summary>エフェクトグループ LipSync の強さ (モーションのカーブ。無ければ 1)。</summary>
        public readonly float Strength;

        public LipSyncInput(float level, float strength)
        {
            Level = level;
            Strength = strength;
        }

        public override string ToString() => $"{Level} (strength {Strength})";
    }

    /// <summary>
    /// 指揮役のモーションのパーツの不透明度のカーブが 1 つのパーツに求めた値と重み (<c>UgoePartOpacityInput</c>。
    /// 受け手が <see cref="Puppet.Follow"/> に渡す)。
    /// </summary>
    public readonly struct PartOpacityInput
    {
        /// <summary>パーツの番号 (<see cref="Puppet.FindPart"/>。指揮役と受け手は同じモデルを使う)。モデルに無い番号は無視される。</summary>
        public readonly int Part;
        /// <summary>カーブの値 (不透明度)。</summary>
        public readonly float Value;
        /// <summary>フェードを含めた重み (0〜1)。</summary>
        public readonly float Weight;

        public PartOpacityInput(int part, float value, float weight)
        {
            Part = part;
            Value = value;
            Weight = weight;
        }
    }

    /// <summary>モーションの優先度 (docs/spec/motion.md#モーションマネージャ)。</summary>
    public enum MotionPriority
    {
        /// <summary>待機。ほかのモーションが再生中なら開始しない。</summary>
        Idle = 0,
        Normal = 1,
        /// <summary>常に開始する。</summary>
        Force = 2,
    }

    /// <summary><see cref="Puppet.Update"/> の段 (<c>UGOE_STAGE_*</c>)。</summary>
    public enum UpdateStage
    {
        Physics = 0,
        EyeBlink = 1,
        Breath = 2,
        LipSync = 3,
        Pose = 4,
    }
}
