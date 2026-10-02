// SPDX-License-Identifier: MIT
// 描画のバッチング: 描画順で隣り合い、同じ材質で描ける要素を 1 つのメッシュにまとめる。Unity に依存しない。
using System;
using System.Collections.Generic;

namespace Ugoe
{
    /// <summary>
    /// 1 回の描画の呼び出しで描く要素の並び (描画順で連続し、テクスチャ・合成モード・読むマスクのテクスチャが同じ。
    /// docs/spec/rendering.md#描画の最適化)。マスクの内容は入れ子の深さごとに 1 枚のテクスチャ (ページ) にまとめて描く
    /// (<see cref="MaskAtlasLayout"/>) ので、同じページの違う内容を読む要素もまとめる。マスクを読まない要素は、どの要素ともまとめる。
    /// 要素ごとの値 (不透明度・乗算色・スクリーン色・マスクの効き具合・ライティングの種類) は頂点属性で渡す。
    /// 裏面カリングは CPU で三角形を選ぶので、状態の違いにならない。
    /// </summary>
    public sealed class DrawBatch
    {
        /// <summary>まとめた描画可能要素の番号 (奥から手前の順)。三角形もこの順に並べる。</summary>
        public int[] Members { get; }
        /// <summary>要素ごとの、まとめたメッシュの中での最初の頂点の番号 (<see cref="Members"/> と同じ並び)。</summary>
        public int[] VertexOffsets { get; }
        public int VertexCount { get; }
        /// <summary>三角形の頂点番号の数の上限 (すべての要素のすべての三角形)。</summary>
        public int IndexCapacity { get; }
        public int Texture { get; }
        public BlendMode BlendMode { get; }
        /// <summary>読むマスクのページ (<see cref="MaskAtlasLayout.PageOf"/>、= 内容の入れ子の深さ)。どの要素もマスクを読まなければ -1。</summary>
        public int MaskPage { get; }

        internal DrawBatch(List<int> members, IReadOnlyList<DrawableInfo> drawables, BlendMode blend, int maskPage)
        {
            Members = members.ToArray();
            VertexOffsets = new int[Members.Length];
            var vertices = 0;
            var indices = 0;
            for (var k = 0; k < Members.Length; k++)
            {
                VertexOffsets[k] = vertices;
                vertices += drawables[Members[k]].VertexCount;
                indices += drawables[Members[k]].IndexCount;
            }
            VertexCount = vertices;
            IndexCapacity = indices;
            Texture = drawables[Members[0]].Texture;
            BlendMode = blend;
            MaskPage = maskPage;
        }

        /// <summary>
        /// 表示する要素の三角形 (裏面カリングする要素は表を向くものだけ) を、まとめたメッシュの頂点番号で
        /// <paramref name="output"/> (長さは <see cref="IndexCapacity"/> 以上) に書き、書いた数を返す。
        /// </summary>
        /// <param name="drawables">描画可能要素の情報 (カリングするか)。</param>
        /// <param name="indices">要素ごとの三角形の頂点番号。</param>
        /// <param name="positions">要素ごとの頂点位置 (キャンバス座標の x, y の組)。</param>
        /// <param name="states">要素ごとの状態 (表示するか)。</param>
        public int WriteIndices(IReadOnlyList<DrawableInfo> drawables, IReadOnlyList<ushort[]> indices,
            IReadOnlyList<float[]> positions, IReadOnlyList<DrawableState> states, int[] output)
        {
            var n = 0;
            for (var k = 0; k < Members.Length; k++)
            {
                var i = Members[k];
                if (!states[i].Visible) continue;
                var offset = VertexOffsets[k];
                var source = indices[i];
                if (drawables[i].Culling)
                {
                    var start = n;
                    n += Culling.FrontFacing(positions[i], source, output, n);
                    for (var t = start; t < n; t++) output[t] += offset;
                }
                else
                {
                    for (var t = 0; t < source.Length; t++) output[n++] = source[t] + offset;
                }
            }
            return n;
        }
    }

    /// <summary>描画順と合成モードから決めたバッチの並び (奥から手前)。</summary>
    public sealed class DrawBatchPlan
    {
        /// <summary>16 ビットの頂点番号で表せる頂点の数。1 つのバッチはこれを超えない。</summary>
        public const int MaxVertices = 65536;

        /// <summary>バッチ (奥から手前の順)。</summary>
        public IReadOnlyList<DrawBatch> Batches { get; }

        /// <summary>描画可能要素ごとの、入るバッチの番号。色として描かない要素 (マスク専用) は -1。</summary>
        public int[] BatchOf { get; }

        private readonly BlendMode[] blends;

        private DrawBatchPlan(List<DrawBatch> batches, int[] batchOf, BlendMode[] blends)
        {
            Batches = batches;
            BatchOf = batchOf;
            this.blends = blends;
        }

        /// <summary>
        /// バッチを決める。<paramref name="batching"/> が false なら、要素ごとに 1 つのバッチにする。
        /// 描画順 (<paramref name="renderOrder"/>) か合成モードが変わったら作り直す (<see cref="NeedsRebuild"/>)。
        /// </summary>
        /// <remarks>
        /// 表示・不透明度・色・頂点の変化ではバッチを変えない (見えない要素は三角形を出さないだけ)。
        /// </remarks>
        public static DrawBatchPlan Build(IReadOnlyList<DrawableInfo> drawables, IReadOnlyList<DrawableState> states,
            IReadOnlyList<int> renderOrder, MaskPlan masks, bool batching, int maxVertices = MaxVertices)
        {
            var count = drawables.Count;
            var batchOf = new int[count];
            var blends = new BlendMode[count];
            for (var i = 0; i < count; i++)
            {
                batchOf[i] = -1;
                blends[i] = states[i].BlendMode;
            }
            var batches = new List<DrawBatch>();
            var members = new List<int>();
            var vertices = 0;
            var current = -1; // 今のバッチの最初の要素
            var page = -1; // 今のバッチが読むマスクのページ
            void Flush()
            {
                if (members.Count == 0) return;
                foreach (var m in members) batchOf[m] = batches.Count;
                batches.Add(new DrawBatch(members, drawables, blends[current], page));
                members.Clear();
                vertices = 0;
                current = -1;
                page = -1;
            }
            foreach (var i in renderOrder)
            {
                var d = drawables[i];
                if (d.MaskOnly) continue;
                var content = masks.ContentOf[i];
                var own = content >= 0 ? masks.Contents[content].Depth : -1;
                var joins = batching && current >= 0
                    && (own < 0 || page < 0 || own == page)
                    && drawables[current].Texture == d.Texture
                    && blends[current] == blends[i]
                    && vertices + d.VertexCount <= maxVertices;
                if (!joins) Flush();
                if (current < 0) current = i;
                if (own >= 0) page = own;
                members.Add(i);
                vertices += d.VertexCount;
            }
            Flush();
            return new DrawBatchPlan(batches, batchOf, blends);
        }

        /// <summary>合成モードが作ったときから変わった要素があるか (あればバッチを作り直す)。</summary>
        public bool NeedsRebuild(IReadOnlyList<DrawableState> states)
        {
            if (states.Count != blends.Length) return true;
            for (var i = 0; i < blends.Length; i++)
            {
                if (states[i].BlendMode != blends[i]) return true;
            }
            return false;
        }
    }
}
