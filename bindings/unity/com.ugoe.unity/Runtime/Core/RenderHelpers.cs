// SPDX-License-Identifier: MIT
// 描画の準備のうち、エンジンに依存しない計算 (docs/spec/rendering.md)。Unity の描画 (Runtime/Unity) から使う。
using System;
using System.Collections.Generic;

namespace Ugoe
{
    /// <summary>テクスチャの変換。</summary>
    public static class TextureConversion
    {
        /// <summary>
        /// 8 ビットのチャンネルに乗算済みアルファを掛ける。参照実装と同じ <c>(c × a + 127) div 255</c>
        /// (docs/spec/rendering.md の Textures)。
        /// </summary>
        public static byte Premultiply(byte c, byte a) => (byte)((c * a + 127) / 255);

        /// <summary>RGBA8 の並び (4 バイトで 1 画素) を、その場で乗算済みアルファにする。</summary>
        public static void PremultiplyRgba32(byte[] rgba)
        {
            for (var i = 0; i + 3 < rgba.Length; i += 4)
            {
                var a = rgba[i + 3];
                if (a == 255) continue;
                rgba[i] = Premultiply(rgba[i], a);
                rgba[i + 1] = Premultiply(rgba[i + 1], a);
                rgba[i + 2] = Premultiply(rgba[i + 2], a);
            }
        }
    }

    /// <summary>裏面カリング (docs/spec/rendering.md の Back-face culling)。</summary>
    public static class Culling
    {
        /// <summary>
        /// 表を向く三角形の頂点番号を <paramref name="output"/> に書き、書いた数を返す。
        /// 向きはキャンバス座標 (Y 下向き) だけで決め、表示の変換 (Unity の Y 上向きへの反転など) には左右されない。
        /// </summary>
        /// <param name="positions">キャンバス座標の x, y の組。</param>
        /// <param name="indices">三角形の頂点番号 (3 つで 1 つ)。</param>
        /// <param name="output">長さは <paramref name="indices"/> 以上。</param>
        public static int FrontFacing(float[] positions, ushort[] indices, int[] output) =>
            FrontFacing(positions, indices, output, 0);

        /// <summary><paramref name="output"/> の <paramref name="start"/> から書く。書いた数を返す。</summary>
        public static int FrontFacing(float[] positions, ushort[] indices, int[] output, int start)
        {
            var n = start;
            for (var t = 0; t + 2 < indices.Length; t += 3)
            {
                int i0 = indices[t], i1 = indices[t + 1], i2 = indices[t + 2];
                float x0 = positions[i0 * 2], y0 = positions[i0 * 2 + 1];
                float x1 = positions[i1 * 2], y1 = positions[i1 * 2 + 1];
                float x2 = positions[i2 * 2], y2 = positions[i2 * 2 + 1];
                var cross = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
                if (cross < 0)
                {
                    output[n++] = i0;
                    output[n++] = i1;
                    output[n++] = i2;
                }
            }
            return n - start;
        }
    }

    /// <summary>
    /// マスクの「内容」1 つ。同じマスクの組 (通常マスクと反転マスクの集合と結合方法) を読む要素は 1 つの内容を共有する。
    /// </summary>
    /// <remarks>
    /// 内容は 2 つのチャンネルに描く (docs/spec/rendering.md の Masks):
    /// <list type="bullet">
    /// <item>R: 和集合なら <c>Π(1 − c)</c> (通常マスク)、共通部分なら <c>Π c</c>。通常マスクが無ければ 1 のまま</item>
    /// <item>G: <c>Π(1 − c)</c> (反転マスク)。反転マスクが無ければ 1 のまま</item>
    /// </list>
    /// 読む側は <c>pos = PositiveOffset + PositiveScale × R</c>、<c>m = pos × G</c>、<c>k = 1 − s × (1 − m)</c> とする。
    /// </remarks>
    public sealed class MaskContent
    {
        /// <summary>通常マスクとして読む描画可能要素 (重複なし、昇順)。</summary>
        public int[] Positive { get; }
        /// <summary>反転マスクとして読む描画可能要素 (重複なし、昇順)。</summary>
        public int[] Negative { get; }
        public MaskMode Mode { get; }
        /// <summary>入れ子の深さ。マスク用メッシュが自身のマスクを持たなければ 0。小さい順に描く。</summary>
        public int Depth { get; internal set; }
        /// <summary>この内容を読む描画可能要素 (昇順。マスク用メッシュ自身のカバレッジにも使う)。</summary>
        public List<int> Readers { get; } = new List<int>();

        /// <summary>R から通常マスクの結合値を得る式のオフセット。</summary>
        public float PositiveOffset => Positive.Length == 0 ? 1 : Mode == MaskMode.Intersection ? 0 : 1;
        /// <summary>R から通常マスクの結合値を得る式の倍率。</summary>
        public float PositiveScale => Positive.Length == 0 ? 0 : Mode == MaskMode.Intersection ? 1 : -1;

        /// <summary>
        /// 内容のテクスチャの値 (R, G) からマスク係数 k を求める。シェーダ (UgoeCommon.hlsl の UgoeMaskFactor) と同じ式。
        /// </summary>
        public float Factor(float r, float g, float maskStrength)
        {
            var pos = PositiveOffset + PositiveScale * r;
            var m = pos * g;
            return 1 - maskStrength * (1 - m);
        }

        internal MaskContent(int[] positive, int[] negative, MaskMode mode)
        {
            Positive = positive;
            Negative = negative;
            Mode = mode;
        }

        internal string Key => string.Join(",", Positive) + "|" + string.Join(",", Negative) + "|" + (int)Mode;
    }

    /// <summary>モデルのマスクの内容と描く順 (描画可能要素の並びが変わらない間は同じ)。</summary>
    public sealed class MaskPlan
    {
        /// <summary>内容 (深さの小さい順)。</summary>
        public IReadOnlyList<MaskContent> Contents { get; }

        /// <summary>描画可能要素ごとの、読む内容の番号 (<see cref="Contents"/> の添字)。マスクを持たなければ -1。</summary>
        public int[] ContentOf { get; }

        /// <summary>描画可能要素ごとの、ほかの要素のマスクとして使われるか。</summary>
        public bool[] IsMaskSource { get; }

        private MaskPlan(List<MaskContent> contents, int[] contentOf, bool[] isMaskSource)
        {
            Contents = contents;
            ContentOf = contentOf;
            IsMaskSource = isMaskSource;
        }

        public static MaskPlan Build(IReadOnlyList<DrawableInfo> drawables)
        {
            var count = drawables.Count;
            var contentOf = new int[count];
            var isSource = new bool[count];
            var contents = new List<MaskContent>();
            var byKey = new Dictionary<string, int>();
            for (var i = 0; i < count; i++)
            {
                contentOf[i] = -1;
                var d = drawables[i];
                if (d.Masks.Length == 0) continue;
                var pos = new SortedSet<int>();
                var neg = new SortedSet<int>();
                foreach (var m in d.Masks)
                {
                    if (m.Drawable < 0 || m.Drawable >= count) continue;
                    (m.Invert ? neg : pos).Add(m.Drawable);
                    isSource[m.Drawable] = true;
                }
                // 通常マスクが 1 つ以下なら、和集合と共通部分は同じ値になる。
                var mode = pos.Count > 1 ? d.MaskMode : MaskMode.Union;
                var content = new MaskContent(ToArray(pos), ToArray(neg), mode);
                if (!byKey.TryGetValue(content.Key, out var index))
                {
                    index = contents.Count;
                    byKey[content.Key] = index;
                    contents.Add(content);
                }
                contents[index].Readers.Add(i);
                contentOf[i] = index;
            }

            // 深さ: マスク用メッシュが読む内容の深さ + 1 の最大 (マスクの参照は循環しない。docs/spec/rendering.md#マスクの参照)。
            var depth = new int[contents.Count];
            var state = new byte[contents.Count]; // 0 = 未計算, 1 = 計算中, 2 = 済み
            int DepthOf(int c)
            {
                if (state[c] == 2) return depth[c];
                if (state[c] == 1) throw new InvalidOperationException("mask references form a cycle");
                state[c] = 1;
                var d = 0;
                foreach (var source in Sources(contents[c]))
                {
                    var own = contentOf[source];
                    if (own >= 0) d = Math.Max(d, DepthOf(own) + 1);
                }
                depth[c] = d;
                state[c] = 2;
                return d;
            }
            for (var c = 0; c < contents.Count; c++) contents[c].Depth = DepthOf(c);

            // 深さの順に並べ替え、番号を振り直す (同じ深さでは最初に現れた順)。
            var order = new List<int>();
            for (var c = 0; c < contents.Count; c++) order.Add(c);
            order.Sort((a, b) => depth[a] != depth[b] ? depth[a].CompareTo(depth[b]) : a.CompareTo(b));
            var remap = new int[contents.Count];
            var sorted = new List<MaskContent>(contents.Count);
            foreach (var c in order)
            {
                remap[c] = sorted.Count;
                sorted.Add(contents[c]);
            }
            for (var i = 0; i < count; i++)
            {
                if (contentOf[i] >= 0) contentOf[i] = remap[contentOf[i]];
            }
            return new MaskPlan(sorted, contentOf, isSource);
        }

        /// <summary>内容が読むマスク用メッシュ (通常と反転の両方)。</summary>
        public static IEnumerable<int> Sources(MaskContent content)
        {
            foreach (var i in content.Positive) yield return i;
            foreach (var i in content.Negative) yield return i;
        }

        private static int[] ToArray(SortedSet<int> set)
        {
            var a = new int[set.Count];
            set.CopyTo(a);
            return a;
        }
    }

    /// <summary>
    /// 毎フレームの評価結果を、変更フラグを見て必要な分だけ取り出す (docs/spec/rendering.md の Skipping unchanged work)。
    /// エンジンは <see cref="PositionsChanged"/> などが立った要素だけを GPU に送り直す。
    /// </summary>
    public sealed class FrameSync
    {
        private readonly Puppet puppet;

        /// <summary>描画可能要素ごとの頂点位置 (キャンバス座標の x, y の組)。変わった要素だけ書き直す。</summary>
        public float[][] Positions { get; }
        public DrawableState[] States { get; }
        /// <summary>直前の <see cref="Pull"/> で頂点位置を取り出したか。</summary>
        public bool[] PositionsChanged { get; }
        /// <summary>奥から手前への描画順。</summary>
        public int[] RenderOrder { get; }
        /// <summary>直前の <see cref="Pull"/> で描画順が変わったか。</summary>
        public bool RenderOrderChanged { get; private set; }
        /// <summary>直前の <see cref="Pull"/> で頂点位置を取り出した要素の数 (転送を省けたかの目安)。</summary>
        public int PositionsCopied { get; private set; }

        /// <summary><paramref name="puppet"/> の今の描画可能要素の並びで作る (着せ替えの後は作り直す)。</summary>
        public FrameSync(Puppet puppet)
        {
            this.puppet = puppet;
            var drawables = puppet.Drawables;
            Positions = new float[drawables.Count][];
            for (var i = 0; i < drawables.Count; i++) Positions[i] = new float[drawables[i].VertexCount * 2];
            States = new DrawableState[drawables.Count];
            PositionsChanged = new bool[drawables.Count];
            RenderOrder = new int[drawables.Count];
        }

        /// <summary><see cref="Puppet.Update"/> の後に呼ぶ。</summary>
        public void Pull()
        {
            PositionsCopied = 0;
            for (var i = 0; i < States.Length; i++)
            {
                var state = puppet.GetDrawableState(i);
                States[i] = state;
                var changed = (state.Changes & DrawableChanges.Positions) != 0;
                PositionsChanged[i] = changed;
                if (changed)
                {
                    puppet.CopyPositions(i, Positions[i]);
                    PositionsCopied++;
                }
            }
            RenderOrderChanged = puppet.RenderOrderChanged;
            if (RenderOrderChanged) puppet.CopyRenderOrder(RenderOrder);
        }
    }
}
