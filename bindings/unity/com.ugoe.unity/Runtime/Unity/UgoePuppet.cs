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
    /// <para>描画可能要素ごとに子の GameObject (<see cref="MeshRenderer"/>) を作り、描画順は <c>sortingOrder</c> で決める
    /// (このオブジェクトの <see cref="SortingGroup"/> の中で並ぶので、ほかの 2D の物とは 1 つのまとまりとして並ぶ)。
    /// 子は保存しない (<see cref="HideFlags.DontSave"/>)。</para>
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
        [Tooltip("マスクの解像度の倍率。1 で画面上の大きさと同じ")]
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
        private Material[] blendMaterials;
        private Material maskMaterial;
        private bool dirty;
        private bool needsLoad = true;
        private bool validated;
        private UgoeModel loadedModel;
        private readonly List<UgoeModule> loadedModules = new List<UgoeModule>();
        private bool lightsDirty = true;
        private float lastMaskScale;

        private static readonly int MainTexId = Shader.PropertyToID("_MainTex");
        private static readonly int NormalTexId = Shader.PropertyToID("_NormalTex");
        private static readonly int ColorId = Shader.PropertyToID("_Color");
        private static readonly int ScreenColorId = Shader.PropertyToID("_ScreenColor");
        private static readonly int LitModeId = Shader.PropertyToID("_LitMode");
        private static readonly int AmbientId = Shader.PropertyToID("_UgoeAmbient");
        private static readonly int LightCountId = Shader.PropertyToID("_UgoeLightCount");
        private static readonly int LightVectorsId = Shader.PropertyToID("_UgoeLightVectors");
        private static readonly int LightColorsId = Shader.PropertyToID("_UgoeLightColors");

        /// <summary>描画可能要素 1 つ分の Unity の物。</summary>
        internal sealed class DrawableView
        {
            public GameObject GameObject;
            public MeshRenderer Renderer;
            public Mesh Mesh;
            public Texture Texture;
            /// <summary>マスクとして描くときのサブメッシュ (カリングする要素は、裏向きも含めた 1 番)。</summary>
            public int MaskSubmesh;
            public Vector3[] Vertices;
            public Vector2[] Canvas;
            public ushort[] Indices;
            public int[] FrontIndices;
            public bool Culling;
            public BlendMode Blend = (BlendMode)(-1);
            public MaterialPropertyBlock Block = new MaterialPropertyBlock();
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

        /// <summary>評価を行う <see cref="Ugoe.Puppet"/> (読み込んでいなければ null)。直接操作してよい
        /// (着せ替えは <see cref="AttachModule"/> などを使うと描画も作り直す)。</summary>
        public Puppet Puppet => puppet;

        public float PixelsPerUnit
        {
            get => pixelsPerUnit;
            set
            {
                pixelsPerUnit = Mathf.Max(value, 1e-3f);
                MarkAllDirty();
            }
        }

        // --- 読み込み ---

        /// <summary><see cref="model"/> を読み直す (モジュールとパラメータも設定し直す)。</summary>
        public void Load()
        {
            needsLoad = false;
            Unload();
            loadedModel = model;
            loadedModules.Clear();
            loadedModules.AddRange(modules);
            if (model == null || model.Bytes == null || model.Bytes.Length == 0) return;
            try
            {
                puppet = model.CreatePuppet();
            }
            catch (Exception e)
            {
                Debug.LogError($"ugoe: could not load {model.name}: {e.Message}", this);
                return;
            }
            foreach (var w in puppet.LoadWarnings) Debug.LogWarning($"ugoe: {model.name}: {w}", this);
            var attach = new List<Module>();
            foreach (var m in modules)
            {
                if (m != null) attach.Add(m.Module);
            }
            if (attach.Count > 0)
            {
                try
                {
                    puppet.SetModules(attach);
                }
                catch (UgoeException e)
                {
                    Debug.LogError($"ugoe: could not attach the modules of {name}: {e.Message} ({e.Kind})", this);
                }
            }
            foreach (var p in parameters) puppet.SetParameter(p.id, p.value);
            textures.LoadAll(puppet, model.name);
            Build();
            ModelLoaded?.Invoke();
        }

        private void Unload()
        {
            DestroyViews();
            textures.Clear();
            puppet?.Dispose();
            puppet = null;
        }

        // 描画可能要素の並びに合わせて、子・メッシュ・マスクを作る (読み込みと着せ替えの後)。
        private void Build()
        {
            DestroyViews();
            EnsureMaterials();
            sync = new FrameSync(puppet);
            plan = MaskPlan.Build(puppet.Drawables);
            masks = new UgoeMaskRenderer(plan, maskMaterial);
            var drawables = puppet.Drawables;
            for (var i = 0; i < drawables.Count; i++)
            {
                var info = drawables[i];
                var go = new GameObject(info.Id) { hideFlags = HideFlags.DontSave | HideFlags.NotEditable };
                go.layer = gameObject.layer;
                go.transform.SetParent(transform, false);
                var mesh = new Mesh { name = info.Id, hideFlags = HideFlags.DontSave };
                mesh.MarkDynamic();
                var uvs = puppet.GetUvs(i);
                var unityUvs = new Vector2[info.VertexCount];
                for (var v = 0; v < unityUvs.Length; v++) unityUvs[v] = new Vector2(uvs[v * 2], 1 - uvs[v * 2 + 1]);
                var view = new DrawableView
                {
                    GameObject = go,
                    Mesh = mesh,
                    Vertices = new Vector3[info.VertexCount],
                    Canvas = new Vector2[info.VertexCount],
                    Indices = puppet.GetIndices(i),
                    Culling = info.Culling,
                    Texture = textures.GetColor(info.Texture),
                };
                mesh.vertices = view.Vertices;
                mesh.uv = unityUvs;
                mesh.uv2 = view.Canvas;
                var all = new int[view.Indices.Length];
                for (var k = 0; k < all.Length; k++) all[k] = view.Indices[k];
                if (info.Culling)
                {
                    view.FrontIndices = new int[all.Length];
                    mesh.subMeshCount = 2;
                    mesh.SetIndices(all, MeshTopology.Triangles, 1, false);
                    view.MaskSubmesh = 1;
                }
                mesh.SetIndices(all, MeshTopology.Triangles, 0, false);

                go.AddComponent<MeshFilter>().sharedMesh = mesh;
                view.Renderer = go.AddComponent<MeshRenderer>();
                view.Renderer.shadowCastingMode = ShadowCastingMode.Off;
                view.Renderer.receiveShadows = false;
                view.Renderer.lightProbeUsage = LightProbeUsage.Off;
                view.Renderer.reflectionProbeUsage = ReflectionProbeUsage.Off;
                view.Renderer.sortingOrder = i;

                var normal = textures.GetNormal(info.Texture);
                view.Block.SetTexture(MainTexId, view.Texture);
                view.Block.SetTexture(NormalTexId, normal != null ? normal : UgoeTextures.FlatNormal);
                view.Block.SetFloat(LitModeId, info.Lit ? (normal != null ? 2 : 1) : 0);
                views.Add(view);
            }
            lightsDirty = true;
            dirty = true;
            // 作った直後はすべての変更フラグが立っているので、次の Pull ですべて送る。
            Advance(0);
        }

        private void DestroyViews()
        {
            foreach (var v in views)
            {
                DestroyObject(v.Mesh);
                DestroyObject(v.GameObject);
            }
            views.Clear();
            masks?.Dispose();
            masks = null;
        }

        private void EnsureMaterials()
        {
            if (blendMaterials != null && blendMaterials[0] != null) return;
            var drawable = Resources.Load<Shader>("Ugoe/UgoeDrawable");
            var mask = Resources.Load<Shader>("Ugoe/UgoeMask");
            blendMaterials = new[]
            {
                BlendMaterial(drawable, "normal", BlendModeFactor.One, BlendModeFactor.OneMinusSrcAlpha, BlendModeFactor.One, BlendModeFactor.OneMinusSrcAlpha),
                BlendMaterial(drawable, "additive", BlendModeFactor.One, BlendModeFactor.One, BlendModeFactor.Zero, BlendModeFactor.One),
                BlendMaterial(drawable, "multiply", BlendModeFactor.DstColor, BlendModeFactor.OneMinusSrcAlpha, BlendModeFactor.Zero, BlendModeFactor.One),
            };
            maskMaterial = new Material(mask) { name = "ugoe mask", hideFlags = HideFlags.DontSave };
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
            if (blendMaterials != null)
            {
                foreach (var m in blendMaterials) DestroyObject(m);
            }
            DestroyObject(maskMaterial);
        }

        private void OnValidate()
        {
            // インスペクタで変えたものは次の LateUpdate で反映する (OnValidate の中では子を作れない)。
            validated = true;
            lightsDirty = true;
        }

        private void LateUpdate()
        {
            if (validated)
            {
                validated = false;
                // モデルかモジュールが変わったときだけ読み直す。パラメータは設定し直す。
                if (model != loadedModel || !SameModules()) needsLoad = true;
                else if (puppet != null)
                {
                    foreach (var p in parameters) puppet.SetParameter(p.id, p.value);
                    dirty = true;
                }
            }
            if (needsLoad) Load();
            if (puppet == null) return;
            var run = playing && (Application.isPlaying || playInEditor);
            var dt = run ? Time.deltaTime * speed : 0f;
            if (run || dirty) Advance(dt);
            if (lightsDirty) ApplyLights();
        }

        private bool SameModules()
        {
            if (modules.Count != loadedModules.Count) return false;
            for (var i = 0; i < modules.Count; i++)
            {
                if (modules[i] != loadedModules[i]) return false;
            }
            return true;
        }

        /// <summary>
        /// 時間を <paramref name="dt"/> 秒進めて描画を更新する (<see cref="playing"/> が false のときに手で進める)。
        /// </summary>
        public void Advance(float dt)
        {
            if (puppet == null) return;
            puppet.Update(dt);
            dirty = false;
            Sync();
            if (MotionEventFired != null)
            {
                foreach (var e in puppet.MotionEvents()) MotionEventFired(e);
            }
        }

        // 変更フラグが立った分だけ Unity に送る。
        private void Sync()
        {
            sync.Pull();
            var ox = puppet.Canvas.OriginX;
            var oy = puppet.Canvas.OriginY;
            var inv = 1 / pixelsPerUnit;
            var maskDirty = false;
            for (var i = 0; i < views.Count; i++)
            {
                var view = views[i];
                var state = sync.States[i];
                var changes = state.Changes;
                if (sync.PositionsChanged[i])
                {
                    var p = sync.Positions[i];
                    for (var v = 0; v < view.Vertices.Length; v++)
                    {
                        float x = p[v * 2], y = p[v * 2 + 1];
                        view.Canvas[v] = new Vector2(x, y);
                        view.Vertices[v] = new Vector3((x - ox) * inv, (oy - y) * inv, 0);
                    }
                    view.Mesh.vertices = view.Vertices;
                    view.Mesh.uv2 = view.Canvas;
                    if (view.Culling)
                    {
                        var n = Culling.FrontFacing(p, view.Indices, view.FrontIndices);
                        view.Mesh.SetIndices(view.FrontIndices, 0, n, MeshTopology.Triangles, 0, false);
                    }
                    view.Mesh.RecalculateBounds();
                    if (plan.IsMaskSource[i] || plan.ContentOf[i] >= 0) maskDirty = true;
                }
                if ((changes & DrawableChanges.MaskStrength) != 0 && plan.IsMaskSource[i] && plan.ContentOf[i] >= 0) maskDirty = true;
                if (view.Blend != state.BlendMode)
                {
                    view.Blend = state.BlendMode;
                    view.Renderer.sharedMaterial = blendMaterials[(int)state.BlendMode];
                }
                if ((changes & DrawableChanges.Visibility) != 0) view.Renderer.enabled = state.Visible;
                if ((changes & (DrawableChanges.Opacity | DrawableChanges.MultiplyColor | DrawableChanges.ScreenColor | DrawableChanges.MaskStrength)) != 0)
                {
                    view.Block.SetVector(ColorId, new Vector4(state.MultiplyR, state.MultiplyG, state.MultiplyB, state.Opacity));
                    view.Block.SetVector(ScreenColorId, new Vector4(state.ScreenR, state.ScreenG, state.ScreenB, 0));
                    masks.BindMaskRead(view.Block, i, state.MaskStrength);
                    view.Renderer.SetPropertyBlock(view.Block);
                }
            }
            if (sync.RenderOrderChanged)
            {
                for (var rank = 0; rank < sync.RenderOrder.Length; rank++) views[sync.RenderOrder[rank]].Renderer.sortingOrder = rank;
            }

            var scale = MaskResolution();
            if (!Mathf.Approximately(scale, lastMaskScale)) maskDirty = true;
            if (masks.HasMasks && maskDirty)
            {
                lastMaskScale = scale;
                masks.Render(views, sync, scale);
                // 矩形とテクスチャが変わりうるので、マスクを読む要素の値を設定し直す。
                for (var i = 0; i < views.Count; i++)
                {
                    if (plan.ContentOf[i] < 0) continue;
                    masks.BindMaskRead(views[i].Block, i, sync.States[i].MaskStrength);
                    views[i].Renderer.SetPropertyBlock(views[i].Block);
                }
            }
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
            foreach (var m in blendMaterials)
            {
                m.SetVector(AmbientId, new Vector4(u.Ambient[0], u.Ambient[1], u.Ambient[2], 0));
                m.SetFloat(LightCountId, u.Count);
                m.SetVectorArray(LightVectorsId, vectors);
                m.SetVectorArray(LightColorsId, colors);
            }
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

        // --- 着せ替え ---

        /// <summary>モジュールを付ける (同じスロットか名前は差し替え)。付けられなければエラーを出して false (何も変えない)。</summary>
        public bool AttachModule(UgoeModule module) => DressUp(() => puppet.AttachModule(module.Module), () =>
        {
            modules.RemoveAll(m => m == null || m.Module.Name == module.Module.Name || m.Module.Slot == module.Module.Slot);
            modules.Add(module);
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
            loadedModules.Clear();
            loadedModules.AddRange(modules);
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
