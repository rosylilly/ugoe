// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;

namespace Ugoe.Unity
{
    /// <summary>光源 1 つ (インスペクタで編集する形)。座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (z、手前向き)。</summary>
    [Serializable]
    public struct UgoeLight
    {
        public LightKind kind;
        /// <summary>平行光源では光が進む向き、点光源では位置。</summary>
        public Vector3 vector;
        [ColorUsage(false, true)] public Color color;
        /// <summary>点光源の、明るさが 0 になる距離。0 以下なら減衰しない。</summary>
        public float range;

        public LightSource ToSource() => new LightSource(kind, vector.x, vector.y, vector.z, color.r, color.g, color.b,
            range > 0 ? range : float.PositiveInfinity);
    }

    /// <summary>Linear の色空間での描き方 (README の「色空間」)。Gamma の色空間ではどちらも画面に直接描く。</summary>
    public enum LinearCompositing
    {
        /// <summary>
        /// 中間のテクスチャに sRGB の値のまま合成してから画面に写す。モデルの中の合成は参照実装と一致する
        /// (シーンとの合成だけが線形で行われる)。
        /// </summary>
        Intermediate,
        /// <summary>画面に直接描く (描画の呼び出しが 1 つ少ない)。半透明の部分の合成が線形で行われ、参照実装と少しずれる。</summary>
        Direct,
    }

    /// <summary>シーンに保存するパラメータの値。</summary>
    [Serializable]
    public struct UgoeParameterValue
    {
        public string id;
        public float value;
    }

    /// <summary>
    /// <c>.ugoe</c> のモデルを表示し、動かすコンポーネント (portability.md の段階 B: 評価は ugoe_ffi、描画は Unity)。
    /// </summary>
    /// <remarks>
    /// <para>描画順で隣り合い、同じテクスチャ・合成モード・マスクの内容で描ける要素を 1 つのメッシュにまとめ
    /// (<see cref="DrawBatchPlan"/>、docs/spec/rendering.md#描画の最適化)、まとめたものごとに子の GameObject
    /// (<see cref="MeshRenderer"/>) を作る。描画順は <c>sortingOrder</c> で決める (このオブジェクトの <see cref="SortingGroup"/> の
    /// 中で並ぶので、ほかの 2D の物とは 1 つのまとまりとして並ぶ)。子は保存しない (<see cref="HideFlags.DontSave"/>)。</para>
    /// <para>キャンバスの原点がこのオブジェクトの原点に来る。キャンバスの 1 ピクセルは <see cref="pixelsPerUnit"/> 分の 1 ユニットで、
    /// Y は上向きに直す。</para>
    /// <para>評価と転送は LateUpdate で行う (ほかのスクリプトが Update で設定したパラメータがそのフレームに効く)。
    /// 頂点・色・描画順は変更フラグが立った要素だけ送り直す。</para>
    /// </remarks>
    [ExecuteAlways]
    [DisallowMultipleComponent]
    [AddComponentMenu("ugoe/Ugoe Puppet")]
    [RequireComponent(typeof(SortingGroup))]
    public sealed class UgoePuppet : MonoBehaviour
    {
        [Tooltip("表示するモデル (.ugoe を取り込んだもの)")]
        [SerializeField] private UgoeModel model;
        [Tooltip("読み込むときに付ける着せ替えのモジュール (合成の順)")]
        [SerializeField] private List<UgoeModule> modules = new List<UgoeModule>();
        [Tooltip("時間を進めるか。止めている間もパラメータの変更は描き直す")]
        public bool playing = true;
        [Tooltip("時間の速さ")]
        public float speed = 1;
        [Tooltip("エディタで再生していない間も時間を進めるか")]
        public bool playInEditor;
        [Tooltip("1 ユニットあたりのキャンバスのピクセル数")]
        [SerializeField] private float pixelsPerUnit = 100;
        [Tooltip("描画順で隣り合い、同じ材質で描ける要素を 1 つのメッシュにまとめるか (テクスチャをアトラスにまとめたモデルで効く)")]
        public bool batching = true;
        [Tooltip("Linear の色空間での描き方。Intermediate は中間のテクスチャに合成して参照実装と同じ色にする")]
        public LinearCompositing linearCompositing = LinearCompositing.Intermediate;
        [Tooltip("マスクと中間のテクスチャの解像度の倍率。1 で画面上の大きさと同じ")]
        [Range(0.125f, 4)] public float maskScale = 1;
        [Tooltip("マスクの解像度を画面上の大きさに合わせるカメラ (空なら Camera.main)")]
        public Camera maskCamera;
        [Tooltip("環境光 (ライティングを受けるモデルだけ)")]
        [ColorUsage(false, true)] [SerializeField] private Color ambientLight = Color.white;
        [Tooltip("光源 (先頭の 8 個まで)")]
        [SerializeField] private List<UgoeLight> lights = new List<UgoeLight>();
        [Tooltip("読み込むときに設定するパラメータの値")]
        [SerializeField] private List<UgoeParameterValue> parameters = new List<UgoeParameterValue>();

        /// <summary>読み込んだ (読み直した) ときに呼ぶ。</summary>
        public event Action ModelLoaded;

        /// <summary>モーションのイベントが発火したときに呼ぶ (評価の直後、発火した順)。</summary>
        public event Action<MotionEvent> MotionEventFired;

        private Puppet puppet;
        private FrameSync sync;
        private MaskPlan plan;
        private UgoeMaskRenderer masks;
        private readonly UgoeTextures textures = new UgoeTextures();
        private readonly List<DrawableView> views = new List<DrawableView>();
        private readonly List<BatchView> batchViews = new List<BatchView>();
        private ushort[][] indexLists;
        private DrawBatchPlan batchPlan;
        private bool batchingUsed;
        private Material[] blendMaterials;
        // 中間のテクスチャに描く材質 (UGOE_GAMMA_TARGET。合成モードごと)
        private Material[] gammaMaterials;
        private Material maskMaterial;
        private Material compositeMaterial;
        private UgoeIntermediate intermediate;
        // 中間のテクスチャに描くバッチ (毎フレーム確保しないよう使い回す)
        private readonly List<(Mesh, Material, MaterialPropertyBlock)> intermediateDraws = new List<(Mesh, Material, MaterialPropertyBlock)>();
        private bool intermediateDirty;
        private bool intermediateUsed;
        private bool dirty;
        private bool needsLoad = true;
        private bool validated;
        // 読み込んだときのモデル・モジュール・pixelsPerUnit (インスペクタでの変更で読み直すかを決める)
        private readonly LoadedSource<UgoeModel, UgoeModule> loaded = new LoadedSource<UgoeModel, UgoeModule>();
        private bool lightsDirty = true;
        private float lastMaskScale;
        private float lastResolution = 1;

        private static readonly int MainTexId = Shader.PropertyToID("_MainTex");
        private static readonly int NormalTexId = Shader.PropertyToID("_NormalTex");
        private static readonly int AmbientId = Shader.PropertyToID("_UgoeAmbient");
        private static readonly int LightCountId = Shader.PropertyToID("_UgoeLightCount");
        private static readonly int LightVectorsId = Shader.PropertyToID("_UgoeLightVectors");
        private static readonly int LightColorsId = Shader.PropertyToID("_UgoeLightColors");

        /// <summary>描画可能要素 1 つ分の頂点と、マスク用メッシュ。</summary>
        internal sealed class DrawableView
        {
            /// <summary>ほかの要素のマスクとして使うときのメッシュ (すべての三角形。マスク用メッシュでなければ null)。</summary>
            public Mesh Mesh;
            public Texture Texture;
            /// <summary>このオブジェクトの座標での頂点 (Y 上向き)。</summary>
            public Vector3[] Vertices;
            /// <summary>キャンバス座標の頂点 (マスクとライティングに使う)。</summary>
            public Vector2[] Canvas;
            /// <summary>Unity の UV (v は上向き)。</summary>
            public Vector2[] Uvs;
            /// <summary>ライティングの種類 (0 = なし、1 = 法線 (0, 0, 1)、2 = 法線マップ)。</summary>
            public float LitMode;
        }

        /// <summary>1 回の描画でまとめて描く要素 (<see cref="DrawBatch"/>) の Unity の物。</summary>
        private sealed class BatchView
        {
            public GameObject GameObject;
            public MeshRenderer Renderer;
            public Mesh Mesh;
            public DrawBatch Batch;
            /// <summary>次の転送ですべてを送るか (作った直後とバッチを作り直した後)。</summary>
            public bool Full;
            public Vector3[] Vertices;
            public Vector2[] Canvas;
            public Vector2[] Uvs;
            public Color[] Colors;
            public Vector4[] Screen;
            public Vector4[] Extra;
            public Vector4[] MaskMap;
            /// <summary>次の転送で、すべての要素の色と頂点属性を送るか (マスクのページの配置が変わったとき)。</summary>
            public bool AllColors;
            public int[] Indices;
            public MaterialPropertyBlock Block = new MaterialPropertyBlock();
            public readonly BatchUpload Upload = new BatchUpload();
        }

        /// <summary>表示中のモデル。代入すると読み直す。</summary>
        public UgoeModel Model
        {
            get => model;
            set
            {
                model = value;
                Load();
            }
        }

        /// <summary>
        /// 直前の転送での描画の呼び出しの数 (まとめたメッシュの数。マスクの内容と、Linear の色空間で中間のテクスチャを
        /// 画面に写す分は含まない)。
        /// テクスチャをアトラスにまとめたモデル (<c>ugoe build --atlas</c>) では要素の数より少なくなる。
        /// </summary>
        public int DrawCalls => batchViews.Count;

        /// <summary>中間のテクスチャを使っているか (Linear の色空間で <see cref="linearCompositing"/> が Intermediate)。</summary>
        public bool UsesIntermediate =>
            QualitySettings.activeColorSpace == ColorSpace.Linear && linearCompositing == LinearCompositing.Intermediate;

        /// <summary>Linear の色空間で画面に直接描くときの注意 (README の「色空間」)。</summary>
        public const string LinearColorSpaceWarning =
            "The project uses the Linear color space and linearCompositing is Direct: semi-transparent pixels blend " +
            "slightly differently from the ugoe reference renderer. Use Intermediate (the default) or the Gamma color " +
            "space to match it exactly.";

        private static bool linearWarned;

        /// <summary>評価を行う <see cref="Ugoe.Puppet"/> (読み込んでいなければ null)。直接操作してよい
        /// (着せ替えは <see cref="AttachModule"/> などを使うと描画も作り直す)。</summary>
        public Puppet Puppet => puppet;

        public float PixelsPerUnit
        {
            get => pixelsPerUnit;
            set
            {
                pixelsPerUnit = PixelScale.Sanitize(value, pixelsPerUnit);
                MarkAllDirty();
            }
        }

        // --- 読み込み ---

        /// <summary><see cref="model"/> を読み直す (モジュールとパラメータも設定し直す)。</summary>
        public void Load()
        {
            needsLoad = false;
            Unload();
            loaded.Remember(model, modules, pixelsPerUnit);
            if (model == null || model.Bytes == null || model.Bytes.Length == 0) return;
            var modelName = model.name;
            // 読めないモジュール (壊れた .ugoemod) は飛ばして、ほかのモジュールと本体を表示する。
            // 準備の途中で失敗したら Puppet を残さない (PuppetLoad)。
            puppet = PuppetLoad.Run(model.CreatePuppet, modules, m => m != null ? m.Module : null, p =>
            {
                puppet = p;
                foreach (var w in p.LoadWarnings) Debug.LogWarning($"ugoe: {modelName}: {w}", this);
                if (QualitySettings.activeColorSpace == ColorSpace.Linear && linearCompositing == LinearCompositing.Direct && !linearWarned)
                {
                    // 1 回だけ知らせる (Gamma に切り替えない選択もある)。
                    linearWarned = true;
                    Debug.LogWarning($"ugoe: {LinearColorSpaceWarning}", this);
                }
                ApplySavedParameters();
                textures.LoadAll(p, modelName);
                Build();
            }, (stage, index, e) => LogLoadError(modelName, stage, index, e));
            if (puppet == null)
            {
                // 作りかけの描画とテクスチャを片付ける (Puppet は PuppetLoad が破棄した)。
                Unload();
                return;
            }
            ModelLoaded?.Invoke();
        }

        private void LogLoadError(string modelName, LoadStage stage, int index, Exception e)
        {
            switch (stage)
            {
                case LoadStage.ReadModule:
                    Debug.LogError($"ugoe: could not read the module {modules[index].name}: {e.Message}", this);
                    break;
                case LoadStage.AttachModules:
                    var kind = e is UgoeException u ? $" ({u.Kind})" : "";
                    Debug.LogError($"ugoe: could not attach the modules of {name}: {e.Message}{kind}", this);
                    break;
                default:
                    Debug.LogError($"ugoe: could not load {modelName}: {e.Message}", this);
                    break;
            }
        }

        private void Unload()
        {
            DestroyViews();
            textures.Clear();
            puppet?.Dispose();
            puppet = null;
        }

        // 描画可能要素の並びに合わせて、頂点の控え・マスク用メッシュ・マスクを作る (読み込みと着せ替えの後)。
        // まとめたメッシュ (子の GameObject) は、最初の評価の後に描画順を見て作る (RebuildBatches)。
        private void Build()
        {
            DestroyViews();
            EnsureMaterials();
            sync = new FrameSync(puppet);
            plan = MaskPlan.Build(puppet.Drawables);
            masks = new UgoeMaskRenderer(plan, maskMaterial);
            var drawables = puppet.Drawables;
            indexLists = new ushort[drawables.Count][];
            for (var i = 0; i < drawables.Count; i++)
            {
                var info = drawables[i];
                var uvs = puppet.GetUvs(i);
                var view = new DrawableView
                {
                    Vertices = new Vector3[info.VertexCount],
                    Canvas = new Vector2[info.VertexCount],
                    Uvs = new Vector2[info.VertexCount],
                    Texture = textures.GetColor(info.Texture),
                    LitMode = info.Lit ? (textures.GetNormal(info.Texture) != null ? 2 : 1) : 0,
                };
                for (var v = 0; v < view.Uvs.Length; v++) view.Uvs[v] = new Vector2(uvs[v * 2], 1 - uvs[v * 2 + 1]);
                indexLists[i] = puppet.GetIndices(i);
                if (plan.IsMaskSource[i])
                {
                    view.Mesh = new Mesh { name = $"{info.Id} (mask)", hideFlags = HideFlags.DontSave };
                    view.Mesh.MarkDynamic();
                    view.Mesh.vertices = view.Vertices;
                    view.Mesh.uv = view.Uvs;
                    view.Mesh.uv2 = view.Canvas;
                    var all = new int[indexLists[i].Length];
                    for (var k = 0; k < all.Length; k++) all[k] = indexLists[i][k];
                    view.Mesh.SetIndices(all, MeshTopology.Triangles, 0, false);
                }
                views.Add(view);
            }
            batchPlan = null;
            lightsDirty = true;
            dirty = true;
            // 作った直後はすべての変更フラグが立っているので、次の Pull ですべて送る。
            Advance(0);
        }

        // 描画順と合成モードからバッチを決め直し、まとめたメッシュを作り直す (子の GameObject は使い回す)。
        private void RebuildBatches()
        {
            batchingUsed = batching;
            batchPlan = DrawBatchPlan.Build(puppet.Drawables, sync.States, sync.RenderOrder, plan, batching);
            var batches = batchPlan.Batches;
            for (var b = 0; b < batches.Count; b++)
            {
                if (b == batchViews.Count) batchViews.Add(CreateBatchView());
                var bv = batchViews[b];
                var batch = batches[b];
                var first = puppet.Drawables[batch.Members[0]].Id;
                bv.GameObject.name = batch.Members.Length == 1 ? first : $"{first} (+{batch.Members.Length - 1})";
                bv.Batch = batch;
                bv.Full = true;
                bv.Vertices = new Vector3[batch.VertexCount];
                bv.Canvas = new Vector2[batch.VertexCount];
                bv.Uvs = new Vector2[batch.VertexCount];
                bv.Colors = new Color[batch.VertexCount];
                bv.Screen = new Vector4[batch.VertexCount];
                bv.Extra = new Vector4[batch.VertexCount];
                bv.MaskMap = new Vector4[batch.VertexCount];
                bv.Indices = new int[batch.IndexCapacity];
                for (var k = 0; k < batch.Members.Length; k++)
                {
                    var uvs = views[batch.Members[k]].Uvs;
                    Array.Copy(uvs, 0, bv.Uvs, batch.VertexOffsets[k], uvs.Length);
                }
                bv.Renderer.sharedMaterial = blendMaterials[(int)batch.BlendMode];
                bv.Renderer.sortingOrder = b;
                var normal = textures.GetNormal(batch.Texture);
                bv.Block.SetTexture(MainTexId, textures.GetColor(batch.Texture));
                bv.Block.SetTexture(NormalTexId, normal != null ? normal : UgoeTextures.FlatNormal);
                masks.BindPageRead(bv.Block, batch.MaskPage);
                bv.Renderer.SetPropertyBlock(bv.Block);
                bv.Renderer.forceRenderingOff = intermediateUsed;
            }
            while (batchViews.Count > batches.Count)
            {
                var last = batchViews[batchViews.Count - 1];
                DestroyObject(last.Mesh);
                DestroyObject(last.GameObject);
                batchViews.RemoveAt(batchViews.Count - 1);
            }
        }

        private BatchView CreateBatchView()
        {
            var go = new GameObject("ugoe batch") { hideFlags = HideFlags.DontSave | HideFlags.NotEditable };
            go.layer = gameObject.layer;
            go.transform.SetParent(transform, false);
            var mesh = new Mesh { name = "ugoe batch", hideFlags = HideFlags.DontSave };
            mesh.MarkDynamic();
            go.AddComponent<MeshFilter>().sharedMesh = mesh;
            var renderer = go.AddComponent<MeshRenderer>();
            renderer.shadowCastingMode = ShadowCastingMode.Off;
            renderer.receiveShadows = false;
            renderer.lightProbeUsage = LightProbeUsage.Off;
            renderer.reflectionProbeUsage = ReflectionProbeUsage.Off;
            return new BatchView { GameObject = go, Renderer = renderer, Mesh = mesh };
        }

        private void DestroyViews()
        {
            foreach (var v in views) DestroyObject(v.Mesh);
            views.Clear();
            foreach (var b in batchViews)
            {
                DestroyObject(b.Mesh);
                DestroyObject(b.GameObject);
            }
            batchViews.Clear();
            batchPlan = null;
            masks?.Dispose();
            masks = null;
            intermediate?.Dispose();
            intermediate = null;
        }

        private void EnsureMaterials()
        {
            if (blendMaterials != null && blendMaterials[0] != null) return;
            var drawable = Resources.Load<Shader>("Ugoe/UgoeDrawable");
            var mask = Resources.Load<Shader>("Ugoe/UgoeMask");
            var composite = Resources.Load<Shader>("Ugoe/UgoeComposite");
            Material[] Blends(bool gammaTarget)
            {
                var suffix = gammaTarget ? " (intermediate)" : "";
                var materials = new[]
                {
                    BlendMaterial(drawable, "normal" + suffix, BlendModeFactor.One, BlendModeFactor.OneMinusSrcAlpha, BlendModeFactor.One, BlendModeFactor.OneMinusSrcAlpha),
                    BlendMaterial(drawable, "additive" + suffix, BlendModeFactor.One, BlendModeFactor.One, BlendModeFactor.Zero, BlendModeFactor.One),
                    BlendMaterial(drawable, "multiply" + suffix, BlendModeFactor.DstColor, BlendModeFactor.OneMinusSrcAlpha, BlendModeFactor.Zero, BlendModeFactor.One),
                };
                if (gammaTarget)
                {
                    foreach (var m in materials) m.EnableKeyword("UGOE_GAMMA_TARGET");
                }
                return materials;
            }
            blendMaterials = Blends(false);
            gammaMaterials = Blends(true);
            maskMaterial = new Material(mask) { name = "ugoe mask", hideFlags = HideFlags.DontSave };
            compositeMaterial = new Material(composite) { name = "ugoe composite", hideFlags = HideFlags.DontSave };
        }

        private enum BlendModeFactor
        {
            Zero = (int)UnityEngine.Rendering.BlendMode.Zero,
            One = (int)UnityEngine.Rendering.BlendMode.One,
            DstColor = (int)UnityEngine.Rendering.BlendMode.DstColor,
            OneMinusSrcAlpha = (int)UnityEngine.Rendering.BlendMode.OneMinusSrcAlpha,
        }

        private static Material BlendMaterial(Shader shader, string label, BlendModeFactor src, BlendModeFactor dst, BlendModeFactor srcA, BlendModeFactor dstA)
        {
            var m = new Material(shader) { name = $"ugoe {label}", hideFlags = HideFlags.DontSave };
            m.SetFloat("_SrcBlend", (int)src);
            m.SetFloat("_DstBlend", (int)dst);
            m.SetFloat("_SrcBlendAlpha", (int)srcA);
            m.SetFloat("_DstBlendAlpha", (int)dstA);
            return m;
        }

        // --- 毎フレーム ---

        private void OnEnable()
        {
            needsLoad = true;
#if UNITY_EDITOR
            // エディタで再生していない間も、有効になったらすぐに表示する。
            if (!Application.isPlaying) UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
#endif
        }

        private void OnDisable()
        {
            Unload();
            needsLoad = true;
        }

        private void OnDestroy()
        {
            Unload();
            foreach (var set in new[] { blendMaterials, gammaMaterials })
            {
                if (set == null) continue;
                foreach (var m in set) DestroyObject(m);
            }
            DestroyObject(maskMaterial);
            DestroyObject(compositeMaterial);
        }

        private void OnValidate()
        {
            // インスペクタで変えたものは次の LateUpdate で反映する (OnValidate の中では子を作れない)。
            pixelsPerUnit = PixelScale.Sanitize(pixelsPerUnit, PixelScale.Default);
            validated = true;
            lightsDirty = true;
        }

        private void LateUpdate()
        {
            if (validated)
            {
                validated = false;
                // モデルかモジュールか頂点の変換 (pixelsPerUnit) が変わったときだけ読み直す。パラメータは設定し直す。
                if (loaded.NeedsReload(model, modules, pixelsPerUnit)) needsLoad = true;
                else if (puppet != null)
                {
                    ApplySavedParameters();
                    dirty = true;
                }
            }
            if (needsLoad) Load();
            if (puppet == null) return;
            // マスクのページが失われた (デバイスの作り直しなど) ら、止まっていても描き直す。
            if (masks != null && masks.Lost) dirty = true;
            var run = playing && (Application.isPlaying || playInEditor);
            // 逆再生はできない (C ABI は負の dt を断る) ので、負の速さは止めているのと同じにする。
            var dt = run ? Mathf.Max(Time.deltaTime * speed, 0f) : 0f;
            if (run || dirty) Advance(dt);
            if (lightsDirty) ApplyLights();
            RenderIntermediate();
        }

        // Linear の色空間で、まとめたメッシュを中間のテクスチャに描いて写す (UgoeIntermediate)。使わないときは画面に直接描く。
        private void RenderIntermediate()
        {
            var use = UsesIntermediate && puppet != null;
            if (use != intermediateUsed)
            {
                intermediateUsed = use;
                foreach (var bv in batchViews) bv.Renderer.forceRenderingOff = use;
                if (intermediate != null) intermediate.Visible = use;
                intermediateDirty = true;
            }
            if (!use) return;
            if (intermediate == null)
            {
                intermediate = new UgoeIntermediate(transform, compositeMaterial);
                intermediateDirty = true;
            }
            if (!intermediateDirty && !intermediate.Lost) return;
            intermediateDirty = false;
            var bounds = CanvasRect.Empty;
            var draws = intermediateDraws;
            draws.Clear();
            foreach (var bv in batchViews)
            {
                if (!bv.Renderer.enabled) continue;
                foreach (var i in bv.Batch.Members) bounds = CanvasRect.Union(bounds, sync.Positions[i]);
                draws.Add((bv.Mesh, gammaMaterials[(int)bv.Batch.BlendMode], bv.Block));
            }
            var ox = puppet.Canvas.OriginX;
            var oy = puppet.Canvas.OriginY;
            // バッチの頂点 ((x − ox) / ppu, (oy − y) / ppu) をキャンバス座標に戻す。
            var toCanvas = Matrix4x4.TRS(new Vector3(ox, oy, 0), Quaternion.identity, new Vector3(pixelsPerUnit, -pixelsPerUnit, 1));
            var inv = 1 / pixelsPerUnit;
            intermediate.Render(bounds, lastResolution, toCanvas, c => new Vector3((c.x - ox) * inv, (oy - c.y) * inv, 0), draws);
            intermediate.Visible = true;
        }

        // シーンに保存したパラメータの値を設定する (知らない ID と NaN は飛ばす)。
        private void ApplySavedParameters()
        {
            foreach (var p in parameters)
            {
                if (!float.IsNaN(p.value)) puppet.SetParameter(p.id, p.value);
            }
        }

        /// <summary>
        /// 時間を <paramref name="dt"/> 秒進めて描画を更新する (<see cref="playing"/> が false のときに手で進める)。
        /// </summary>
        public void Advance(float dt)
        {
            if (puppet == null) return;
            puppet.Update(dt);
            AfterEvaluate();
            if (MotionEventFired != null)
            {
                foreach (var e in puppet.MotionEvents()) MotionEventFired(e);
            }
        }

        /// <summary>
        /// 受け手として、指揮役から受けた値で <paramref name="dt"/> 秒進めて描画を更新する (<see cref="Ugoe.Puppet.Follow"/>)。
        /// 受け手では <see cref="playing"/> を false にして毎フレームこれを呼び、<see cref="SetParameter"/> などは使わない
        /// (使うと次の LateUpdate が <see cref="Ugoe.Puppet.Update"/> で評価し直す)。
        /// </summary>
        public void Follow(float[] values, float? lipSyncLevel, float lipSyncStrength, IReadOnlyList<PartOpacityInput> partOpacities,
            float dt)
        {
            if (puppet == null) return;
            puppet.Follow(values, lipSyncLevel, lipSyncStrength, partOpacities, dt);
            AfterEvaluate();
        }

        private void AfterEvaluate()
        {
            dirty = false;
            Sync();
            intermediateDirty = true;
        }

        // 変更フラグが立った分だけ Unity に送る。
        private void Sync()
        {
            sync.Pull();
            if (SyncRules.NeedsBatchRebuild(batchPlan, sync.RenderOrderChanged, batching, batchingUsed, sync.States))
            {
                RebuildBatches();
            }
            var ox = puppet.Canvas.OriginX;
            var oy = puppet.Canvas.OriginY;
            var inv = 1 / pixelsPerUnit;
            for (var i = 0; i < views.Count; i++)
            {
                var view = views[i];
                if (sync.PositionsChanged[i])
                {
                    var p = sync.Positions[i];
                    for (var v = 0; v < view.Vertices.Length; v++)
                    {
                        float x = p[v * 2], y = p[v * 2 + 1];
                        view.Canvas[v] = new Vector2(x, y);
                        view.Vertices[v] = new Vector3((x - ox) * inv, (oy - y) * inv, 0);
                    }
                    if (view.Mesh != null)
                    {
                        view.Mesh.vertices = view.Vertices;
                        view.Mesh.uv2 = view.Canvas;
                        view.Mesh.RecalculateBounds();
                    }
                }
            }
            var scale = MaskResolution();
            var maskDirty = SyncRules.NeedsMaskRedraw(masks.HasMasks,
                SyncRules.MaskContentChanged(plan, sync.PositionsChanged, sync.States), scale, lastMaskScale, masks.Lost);
            lastResolution = scale;
            if (maskDirty)
            {
                lastMaskScale = scale;
                if (masks.Render(views, sync, scale))
                {
                    // ページのテクスチャか内容の位置が変わったので、読むバッチのテクスチャと、要素ごとの写像を設定し直す。
                    foreach (var bv in batchViews)
                    {
                        bv.AllColors = true;
                        if (bv.Batch.MaskPage < 0) continue;
                        masks.BindPageRead(bv.Block, bv.Batch.MaskPage);
                        bv.Renderer.SetPropertyBlock(bv.Block);
                    }
                }
            }
            foreach (var bv in batchViews) SyncBatch(bv);
        }

        // まとめたメッシュ 1 つに、変わった要素の頂点・色・三角形を写して送る。
        private void SyncBatch(BatchView bv)
        {
            var batch = bv.Batch;
            var full = bv.Full;
            var upload = bv.Upload;
            upload.Plan(batch, puppet.Drawables, sync.PositionsChanged, sync.States, full, bv.AllColors);
            bv.Full = false;
            bv.AllColors = false;
            for (var k = 0; k < batch.Members.Length; k++)
            {
                var i = batch.Members[k];
                var offset = batch.VertexOffsets[k];
                var view = views[i];
                var state = sync.States[i];
                if (upload.MemberPositions[k])
                {
                    Array.Copy(view.Vertices, 0, bv.Vertices, offset, view.Vertices.Length);
                    Array.Copy(view.Canvas, 0, bv.Canvas, offset, view.Canvas.Length);
                }
                if (upload.MemberColors[k])
                {
                    var color = new Color(state.MultiplyR, state.MultiplyG, state.MultiplyB, state.Opacity);
                    var screen = new Vector4(state.ScreenR, state.ScreenG, state.ScreenB, 0);
                    // マスクを読まない要素 (とページをまだ描いていない要素) は mask_strength 0 (ページを読んでも係数は 1)。
                    var read = masks.VertexAttributes(i);
                    var reads = plan.ContentOf[i] >= 0 && masks.GetPage(bv.Batch.MaskPage) != null;
                    var extra = new Vector4(reads ? Mathf.Clamp01(state.MaskStrength) : 0, view.LitMode, read.Positive.x, read.Positive.y);
                    for (var v = 0; v < view.Vertices.Length; v++)
                    {
                        bv.Colors[offset + v] = color;
                        bv.Screen[offset + v] = screen;
                        bv.Extra[offset + v] = extra;
                        bv.MaskMap[offset + v] = read.Map;
                    }
                }
            }
            bool vertices = upload.Vertices, colors = upload.Colors, indices = upload.Indices;
            var mesh = bv.Mesh;
            if (full)
            {
                // 頂点の数が変わるので、三角形を消してから頂点を入れ替える。
                mesh.Clear();
                mesh.vertices = bv.Vertices;
                mesh.uv = bv.Uvs;
            }
            else if (vertices)
            {
                mesh.vertices = bv.Vertices;
            }
            if (vertices) mesh.uv2 = bv.Canvas;
            if (colors)
            {
                mesh.colors = bv.Colors;
                mesh.SetUVs(2, bv.Screen);
                mesh.SetUVs(3, bv.Extra);
                mesh.SetUVs(4, bv.MaskMap);
            }
            if (indices)
            {
                var n = batch.WriteIndices(puppet.Drawables, indexLists, sync.Positions, sync.States, bv.Indices);
                mesh.SetIndices(bv.Indices, 0, n, MeshTopology.Triangles, 0, false);
                bv.Renderer.enabled = n > 0;
            }
            if (vertices) mesh.RecalculateBounds();
        }

        // マスクの解像度 (キャンバスの 1 ピクセルあたりの画素数)。画面上の大きさに合わせ、揺れで作り直さないように 2^(1/4) 刻みに丸める。
        private float MaskResolution()
        {
            var cam = maskCamera != null ? maskCamera : Camera.main;
            var perPixel = 1f;
            if (cam != null)
            {
                var a = cam.WorldToScreenPoint(transform.position);
                var bx = cam.WorldToScreenPoint(transform.TransformPoint(new Vector3(1, 0, 0)));
                var by = cam.WorldToScreenPoint(transform.TransformPoint(new Vector3(0, 1, 0)));
                var perUnit = Mathf.Max(Vector2.Distance(a, bx), Vector2.Distance(a, by));
                if (perUnit > 0 && !float.IsInfinity(perUnit) && !float.IsNaN(perUnit)) perPixel = perUnit / pixelsPerUnit;
            }
            var s = Mathf.Clamp(perPixel * maskScale, 1f / 16, 16);
            return Mathf.Pow(2, Mathf.Ceil(Mathf.Log(s, 2) * 4) / 4);
        }

        private void MarkAllDirty()
        {
            if (puppet == null) return;
            // 頂点の変換が変わったので、次のフレームで全部を送り直す (読み直すのが簡単で確実)。
            needsLoad = true;
        }

        // --- ライティング ---

        /// <summary>環境光 (既定は白)。</summary>
        public Color AmbientLight
        {
            get => ambientLight;
            set
            {
                ambientLight = value;
                lightsDirty = true;
            }
        }

        /// <summary>光源の一覧 (変えたら <see cref="SetLights"/> で設定する)。</summary>
        public IReadOnlyList<UgoeLight> Lights => lights;

        /// <summary>光源を置き換える。座標はキャンバス座標 (<see cref="WorldToCanvas"/>)。</summary>
        public void SetLights(IEnumerable<UgoeLight> newLights)
        {
            lights = new List<UgoeLight>(newLights);
            lightsDirty = true;
        }

        private void ApplyLights()
        {
            lightsDirty = false;
            if (blendMaterials == null) return;
            intermediateDirty = true;
            var sources = new List<LightSource>();
            foreach (var l in lights) sources.Add(l.ToSource());
            LightingUniform u;
            try
            {
                u = LightingUniform.Create(ambientLight.r, ambientLight.g, ambientLight.b, sources);
            }
            catch (UgoeException e)
            {
                Debug.LogWarning($"ugoe: invalid lights: {e.Message}", this);
                u = LightingUniform.Default;
            }
            var vectors = new Vector4[LightingUniform.MaxLights];
            var colors = new Vector4[LightingUniform.MaxLights];
            for (var i = 0; i < LightingUniform.MaxLights; i++)
            {
                vectors[i] = new Vector4(u.Vectors[i * 4], u.Vectors[i * 4 + 1], u.Vectors[i * 4 + 2], u.Vectors[i * 4 + 3]);
                colors[i] = new Vector4(u.Colors[i * 4], u.Colors[i * 4 + 1], u.Colors[i * 4 + 2], u.Colors[i * 4 + 3]);
            }
            foreach (var m in AllBlendMaterials())
            {
                m.SetVector(AmbientId, new Vector4(u.Ambient[0], u.Ambient[1], u.Ambient[2], 0));
                m.SetFloat(LightCountId, u.Count);
                m.SetVectorArray(LightVectorsId, vectors);
                m.SetVectorArray(LightColorsId, colors);
            }
        }

        private IEnumerable<Material> AllBlendMaterials()
        {
            foreach (var m in blendMaterials) yield return m;
            if (gammaMaterials == null) yield break;
            foreach (var m in gammaMaterials) yield return m;
        }

        // --- 座標 ---

        /// <summary>ワールド座標をキャンバス座標 (ピクセル・Y 下向き) にする (当たり判定・光源の位置に使う)。</summary>
        public Vector2 WorldToCanvas(Vector3 world)
        {
            var local = transform.InverseTransformPoint(world);
            if (puppet == null) return new Vector2(local.x * pixelsPerUnit, -local.y * pixelsPerUnit);
            return new Vector2(local.x * pixelsPerUnit + puppet.Canvas.OriginX, puppet.Canvas.OriginY - local.y * pixelsPerUnit);
        }

        /// <summary>キャンバス座標をワールド座標にする。</summary>
        public Vector3 CanvasToWorld(Vector2 canvas)
        {
            var o = puppet != null ? new Vector2(puppet.Canvas.OriginX, puppet.Canvas.OriginY) : Vector2.zero;
            return transform.TransformPoint(new Vector3((canvas.x - o.x) / pixelsPerUnit, (o.y - canvas.y) / pixelsPerUnit, 0));
        }

        // --- パラメータ・モーションなど (Puppet の薄い包み) ---

        /// <summary>ID でパラメータを設定する (次のフレームで描き直す)。知らない ID なら false。</summary>
        public bool SetParameter(string id, float value)
        {
            if (puppet == null || !puppet.SetParameter(id, value)) return false;
            dirty = true;
            return true;
        }

        public float GetParameter(string id)
        {
            var i = puppet?.FindParameter(id) ?? -1;
            return i >= 0 ? puppet.GetParameter(i) : float.NaN;
        }

        /// <summary>シーンに保存するパラメータの値を設定する (エディタのインスペクタから使う)。</summary>
        public void SetSavedParameter(string id, float value)
        {
            var index = parameters.FindIndex(p => p.id == id);
            var entry = new UgoeParameterValue { id = id, value = value };
            if (index >= 0) parameters[index] = entry;
            else parameters.Add(entry);
            SetParameter(id, value);
        }

        public bool StartMotion(string name, MotionPriority priority = MotionPriority.Normal)
        {
            if (puppet == null) return false;
            return puppet.StartMotion(name, priority);
        }

        public void SetExpression(string name)
        {
            puppet?.SetExpression(name);
            dirty = true;
        }

        /// <summary>ポーズで、パーツ <paramref name="part"/> の要素を表示する (フェードで切り替わる)。ポーズの要素でなければ false。</summary>
        public bool ShowPose(string part)
        {
            if (puppet == null || !puppet.ShowPose(part)) return false;
            dirty = true;
            return true;
        }

        // --- 着せ替え ---

        /// <summary>モジュールを付ける (同じスロットか名前は差し替え)。付けられなければエラーを出して false (何も変えない)。</summary>
        public bool AttachModule(UgoeModule module) => DressUp(() => puppet.AttachModule(module.Module), () =>
        {
            // C ABI と同じ規則で並びを変える (差し替えは同じ位置に入る)。読み直したときに同じ合成の順になるように。
            modules.RemoveAll(m => m == null);
            ModuleOrder.Attach(modules, module, m => m.Module.Name, m => m.Module.Slot);
        });

        /// <summary>名前でモジュールを外す。</summary>
        public bool DetachModule(string moduleName) => DressUp(() => puppet.DetachModule(moduleName),
            () => modules.RemoveAll(m => m == null || m.Module.Name == moduleName));

        /// <summary>付けるモジュールの組を決める (合成の順)。空なら本体だけに戻す。</summary>
        public bool SetModules(IReadOnlyList<UgoeModule> list)
        {
            var native = new List<Module>();
            foreach (var m in list) native.Add(m.Module);
            return DressUp(() => puppet.SetModules(native), () => modules = new List<UgoeModule>(list));
        }

        private bool DressUp(Action change, Action remember)
        {
            if (puppet == null) return false;
            try
            {
                change();
            }
            catch (UgoeException e)
            {
                Debug.LogError($"ugoe: dress-up failed: {e.Message} ({e.Kind})", this);
                return false;
            }
            remember();
            loaded.RememberModules(modules);
            // 描画可能要素の並びが変わったので作り直す。変わっていないテクスチャはデコードし直さない。
            textures.Refresh(puppet, model != null ? model.name : name);
            Build();
            return true;
        }

        // --- 当たり判定 ---

        /// <summary>ワールド座標の点を含む当たり判定の ID (手前から順)。</summary>
        public List<string> HitAreasAt(Vector3 world)
        {
            var result = new List<string>();
            if (puppet == null) return result;
            var c = WorldToCanvas(world);
            foreach (var i in puppet.HitAreasAt(c.x, c.y)) result.Add(puppet.HitAreaIds[i]);
            return result;
        }

        private static void DestroyObject(UnityEngine.Object o)
        {
            if (o == null) return;
            if (Application.isPlaying) Destroy(o);
            else DestroyImmediate(o);
        }
    }
}
