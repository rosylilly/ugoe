// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;

namespace Ugoe.Unity
{
    /// <summary>
    /// マスクの内容 (<see cref="MaskContent"/>) を、内容ごとの <see cref="RenderTexture"/> に描く (spec/rendering.md の Masks)。
    /// </summary>
    /// <remarks>
    /// - 内容のテクスチャは、それを読む要素の外接矩形 (キャンバス座標) を写す。要素は自分のキャンバス座標で読むので、
    ///   Transform・カメラには左右されない。解像度はキャンバスの 1 ピクセルあたり <c>scale</c> 画素。
    /// - 入れ子のマスク (マスク用メッシュ自身がマスクを読む) は、深さの順に描く。同じフレームのうちに描くので遅れない。
    /// - <see cref="UgoePuppet"/> が評価の後 (LateUpdate) に <see cref="Graphics.ExecuteCommandBuffer"/> で描く。
    ///   頂点・マスクの効き具合が変わらなければ描き直さない。
    /// </remarks>
    internal sealed class UgoeMaskRenderer : IDisposable
    {
        /// <summary>内容のテクスチャの 1 辺の上限 (画素)。超えるときは解像度を下げる。</summary>
        public const int MaxSize = 4096;

        private const int PassCoverR = 0;
        private const int PassCoverG = 1;
        private const int PassMultiplyR = 2;
        private const int SizeStep = 64;

        private static readonly int MainTexId = Shader.PropertyToID("_MainTex");
        private static readonly int MaskTexId = Shader.PropertyToID("_MaskTex");
        private static readonly int MaskRectId = Shader.PropertyToID("_MaskRect");
        private static readonly int MaskParamsId = Shader.PropertyToID("_MaskParams");
        private static readonly int QuadRectId = Shader.PropertyToID("_QuadRect");
        private static readonly int ScratchId = Shader.PropertyToID("_UgoeScratch");

        private readonly MaskPlan plan;
        private readonly Material material;
        private readonly CommandBuffer commands = new CommandBuffer { name = "ugoe masks" };
        private readonly RenderTexture[] textures;
        private readonly Vector4[] rects;
        private readonly MaterialPropertyBlock block = new MaterialPropertyBlock();
        private Mesh quad;

        public UgoeMaskRenderer(MaskPlan plan, Material material)
        {
            this.plan = plan;
            this.material = material;
            textures = new RenderTexture[plan.Contents.Count];
            rects = new Vector4[plan.Contents.Count];
        }

        public bool HasMasks => plan.Contents.Count > 0;

        /// <summary>内容 <paramref name="content"/> のテクスチャ (まだ描いていなければ null)。</summary>
        public RenderTexture GetTexture(int content) => textures[content];

        /// <summary>内容 <paramref name="content"/> の矩形 (xy = 左上のキャンバス座標、zw = 1 / 大きさ)。</summary>
        public Vector4 GetRect(int content) => rects[content];

        /// <summary>
        /// 要素 <paramref name="drawable"/> が自分のマスクを読むためのシェーダの値 (<c>_MaskTex</c>・<c>_MaskRect</c>・<c>_MaskParams</c>)
        /// を <paramref name="target"/> に書く。マスクを持たなければ読まない設定にする。
        /// </summary>
        public void BindMaskRead(MaterialPropertyBlock target, int drawable, float maskStrength)
        {
            var content = plan.ContentOf[drawable];
            if (content < 0 || textures[content] == null)
            {
                target.SetVector(MaskParamsId, new Vector4(1, 0, 0, 0));
                target.SetTexture(MaskTexId, Texture2D.whiteTexture);
                return;
            }
            var c = plan.Contents[content];
            target.SetTexture(MaskTexId, textures[content]);
            target.SetVector(MaskRectId, rects[content]);
            target.SetVector(MaskParamsId, new Vector4(c.PositiveOffset, c.PositiveScale, Mathf.Clamp01(maskStrength), 1));
        }

        /// <summary>すべての内容を描く。</summary>
        /// <param name="views">描画可能要素ごとのメッシュとテクスチャ。</param>
        /// <param name="sync">直前の評価の頂点位置とマスクの効き具合。</param>
        /// <param name="scale">キャンバスの 1 ピクセルあたりの画素数。</param>
        public void Render(IReadOnlyList<UgoePuppet.DrawableView> views, FrameSync sync, float scale)
        {
            if (!HasMasks) return;
            commands.Clear();
            for (var ci = 0; ci < plan.Contents.Count; ci++)
            {
                var content = plan.Contents[ci];
                var bounds = Bounds(content, sync.Positions);
                var effective = Mathf.Min(scale, MaxSize / Mathf.Max(bounds.width, 1e-3f), MaxSize / Mathf.Max(bounds.height, 1e-3f));
                var width = Mathf.Clamp(Mathf.CeilToInt(bounds.width * effective) + 2, 1, MaxSize);
                var height = Mathf.Clamp(Mathf.CeilToInt(bounds.height * effective) + 2, 1, MaxSize);
                var texture = Ensure(ci, width, height);
                // テクスチャ全体が写すキャンバスの矩形 (1 画素の余白を含む)。
                var x0 = bounds.xMin - 1 / effective;
                var y0 = bounds.yMin - 1 / effective;
                var w = texture.width / effective;
                var h = texture.height / effective;
                rects[ci] = new Vector4(x0, y0, 1 / w, 1 / h);

                // Ortho の bottom = y0: テクスチャの v = 0 がキャンバスの y0 になる (読む側の UV と同じ向き)。
                var projection = Matrix4x4.Ortho(x0, x0 + w, y0, y0 + h, -1, 1);
                commands.SetRenderTarget(texture);
                commands.SetViewProjectionMatrices(Matrix4x4.identity, projection);
                commands.ClearRenderTarget(false, true, Color.white);
                if (content.Mode == MaskMode.Intersection && content.Positive.Length > 1)
                {
                    // 共通部分: マスクごとに 1 − c_i を作業用のテクスチャに描き、R に c_i を掛ける。
                    commands.GetTemporaryRT(ScratchId, texture.width, texture.height, 0, FilterMode.Bilinear, RenderTextureFormat.ARGB32);
                    foreach (var source in content.Positive)
                    {
                        commands.SetRenderTarget(ScratchId);
                        commands.SetViewProjectionMatrices(Matrix4x4.identity, projection);
                        commands.ClearRenderTarget(false, true, Color.white);
                        Cover(views[source], source, sync, PassCoverR);
                        commands.SetRenderTarget(texture);
                        commands.SetViewProjectionMatrices(Matrix4x4.identity, projection);
                        commands.SetGlobalTexture(ScratchId, ScratchId);
                        block.Clear();
                        block.SetVector(QuadRectId, new Vector4(x0, y0, w, h));
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
            Graphics.ExecuteCommandBuffer(commands);
        }

        // マスク用メッシュ 1 つのカバレッジを描く (自身のマスクを読む)。
        private void Cover(UgoePuppet.DrawableView view, int drawable, FrameSync sync, int pass)
        {
            block.Clear();
            block.SetTexture(MainTexId, view.Texture);
            BindMaskRead(block, drawable, sync.States[drawable].MaskStrength);
            commands.DrawMesh(view.Mesh, Matrix4x4.identity, material, view.MaskSubmesh, pass, block);
        }

        // 内容を読む要素の頂点の外接矩形 (キャンバス座標)。
        private static Rect Bounds(MaskContent content, float[][] positions)
        {
            float xMin = float.PositiveInfinity, yMin = float.PositiveInfinity;
            float xMax = float.NegativeInfinity, yMax = float.NegativeInfinity;
            foreach (var reader in content.Readers)
            {
                var p = positions[reader];
                for (var i = 0; i + 1 < p.Length; i += 2)
                {
                    xMin = Mathf.Min(xMin, p[i]);
                    xMax = Mathf.Max(xMax, p[i]);
                    yMin = Mathf.Min(yMin, p[i + 1]);
                    yMax = Mathf.Max(yMax, p[i + 1]);
                }
            }
            if (xMin > xMax || yMin > yMax) return new Rect(0, 0, 1, 1);
            return Rect.MinMaxRect(xMin, yMin, xMax, yMax);
        }

        // 大きさが足りないか、半分より大きく余るときだけ作り直す (動くたびに作り直さないように、64 画素単位に丸める)。
        private RenderTexture Ensure(int content, int width, int height)
        {
            var texture = textures[content];
            if (texture != null && texture.width >= width && texture.height >= height
                && texture.width <= width * 2 + SizeStep && texture.height <= height * 2 + SizeStep)
            {
                return texture;
            }
            Release(texture);
            texture = new RenderTexture(RoundUp(width), RoundUp(height), 0, RenderTextureFormat.ARGB32, RenderTextureReadWrite.Linear)
            {
                name = $"ugoe mask {content}",
                filterMode = FilterMode.Bilinear,
                wrapMode = TextureWrapMode.Clamp,
                useMipMap = false,
                hideFlags = HideFlags.HideAndDontSave,
            };
            texture.Create();
            textures[content] = texture;
            return texture;
        }

        private static int RoundUp(int size) => Mathf.Min(MaxSize, (size + SizeStep - 1) / SizeStep * SizeStep);

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
            for (var i = 0; i < textures.Length; i++)
            {
                Release(textures[i]);
                textures[i] = null;
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
