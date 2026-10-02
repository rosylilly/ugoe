// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;

namespace Ugoe.Unity
{
    /// <summary>
    /// マスクの内容 (<see cref="MaskContent"/>) を、入れ子の深さごとに 1 枚の <see cref="RenderTexture"/> (ページ) に並べて描く
    /// (spec/rendering.md の Masks、配置は <see cref="MaskAtlasLayout"/>)。
    /// </summary>
    /// <remarks>
    /// - 各内容は、それを読む要素の外接矩形 (キャンバス座標) を写す。要素は自分のキャンバス座標から内容の位置を求めて読む
    ///   (写像は頂点属性で渡す) ので、Transform・カメラには左右されず、同じページの違う内容を読む要素も 1 回の描画にまとめられる。
    ///   解像度はキャンバスの 1 ピクセルあたり <c>scale</c> 画素 (ページに収まらなければ下げる)。
    /// - 入れ子のマスク (マスク用メッシュ自身がマスクを読む) は、浅いページを読んで深いページに描く。同じフレームのうちに描くので遅れない。
    /// - <see cref="UgoePuppet"/> が評価の後 (LateUpdate) に <see cref="Graphics.ExecuteCommandBuffer"/> で描く。
    ///   頂点・マスクの効き具合が変わらなければ描き直さない。
    /// </remarks>
    internal sealed class UgoeMaskRenderer : IDisposable
    {
        private const int PassCoverR = 0;
        private const int PassCoverG = 1;
        private const int PassMultiplyR = 2;
        private const int SizeStep = 64;

        private static readonly int MainTexId = Shader.PropertyToID("_MainTex");
        private static readonly int MaskTexId = Shader.PropertyToID("_MaskTex");
        private static readonly int MaskRectId = Shader.PropertyToID("_MaskRect");
        private static readonly int MaskParamsId = Shader.PropertyToID("_MaskParams");
        private static readonly int QuadRectId = Shader.PropertyToID("_QuadRect");
        private static readonly int QuadMapId = Shader.PropertyToID("_QuadMap");
        private static readonly int ScratchId = Shader.PropertyToID("_UgoeScratch");

        private readonly MaskPlan plan;
        private readonly Material material;
        private readonly CommandBuffer commands = new CommandBuffer { name = "ugoe masks" };
        private RenderTexture[] pages = new RenderTexture[0];
        private readonly Vector4[] maps;
        private readonly int[] pageOf;
        private readonly CanvasRect[] bounds;
        private readonly int[] depths;
        private readonly MaterialPropertyBlock block = new MaterialPropertyBlock();
        private Mesh quad;

        public UgoeMaskRenderer(MaskPlan plan, Material material)
        {
            this.plan = plan;
            this.material = material;
            maps = new Vector4[plan.Contents.Count];
            pageOf = new int[plan.Contents.Count];
            bounds = new CanvasRect[plan.Contents.Count];
            depths = new int[plan.Contents.Count];
            for (var c = 0; c < depths.Length; c++)
            {
                depths[c] = plan.Contents[c].Depth;
                pageOf[c] = depths[c];
            }
        }

        public bool HasMasks => plan.Contents.Count > 0;

        /// <summary>ページ <paramref name="page"/> のテクスチャ (まだ描いていなければ null)。</summary>
        public RenderTexture GetPage(int page) => page >= 0 && page < pages.Length ? pages[page] : null;

        /// <summary>内容 <paramref name="content"/> を読む写像 (ページの uv = canvas × xy + zw)。</summary>
        public Vector4 GetMap(int content) => maps[content];

        /// <summary>
        /// 要素 <paramref name="drawable"/> が自分のマスクを読むためのシェーダの値 (<c>_MaskTex</c>・<c>_MaskRect</c>・<c>_MaskParams</c>)
        /// を <paramref name="target"/> に書く (マスク用メッシュを内容に描くとき)。マスクを持たなければ読まない設定にする。
        /// </summary>
        public void BindMaskRead(MaterialPropertyBlock target, int drawable, float maskStrength)
        {
            var content = plan.ContentOf[drawable];
            var page = content >= 0 ? GetPage(pageOf[content]) : null;
            if (page == null)
            {
                target.SetVector(MaskParamsId, new Vector4(1, 0, 0, 0));
                target.SetTexture(MaskTexId, Texture2D.whiteTexture);
                return;
            }
            var c = plan.Contents[content];
            target.SetTexture(MaskTexId, page);
            target.SetVector(MaskRectId, maps[content]);
            target.SetVector(MaskParamsId, new Vector4(c.PositiveOffset, c.PositiveScale, Mathf.Clamp01(maskStrength), 1));
        }

        /// <summary>
        /// バッチ (<see cref="DrawBatch"/>) がページ <paramref name="page"/> を読むためのテクスチャを <paramref name="target"/> に書く。
        /// 内容の位置と mask_strength は要素ごとの頂点属性で渡す (<see cref="VertexAttributes"/>)。-1 か、まだ描いていなければ白。
        /// </summary>
        public void BindPageRead(MaterialPropertyBlock target, int page)
        {
            var texture = GetPage(page);
            target.SetTexture(MaskTexId, texture != null ? (Texture)texture : Texture2D.whiteTexture);
        }

        /// <summary>
        /// 要素 <paramref name="drawable"/> の、マスクを読むための頂点属性 (uv3 の zw = 通常マスクの結合値の係数、
        /// uv4 = 内容の写像)。マスクを持たないか、まだ描いていなければ読まない値 (strength は呼び出し側が 0 にする)。
        /// </summary>
        public (Vector2 Positive, Vector4 Map) VertexAttributes(int drawable)
        {
            var content = plan.ContentOf[drawable];
            if (content < 0 || GetPage(pageOf[content]) == null) return (new Vector2(1, 0), new Vector4(0, 0, 0.5f, 0.5f));
            var c = plan.Contents[content];
            return (new Vector2(c.PositiveOffset, c.PositiveScale), maps[content]);
        }

        /// <summary>すべての内容を描く。</summary>
        /// <param name="views">描画可能要素ごとのメッシュとテクスチャ (マスク用メッシュの分だけ使う)。</param>
        /// <param name="sync">直前の評価の頂点位置とマスクの効き具合。</param>
        /// <param name="scale">キャンバスの 1 ピクセルあたりの画素数。</param>
        /// <returns>ページのテクスチャか内容の位置が変わったか (変われば、読む側の値を設定し直す)。</returns>
        public bool Render(IReadOnlyList<UgoePuppet.DrawableView> views, FrameSync sync, float scale)
        {
            if (!HasMasks) return false;
            for (var ci = 0; ci < plan.Contents.Count; ci++)
            {
                var b = CanvasRect.Empty;
                foreach (var reader in plan.Contents[ci].Readers) b = CanvasRect.Union(b, sync.Positions[reader]);
                bounds[ci] = b;
            }
            var layout = MaskAtlasLayout.Build(bounds, depths, scale);
            var changed = false;
            if (pages.Length != layout.PageSizes.Length)
            {
                Array.Resize(ref pages, layout.PageSizes.Length);
                changed = true;
            }
            for (var p = 0; p < pages.Length; p++)
            {
                var size = layout.PageSizes[p];
                var before = pages[p];
                pages[p] = Ensure(pages[p], p, size.Width, size.Height);
                if (pages[p] != before) changed = true;
            }
            for (var ci = 0; ci < plan.Contents.Count; ci++)
            {
                var page = pages[pageOf[ci]];
                var m = layout.Regions[ci].UvMap(page.width, page.height);
                var map = new Vector4(m.X, m.Y, m.Z, m.W);
                if (map != maps[ci]) changed = true;
                maps[ci] = map;
            }

            commands.Clear();
            for (var p = 0; p < pages.Length; p++)
            {
                commands.SetRenderTarget(pages[p]);
                commands.ClearRenderTarget(false, true, Color.white);
                for (var ci = 0; ci < plan.Contents.Count; ci++)
                {
                    if (pageOf[ci] == p) RenderContent(ci, layout.Regions[ci], pages[p], views, sync);
                }
            }
            Graphics.ExecuteCommandBuffer(commands);
            return changed;
        }

        // 内容 1 つを、ページの中の領域 region に描く。
        private void RenderContent(int ci, CanvasRegion region, RenderTexture page, IReadOnlyList<UgoePuppet.DrawableView> views,
            FrameSync sync)
        {
            var content = plan.Contents[ci];
            var viewport = new Rect(region.Px, region.Py, region.Width, region.Height);
            // Ortho の bottom = Y0: 領域の下の端 (v の小さい側) がキャンバスの Y0 になる (読む側の写像と同じ向き)。
            var projection = Matrix4x4.Ortho(region.X0, region.X0 + region.CanvasWidth, region.Y0, region.Y0 + region.CanvasHeight, -1, 1);
            void Target(RenderTargetIdentifier target)
            {
                commands.SetRenderTarget(target);
                commands.SetViewport(viewport);
                commands.SetViewProjectionMatrices(Matrix4x4.identity, projection);
            }
            Target(page);
            if (content.Mode == MaskMode.Intersection && content.Positive.Length > 1)
            {
                // 共通部分: マスクごとに 1 − c_i を作業用のテクスチャ (ページと同じ大きさ・配置) に描き、R に c_i を掛ける。
                commands.GetTemporaryRT(ScratchId, page.width, page.height, 0, FilterMode.Bilinear, RenderTextureFormat.ARGB32);
                foreach (var source in content.Positive)
                {
                    // 作業用のテクスチャは、この内容のためだけに使うので全体を消してよい。
                    commands.SetRenderTarget(ScratchId);
                    commands.ClearRenderTarget(false, true, Color.white);
                    Target(ScratchId);
                    Cover(views[source], source, sync, PassCoverR);
                    Target(page);
                    commands.SetGlobalTexture(ScratchId, ScratchId);
                    block.Clear();
                    block.SetVector(QuadRectId, new Vector4(region.X0, region.Y0, region.CanvasWidth, region.CanvasHeight));
                    block.SetVector(QuadMapId, maps[ci]);
                    commands.DrawMesh(Quad(), Matrix4x4.identity, material, 0, PassMultiplyR, block);
                }
                commands.ReleaseTemporaryRT(ScratchId);
            }
            else
            {
                foreach (var source in content.Positive) Cover(views[source], source, sync, PassCoverR);
            }
            foreach (var source in content.Negative) Cover(views[source], source, sync, PassCoverG);
        }

        // マスク用メッシュ 1 つのカバレッジを描く (自身のマスクを読む。裏向きの三角形も使う)。
        private void Cover(UgoePuppet.DrawableView view, int drawable, FrameSync sync, int pass)
        {
            block.Clear();
            block.SetTexture(MainTexId, view.Texture);
            BindMaskRead(block, drawable, sync.States[drawable].MaskStrength);
            commands.DrawMesh(view.Mesh, Matrix4x4.identity, material, 0, pass, block);
        }

        // 大きさが足りないか、半分より大きく余るときだけ作り直す (動くたびに作り直さないように、64 画素単位に丸める)。
        private static RenderTexture Ensure(RenderTexture texture, int page, int width, int height)
        {
            if (texture != null && texture.IsCreated() && texture.width >= width && texture.height >= height
                && texture.width <= width * 2 + SizeStep && texture.height <= height * 2 + SizeStep)
            {
                return texture;
            }
            Release(texture);
            texture = new RenderTexture(RoundUp(width), RoundUp(height), 0, RenderTextureFormat.ARGB32, RenderTextureReadWrite.Linear)
            {
                name = $"ugoe mask page {page}",
                filterMode = FilterMode.Bilinear,
                wrapMode = TextureWrapMode.Clamp,
                useMipMap = false,
                hideFlags = HideFlags.HideAndDontSave,
            };
            texture.Create();
            return texture;
        }

        private static int RoundUp(int size) => Mathf.Min(MaskAtlasLayout.MaxSize, (size + SizeStep - 1) / SizeStep * SizeStep);

        private Mesh Quad()
        {
            if (quad != null) return quad;
            quad = new Mesh { name = "ugoe mask quad", hideFlags = HideFlags.HideAndDontSave };
            quad.SetVertices(new List<Vector3> { Vector3.zero, Vector3.right, Vector3.up, new Vector3(1, 1, 0) });
            quad.SetUVs(0, new List<Vector2> { Vector2.zero, Vector2.right, Vector2.up, Vector2.one });
            quad.SetIndices(new[] { 0, 1, 2, 2, 1, 3 }, MeshTopology.Triangles, 0);
            quad.bounds = new Bounds(Vector3.zero, Vector3.one * 1e6f);
            return quad;
        }

        private static void Release(RenderTexture texture)
        {
            if (texture == null) return;
            texture.Release();
            if (Application.isPlaying) UnityEngine.Object.Destroy(texture);
            else UnityEngine.Object.DestroyImmediate(texture);
        }

        public void Dispose()
        {
            for (var i = 0; i < pages.Length; i++)
            {
                Release(pages[i]);
                pages[i] = null;
            }
            commands.Release();
            if (quad != null)
            {
                if (Application.isPlaying) UnityEngine.Object.Destroy(quad);
                else UnityEngine.Object.DestroyImmediate(quad);
            }
        }
    }
}
