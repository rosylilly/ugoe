// SPDX-License-Identifier: MIT
using System;

namespace Ugoe
{
    /// <summary>
    /// 1 フレーム分のトラッキングの入力 (<c>UgoeTrackingInput</c>)。回転は度で、符号はアバタープロファイルの向きの約束
    /// (docs/spec/avatar-profile.md#向きと左右の約束) に合わせる。体と肩は NaN で省略。平滑化はホストが行う。
    /// </summary>
    public sealed class TrackingInput
    {
        /// <summary>ARKit のブレンドシェイプの数。</summary>
        public const int BlendShapeCount = 52;

        /// <summary>頭のヨー (正でキャラクターから見て左を向く。<c>AngleX</c> の向き)。</summary>
        public float HeadYaw;
        /// <summary>頭のピッチ (正で上を向く。<c>AngleY</c>)。</summary>
        public float HeadPitch;
        /// <summary>頭のロール (正で時計回り。<c>AngleZ</c>)。</summary>
        public float HeadRoll;
        public float BodyYaw = float.NaN;
        public float BodyPitch = float.NaN;
        public float BodyRoll = float.NaN;
        /// <summary>左肩の上下 (0 が中立、1 ですくめた状態)。</summary>
        public float ShoulderLeft = float.NaN;
        public float ShoulderRight = float.NaN;

        /// <summary>ブレンドシェイプの値 (0〜1)。添字は <see cref="BlendShapeName"/> の順。</summary>
        public readonly float[] BlendShapes = new float[BlendShapeCount];

        /// <summary>名前 (ARKit、または MediaPipe のカテゴリ名) でブレンドシェイプを設定する。知らない名前なら false。</summary>
        public bool SetBlendShape(string name, float value)
        {
            var index = BlendShapeIndex(name);
            if (index < 0) return false;
            BlendShapes[index] = value;
            return true;
        }

        /// <summary>ブレンドシェイプの名前 (<c>"eyeBlinkLeft"</c> など)。</summary>
        public static string BlendShapeName(int index)
        {
            UgoeException.Check(Native.ugoe_blend_shape_name((uint)index, out var name));
            return Utf8.Decode(name);
        }

        /// <summary>名前からブレンドシェイプの添字を引く。知らない名前なら -1。</summary>
        public static int BlendShapeIndex(string name)
        {
            var r = Native.ugoe_blend_shape_index(Utf8.Encode(name), out var index);
            if (r == (int)UgoeResult.NotFound) return -1;
            UgoeException.Check(r);
            return (int)index;
        }

        internal float[] ToNative()
        {
            var data = new float[Native.TrackingInputFloats];
            data[0] = HeadYaw;
            data[1] = HeadPitch;
            data[2] = HeadRoll;
            data[3] = BodyYaw;
            data[4] = BodyPitch;
            data[5] = BodyRoll;
            data[6] = ShoulderLeft;
            data[7] = ShoulderRight;
            Array.Copy(BlendShapes, 0, data, 8, BlendShapeCount);
            return data;
        }
    }
}
