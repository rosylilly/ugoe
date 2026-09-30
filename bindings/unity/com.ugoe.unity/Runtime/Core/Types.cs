// SPDX-License-Identifier: MIT
// C ABI の値を、Unity に依存しない C# の型にしたもの。
using System;

namespace Ugoe
{
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

        public MotionEvent(float time, string value)
        {
            Time = time;
            Value = value;
        }

        public override string ToString() => $"{Value} @ {Time}";
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
