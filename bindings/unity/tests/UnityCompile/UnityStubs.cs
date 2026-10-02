// SPDX-License-Identifier: MIT
// Unity の API の「形」だけを手で書いたもの (本体は空)。Unity の無い環境で、Runtime/Unity・Editor・Samples~ の C# が
// 型の上で通るかを dotnet build で確かめるために使う (bindings/unity/build.mjs --check)。
// Unity の参照アセンブリ (UnityEngine.dll) はリポジトリに入れない。ここに無い API を使ったら、Unity の
// スクリプトリファレンスで形を確かめてから足す。実際に Unity で通るかは Unity で確かめる (README)。
#pragma warning disable CS0067, CS0626, CS1591
using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public class Object
    {
        public string name { get; set; }
        public HideFlags hideFlags { get; set; }
        public static void Destroy(Object o) { }
        public static void DestroyImmediate(Object o) { }
        public static implicit operator bool(Object o) => !ReferenceEquals(o, null);
    }

    [Flags]
    public enum HideFlags { None = 0, HideInHierarchy = 1, HideInInspector = 2, DontSaveInEditor = 4, NotEditable = 8, DontSaveInBuild = 16, DontUnloadUnusedAsset = 32, DontSave = 52, HideAndDontSave = 61 }

    public class ScriptableObject : Object
    {
        public static T CreateInstance<T>() where T : ScriptableObject => default;
    }

    public class Component : Object
    {
        public GameObject gameObject => null;
        public Transform transform => null;
        public T GetComponent<T>() => default;
    }

    public class Behaviour : Component
    {
        public bool enabled { get; set; }
    }

    public class MonoBehaviour : Behaviour { }

    public sealed class GameObject : Object
    {
        public GameObject(string name) { }
        public int layer { get; set; }
        public Transform transform => null;
        public T AddComponent<T>() where T : Component => default;
    }

    public class Transform : Component
    {
        public Vector3 position { get; set; }
        public void SetParent(Transform parent, bool worldPositionStays) { }
        public Vector3 TransformPoint(Vector3 p) => p;
        public Vector3 InverseTransformPoint(Vector3 p) => p;
    }

    public static class Application
    {
        public static bool isPlaying => false;
    }

    public static class Debug
    {
        public static void Log(object message) { }
        public static void LogWarning(object message) { }
        public static void LogWarning(object message, Object context) { }
        public static void LogError(object message, Object context) { }
    }

    public enum ColorSpace { Uninitialized = -1, Gamma = 0, Linear = 1 }

    public static class QualitySettings
    {
        public static ColorSpace activeColorSpace => ColorSpace.Gamma;
    }

    public static class Time
    {
        public static float deltaTime => 0;
    }

    public static class Mathf
    {
        public static float Min(params float[] values) => 0;
        public static float Min(float a, float b) => 0;
        public static int Min(int a, int b) => 0;
        public static float Max(float a, float b) => 0;
        public static float Clamp(float v, float min, float max) => 0;
        public static int Clamp(int v, int min, int max) => 0;
        public static float Clamp01(float v) => 0;
        public static int CeilToInt(float f) => 0;
        public static float Ceil(float f) => 0;
        public static bool Approximately(float a, float b) => false;
        public static float Pow(float f, float p) => 0;
        public static float Log(float f, float p) => 0;
        public static float Sin(float f) => 0;
    }

    public struct Vector2
    {
        public float x, y;
        public Vector2(float x, float y) { this.x = x; this.y = y; }
        public static Vector2 zero => default;
        public static Vector2 one => default;
        public static Vector2 right => default;
        public static Vector2 up => default;
        public static float Distance(Vector2 a, Vector2 b) => 0;
        public static implicit operator Vector2(Vector3 v) => default;
        public static implicit operator Vector3(Vector2 v) => default;
    }

    public struct Vector3
    {
        public float x, y, z;
        public Vector3(float x, float y, float z) { this.x = x; this.y = y; this.z = z; }
        public static Vector3 zero => default;
        public static Vector3 one => default;
        public static Vector3 right => default;
        public static Vector3 up => default;
        public static Vector3 operator -(Vector3 a, Vector3 b) => default;
        public static Vector3 operator *(Vector3 a, float b) => default;
    }

    public struct Vector4
    {
        public float x, y, z, w;
        public Vector4(float x, float y, float z, float w) { this.x = x; this.y = y; this.z = z; this.w = w; }
        public static bool operator ==(Vector4 a, Vector4 b) => a.x == b.x && a.y == b.y && a.z == b.z && a.w == b.w;
        public static bool operator !=(Vector4 a, Vector4 b) => !(a == b);
        public override bool Equals(object o) => o is Vector4 v && v == this;
        public override int GetHashCode() => 0;
    }

    public struct Quaternion
    {
        public static Quaternion identity => default;
    }

    public struct Color
    {
        public float r, g, b, a;
        public Color(float r, float g, float b) { this.r = r; this.g = g; this.b = b; a = 1; }
        public Color(float r, float g, float b, float a) { this.r = r; this.g = g; this.b = b; this.a = a; }
        public static Color white => default;
    }

    public struct Color32
    {
        public byte r, g, b, a;
        public Color32(byte r, byte g, byte b, byte a) { this.r = r; this.g = g; this.b = b; this.a = a; }
        public static implicit operator Color(Color32 c) => default;
    }

    public struct Rect
    {
        public Rect(float x, float y, float width, float height) { }
        public float xMin => 0;
        public float yMin => 0;
        public float width => 0;
        public float height => 0;
        public static Rect MinMaxRect(float xmin, float ymin, float xmax, float ymax) => default;
        public bool Contains(Vector2 point) => false;
    }

    public struct Bounds
    {
        public Bounds(Vector3 center, Vector3 size) { }
    }

    public struct Matrix4x4
    {
        public static Matrix4x4 identity => default;
        public static Matrix4x4 Ortho(float left, float right, float bottom, float top, float zNear, float zFar) => default;
        public static Matrix4x4 TRS(Vector3 pos, Quaternion q, Vector3 s) => default;
    }

    public enum MeshTopology { Triangles = 0 }

    public sealed class Mesh : Object
    {
        public Vector3[] vertices { get; set; }
        public Vector2[] uv { get; set; }
        public Vector2[] uv2 { get; set; }
        public Color[] colors { get; set; }
        public int subMeshCount { get; set; }
        public void Clear() { }
        public Bounds bounds { get; set; }
        public void MarkDynamic() { }
        public void SetVertices(List<Vector3> v) { }
        public void SetUVs(int channel, List<Vector2> uvs) { }
        public void SetUVs(int channel, Vector2[] uvs) { }
        public void SetUVs(int channel, Vector4[] uvs) { }
        public void SetIndices(int[] indices, MeshTopology topology, int submesh, bool calculateBounds = true, int baseVertex = 0) { }
        public void SetIndices(int[] indices, int indicesStart, int indicesLength, MeshTopology topology, int submesh, bool calculateBounds = true, int baseVertex = 0) { }
        public void RecalculateBounds() { }
    }

    public sealed class MeshFilter : Component
    {
        public Mesh sharedMesh { get; set; }
    }

    public class Renderer : Component
    {
        public bool enabled { get; set; }
        public bool forceRenderingOff { get; set; }
        public Rendering.ShadowCastingMode shadowCastingMode { get; set; }
        public bool receiveShadows { get; set; }
        public Rendering.LightProbeUsage lightProbeUsage { get; set; }
        public Rendering.ReflectionProbeUsage reflectionProbeUsage { get; set; }
        public int sortingOrder { get; set; }
        public Material sharedMaterial { get; set; }
        public void SetPropertyBlock(MaterialPropertyBlock block) { }
    }

    public sealed class MeshRenderer : Renderer { }

    public class Shader : Object
    {
        public static int PropertyToID(string name) => 0;
    }

    public class Material : Object
    {
        public Material(Shader shader) { }
        public void EnableKeyword(string keyword) { }
        public void SetFloat(string name, float value) { }
        public void SetFloat(int id, float value) { }
        public void SetVector(int id, Vector4 value) { }
        public void SetVectorArray(int id, Vector4[] values) { }
    }

    public sealed class MaterialPropertyBlock
    {
        public void Clear() { }
        public void SetTexture(int id, Texture value) { }
        public void SetVector(int id, Vector4 value) { }
        public void SetFloat(int id, float value) { }
    }

    public static class Resources
    {
        public static T Load<T>(string path) where T : Object => default;
    }

    public enum TextureFormat { RGBA32 = 4 }
    public enum FilterMode { Point, Bilinear, Trilinear }
    public enum TextureWrapMode { Repeat, Clamp }
    public enum RenderTextureFormat { ARGB32 = 0 }
    public enum RenderTextureReadWrite { Default, Linear, sRGB }

    public class Texture : Object
    {
        public int width => 0;
        public int height => 0;
        public FilterMode filterMode { get; set; }
        public TextureWrapMode wrapMode { get; set; }
        public int anisoLevel { get; set; }
    }

    public sealed class Texture2D : Texture
    {
        public Texture2D(int width, int height, TextureFormat format, bool mipChain, bool linear) { }
        public static Texture2D whiteTexture => null;
        public Color32[] GetPixels32() => null;
        public void SetPixels32(Color32[] colors) { }
        public void SetPixel(int x, int y, Color color) { }
        public void Apply(bool updateMipmaps, bool makeNoLongerReadable) { }
    }

    public static class ImageConversion
    {
        public static bool LoadImage(this Texture2D tex, byte[] data, bool markNonReadable) => false;
    }

    public sealed class RenderTexture : Texture
    {
        public RenderTexture(int width, int height, int depth, RenderTextureFormat format, RenderTextureReadWrite readWrite) { }
        public bool useMipMap { get; set; }
        public bool Create() => true;
        public bool IsCreated() => true;
        public void Release() { }
    }

    public static class Graphics
    {
        public static void ExecuteCommandBuffer(Rendering.CommandBuffer buffer) { }
    }

    public sealed class Camera : Behaviour
    {
        public static Camera main => null;
        public bool orthographic { get; set; }
        public float orthographicSize { get; set; }
        public Vector3 WorldToScreenPoint(Vector3 p) => p;
        public Vector3 ScreenToWorldPoint(Vector3 p) => p;
    }

    public static class Input
    {
        public static Vector3 mousePosition => default;
        public static bool GetMouseButtonDown(int button) => false;
    }

    public static class Screen
    {
        public static int height => 0;
    }

    public class GUIStyle { }

    public class GUISkin
    {
        public GUIStyle box => null;
    }

    public static class GUI
    {
        public static GUISkin skin => null;
    }

    public class GUILayoutOption { }

    public static class GUILayout
    {
        public static void BeginArea(Rect rect, GUIStyle style) { }
        public static void EndArea() { }
        public static Vector2 BeginScrollView(Vector2 position, params GUILayoutOption[] options) => position;
        public static void EndScrollView() { }
        public static void Label(string text, params GUILayoutOption[] options) { }
        public static bool Toggle(bool value, string text, params GUILayoutOption[] options) => value;
        public static bool Button(string text, params GUILayoutOption[] options) => false;
        public static float HorizontalSlider(float value, float left, float right, params GUILayoutOption[] options) => value;
    }

    [AttributeUsage(AttributeTargets.Field)] public sealed class SerializeField : Attribute { }
    [AttributeUsage(AttributeTargets.Field)] public sealed class HideInInspector : Attribute { }
    [AttributeUsage(AttributeTargets.Field)] public sealed class TooltipAttribute : Attribute { public TooltipAttribute(string t) { } }
    [AttributeUsage(AttributeTargets.Field)] public sealed class RangeAttribute : Attribute { public RangeAttribute(float min, float max) { } }
    [AttributeUsage(AttributeTargets.Field)] public sealed class ColorUsageAttribute : Attribute { public ColorUsageAttribute(bool showAlpha, bool hdr) { } }
    [AttributeUsage(AttributeTargets.Class)] public sealed class ExecuteAlways : Attribute { }
    [AttributeUsage(AttributeTargets.Class)] public sealed class DisallowMultipleComponent : Attribute { }
    [AttributeUsage(AttributeTargets.Class)] public sealed class AddComponentMenu : Attribute { public AddComponentMenu(string menu) { } }
    [AttributeUsage(AttributeTargets.Class, AllowMultiple = true)] public sealed class RequireComponent : Attribute { public RequireComponent(Type t) { } }
    [AttributeUsage(AttributeTargets.Class)]
    public sealed class CreateAssetMenuAttribute : Attribute
    {
        public string menuName { get; set; }
        public string fileName { get; set; }
    }
}

namespace UnityEngine.Rendering
{
    public enum BlendMode { Zero = 0, One = 1, DstColor = 2, SrcColor = 3, OneMinusDstColor = 4, SrcAlpha = 5, OneMinusSrcColor = 6, DstAlpha = 7, OneMinusDstAlpha = 8, SrcAlphaSaturate = 9, OneMinusSrcAlpha = 10 }
    public enum ShadowCastingMode { Off, On }
    public enum LightProbeUsage { Off, BlendProbes }
    public enum ReflectionProbeUsage { Off, BlendProbes }

    public sealed class SortingGroup : Behaviour { }

    public struct RenderTargetIdentifier
    {
        public static implicit operator RenderTargetIdentifier(int nameID) => default;
        public static implicit operator RenderTargetIdentifier(Texture tex) => default;
    }

    public sealed class CommandBuffer : IDisposable
    {
        public string name { get; set; }
        public void Clear() { }
        public void SetRenderTarget(RenderTargetIdentifier rt) { }
        public void SetViewProjectionMatrices(Matrix4x4 view, Matrix4x4 proj) { }
        public void SetViewport(Rect pixelRect) { }
        public void ClearRenderTarget(bool clearDepth, bool clearColor, Color backgroundColor) { }
        public void GetTemporaryRT(int nameID, int width, int height, int depthBuffer, FilterMode filter, RenderTextureFormat format) { }
        public void ReleaseTemporaryRT(int nameID) { }
        public void SetGlobalTexture(int nameID, RenderTargetIdentifier value) { }
        public void DrawMesh(Mesh mesh, Matrix4x4 matrix, Material material, int submeshIndex, int shaderPass, MaterialPropertyBlock properties) { }
        public void Release() { }
        public void Dispose() { }
    }
}

namespace UnityEditor
{
    using UnityEngine;

    public static class EditorApplication
    {
        public static void QueuePlayerLoopUpdate() { }
    }

    public static class AssetDatabase
    {
        public static string GetAssetPath(Object o) => null;
        public static bool IsValidFolder(string path) => false;
        public static string[] FindAssets(string filter, string[] folders) => null;
        public static string GUIDToAssetPath(string guid) => null;
        public static T LoadAssetAtPath<T>(string path) where T : Object => default;
    }

    public sealed class MonoScript : Object
    {
        public static MonoScript FromMonoBehaviour(MonoBehaviour b) => null;
    }

    public class Editor : ScriptableObject
    {
        public Object target => null;
        public virtual void OnInspectorGUI() { }
        public bool DrawDefaultInspector() => false;
    }

    [AttributeUsage(AttributeTargets.Class)] public sealed class CustomEditor : Attribute { public CustomEditor(Type t) { } }

    public enum MessageType { None, Info, Warning, Error }

    public static class EditorGUILayout
    {
        public static void Space() { }
        public static void LabelField(string label, params GUILayoutOption[] options) { }
        public static void LabelField(string label, string label2, params GUILayoutOption[] options) { }
        public static void HelpBox(string message, MessageType type) { }
        public static bool Foldout(bool foldout, string content, bool toggleOnLabelClick) => foldout;
        public static float Slider(string label, float value, float left, float right, params GUILayoutOption[] options) => value;

        public sealed class HorizontalScope : IDisposable
        {
            public HorizontalScope(params GUILayoutOption[] options) { }
            public void Dispose() { }
        }
    }

    public static class EditorGUI
    {
        public static void BeginChangeCheck() { }
        public static bool EndChangeCheck() => false;
    }

    public static class Undo
    {
        public static void RecordObject(Object o, string name) { }
    }

    public static class EditorUtility
    {
        public static void SetDirty(Object o) { }
    }
}

namespace UnityEditor.AssetImporters
{
    using UnityEngine;

    public abstract class ScriptedImporter : Object
    {
        public abstract void OnImportAsset(AssetImportContext ctx);
    }

    [AttributeUsage(AttributeTargets.Class)]
    public sealed class ScriptedImporterAttribute : Attribute
    {
        public ScriptedImporterAttribute(int version, string ext) { }
    }

    public sealed class AssetImportContext
    {
        public string assetPath => null;
        public void AddObjectToAsset(string identifier, Object obj) { }
        public void SetMainObject(Object obj) { }
        public void LogImportWarning(string msg) { }
        public void LogImportError(string msg) { }
    }
}
