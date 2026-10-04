// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;

namespace Ugoe.Unity
{
    /// <summary>
    /// Linear の色空間で参照実装と同じ色にするための中間のテクスチャ (README の「色空間」)。
    /// </summary>
    /// <remarks>
    /// <para>まとめたメッシュ (バッチ) を、sRGB の変換をしない <see cref="RenderTexture"/> (<see cref="RenderTextureReadWrite.Linear"/>) に
    /// sRGB の値のまま描く (シェーダのキーワード <c>UGOE_GAMMA_TARGET</c>)。合成も sRGB の値で行われるので、参照実装 (spec/rendering.md) の
    /// 透明な背景への描画と同じ結果になる。それを 1 枚の四角形 (子の GameObject) で画面に写す (<c>Hidden/Ugoe/Composite</c>)。
    /// 写すときにストレートの色を線形に直すので、不透明な画素は画面でも中間のテクスチャと同じ値になる。シーンとの合成
    /// (モデルの輪郭の半透明の画素) だけは、ほかのスプライトと同じく線形で行われる。</para>
    /// <para>テクスチャはキャンバス座標の外接矩形を写し、原点を画素の格子にそろえる (<see cref="CanvasRegion.Fit"/>)。
    /// 解像度はマスクと同じく画面上の大きさに合わせる (倍率 1 なら <c>ugoe render</c> のキャンバスの画素と重なる)。
    /// 描画パイプライン (Built-in・URP) に依存しない (<see cref="Graphics.ExecuteCommandBuffer"/> で LateUpdate に描く)。</para>
    /// </remarks>
    internal sealed class UgoeIntermediate : IDisposable
    {
        private const int SizeStep = 64;
        private static readonly int MainTexId = Shader.PropertyToID("_MainTex");

        private readonly CommandBuffer commands = new CommandBuffer { name = "ugoe intermediate" };
        private readonly MaterialPropertyBlock block = new MaterialPropertyBlock();
        private readonly GameObject gameObject;
        private readonly MeshRenderer renderer;
        private readonly Mesh quad;
        // 写す四角形の頂点 (毎フレーム確保しないよう使い回す)
        private readonly Vector3[] corners = new Vector3[4];
        private RenderTexture texture;

        /// <summary>直前に描いた領域 (キャンバス座標と画素の対応)。</summary>
        public CanvasRegion Region { get; private set; }

        /// <summary>中間のテクスチャ (まだ描いていなければ null)。</summary>
        public RenderTexture Texture => texture;

        /// <param name="parent">写す四角形を置く親 (<see cref="UgoePuppet"/> のオブジェクト)。</param>
        /// <param name="composite">写すための材質 (<c>Hidden/Ugoe/Composite</c>)。</param>
        public UgoeIntermediate(Transform parent, Material composite)
        {
            gameObject = new GameObject("ugoe intermediate") { hideFlags = HideFlags.DontSave | HideFlags.NotEditable };
            gameObject.layer = parent.gameObject.layer;
            gameObject.transform.SetParent(parent, false);
            quad = new Mesh { name = "ugoe intermediate", hideFlags = HideFlags.DontSave };
            quad.MarkDynamic();
            // テクスチャ全体を写す四角形 (uv はキャンバスの向きのまま。v = 0 が上の端)。頂点は Render で動かす。
            quad.vertices = corners;
            quad.uv = new[] { Vector2.zero, Vector2.right, Vector2.up, Vector2.one };
            quad.SetIndices(new[] { 0, 1, 2, 2, 1, 3 }, MeshTopology.Triangles, 0);
            gameObject.AddComponent<MeshFilter>().sharedMesh = quad;
            renderer = gameObject.AddComponent<MeshRenderer>();
            renderer.shadowCastingMode = ShadowCastingMode.Off;
            renderer.receiveShadows = false;
            renderer.lightProbeUsage = LightProbeUsage.Off;
            renderer.reflectionProbeUsage = ReflectionProbeUsage.Off;
            renderer.sharedMaterial = composite;
            renderer.sortingOrder = 0;
        }

        /// <summary>写す四角形を表示するか (中間のテクスチャを使わない間は隠す)。</summary>
        public bool Visible
        {
            get => renderer.enabled;
            set => renderer.enabled = value;
        }

        /// <summary>中間のテクスチャが失われたか (デバイスの作り直しなど。描き直す)。</summary>
        public bool Lost => texture == null || !texture.IsCreated();

        /// <summary>
        /// バッチを中間のテクスチャに描き、写す四角形を合わせる。
        /// </summary>
        /// <param name="bounds">描く要素の頂点の外接矩形 (キャンバス座標)。</param>
        /// <param name="scale">キャンバスの 1 ピクセルあたりの画素数。</param>
        /// <param name="toCanvas">バッチのメッシュの座標 (このオブジェクトの座標) からキャンバス座標への変換。</param>
        /// <param name="canvasToLocal">キャンバス座標からこのオブジェクトの座標への変換 (写す四角形の頂点)。</param>
        /// <param name="draws">描画順のバッチ (メッシュ・材質・プロパティ)。</param>
        public void Render(CanvasRect bounds, float scale, Matrix4x4 toCanvas, Func<Vector2, Vector3> canvasToLocal,
            IEnumerable<(Mesh Mesh, Material Material, MaterialPropertyBlock Block)> draws)
        {
            var region = CanvasRegion.Fit(bounds, scale, MaskAtlasLayout.MaxSize);
            Region = region;
            texture = Ensure(texture, region.Width, region.Height);
            var w = texture.width / region.Scale;
            var h = texture.height / region.Scale;
            // Ortho の bottom = Y0: テクスチャの v = 0 がキャンバスの Y0 (上の端) になる (マスクの内容と同じ向き)。
            var projection = Matrix4x4.Ortho(region.X0, region.X0 + w, region.Y0, region.Y0 + h, -1, 1);
            commands.Clear();
            commands.SetRenderTarget(texture);
            commands.SetViewProjectionMatrices(Matrix4x4.identity, projection);
            commands.ClearRenderTarget(false, true, new Color(0, 0, 0, 0));
            foreach (var d in draws) commands.DrawMesh(d.Mesh, toCanvas, d.Material, 0, 0, d.Block);
            Graphics.ExecuteCommandBuffer(commands);

            // 写す四角形をテクスチャ全体に合わせる (uv と三角形は作ったときのまま)。
            var x0 = region.X0;
            var y0 = region.Y0;
            corners[0] = canvasToLocal(new Vector2(x0, y0));
            corners[1] = canvasToLocal(new Vector2(x0 + w, y0));
            corners[2] = canvasToLocal(new Vector2(x0, y0 + h));
            corners[3] = canvasToLocal(new Vector2(x0 + w, y0 + h));
            quad.vertices = corners;
            quad.RecalculateBounds();
            block.SetTexture(MainTexId, texture);
            renderer.SetPropertyBlock(block);
        }

        // 大きさが足りないか、半分より大きく余るときだけ作り直す (64 画素単位に丸める)。
        private static RenderTexture Ensure(RenderTexture current, int width, int height)
        {
            if (current != null && current.IsCreated() && current.width >= width && current.height >= height
                && current.width <= width * 2 + SizeStep && current.height <= height * 2 + SizeStep)
            {
                return current;
            }
            Release(current);
            // sRGB の変換をしない (書いた値をそのまま持ち、そのまま読む) ので、合成は sRGB の値で行われる。
            var created = new RenderTexture(RoundUp(width), RoundUp(height), 0, RenderTextureFormat.ARGB32, RenderTextureReadWrite.Linear)
            {
                name = "ugoe intermediate",
                filterMode = FilterMode.Bilinear,
                wrapMode = TextureWrapMode.Clamp,
                useMipMap = false,
                hideFlags = HideFlags.HideAndDontSave,
            };
            created.Create();
            return created;
        }

        private static int RoundUp(int size) => Mathf.Min(MaskAtlasLayout.MaxSize, (size + SizeStep - 1) / SizeStep * SizeStep);

        private static void Release(RenderTexture t)
        {
            if (t == null) return;
            t.Release();
            DestroyObject(t);
        }

        private static void DestroyObject(UnityEngine.Object o)
        {
            if (o == null) return;
            if (Application.isPlaying) UnityEngine.Object.Destroy(o);
            else UnityEngine.Object.DestroyImmediate(o);
        }

        public void Dispose()
        {
            Release(texture);
            texture = null;
            commands.Release();
            DestroyObject(quad);
            DestroyObject(gameObject);
        }
    }
}
