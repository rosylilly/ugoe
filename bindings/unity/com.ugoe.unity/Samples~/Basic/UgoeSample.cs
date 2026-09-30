// SPDX-License-Identifier: MIT
// ugoe の見本: モデルを選んで表示し、パラメータのスライダー・表情とモーションのボタン・着せ替えのチェックボックスを出す。
// ライティングを受けるモデルでは、マウスの位置に点光源を置く。触っていない間は AngleX を揺らす。
// クリックした位置の当たり判定をコンソールに出す。
using System.Collections.Generic;
using Ugoe.Unity;
using UnityEngine;

namespace Ugoe.Samples
{
    [RequireComponent(typeof(UgoePuppet))]
    public sealed class UgoeSample : MonoBehaviour
    {
        [Tooltip("選べるモデル。空ならエディタでは、このスクリプトの隣の Models フォルダから探す")]
        public List<UgoeModel> models = new List<UgoeModel>();
        [Tooltip("着せ替えのモジュール (dress-up のモデル用)")]
        public List<UgoeModule> modules = new List<UgoeModule>();
        [Tooltip("モデルの高さを画面の高さの何割にするか")]
        [Range(0.2f, 1)] public float fit = 0.9f;

        private UgoePuppet puppet;
        private int current;
        private Vector2 scroll;
        private float idleTime;
        private float sway;

        private void Awake()
        {
            puppet = GetComponent<UgoePuppet>();
            puppet.ModelLoaded += Fit;
        }

        private void Start()
        {
            if (models.Count > 0 && puppet.Model == null) puppet.Model = models[0];
        }

#if UNITY_EDITOR
        private void OnValidate()
        {
            if (models.Count > 0) return;
            var folder = System.IO.Path.GetDirectoryName(UnityEditor.AssetDatabase.GetAssetPath(
                UnityEditor.MonoScript.FromMonoBehaviour(this)))?.Replace('\\', '/') + "/Models";
            if (!UnityEditor.AssetDatabase.IsValidFolder(folder)) return;
            foreach (var guid in UnityEditor.AssetDatabase.FindAssets("t:UgoeModel", new[] { folder }))
            {
                models.Add(UnityEditor.AssetDatabase.LoadAssetAtPath<UgoeModel>(UnityEditor.AssetDatabase.GUIDToAssetPath(guid)));
            }
            foreach (var guid in UnityEditor.AssetDatabase.FindAssets("t:UgoeModule", new[] { folder }))
            {
                modules.Add(UnityEditor.AssetDatabase.LoadAssetAtPath<UgoeModule>(UnityEditor.AssetDatabase.GUIDToAssetPath(guid)));
            }
        }
#endif

        // モデルの高さが画面に収まるように、1 ユニットあたりのピクセル数を決める (カメラは正射影)。
        private void Fit()
        {
            var cam = Camera.main;
            var p = puppet.Puppet;
            if (cam == null || p == null || !cam.orthographic) return;
            var ppu = p.Canvas.Height / (cam.orthographicSize * 2 * fit);
            if (!Mathf.Approximately(ppu, puppet.PixelsPerUnit))
            {
                puppet.PixelsPerUnit = ppu;
                return;
            }
            // 原点をキャンバスの中央に来るように置く。
            var center = puppet.CanvasToWorld(new Vector2(p.Canvas.Width / 2, p.Canvas.Height / 2)) - transform.position;
            transform.position = new Vector3(-center.x, -center.y, transform.position.z);
        }

        private void Update()
        {
            var p = puppet.Puppet;
            if (p == null) return;

            var cam = Camera.main;
            if (cam != null)
            {
                var world = cam.ScreenToWorldPoint(new Vector3(Input.mousePosition.x, Input.mousePosition.y, -cam.transform.position.z));
                if (p.UsesLighting)
                {
                    var c = puppet.WorldToCanvas(world);
                    puppet.AmbientLight = new Color(0.35f, 0.35f, 0.4f);
                    puppet.SetLights(new[]
                    {
                        new UgoeLight { kind = LightKind.Directional, vector = new Vector3(1, 1, -1), color = new Color(0.4f, 0.4f, 0.4f) },
                        new UgoeLight { kind = LightKind.Point, vector = new Vector3(c.x, c.y, 250), color = new Color(1, 0.9f, 0.8f), range = 900 },
                    });
                }
                if (Input.GetMouseButtonDown(0) && !new Rect(0, 0, 260, Screen.height).Contains(new Vector2(Input.mousePosition.x, Screen.height - Input.mousePosition.y)))
                {
                    Debug.Log($"ugoe hit areas: [{string.Join(", ", puppet.HitAreasAt(world))}]");
                }
            }

            // 触っていない間は AngleX を揺らす。
            idleTime += Time.deltaTime;
            if (idleTime > 2)
            {
                sway += Time.deltaTime;
                puppet.SetParameter("AngleX", Mathf.Sin(sway * 1.3f) * 20);
            }
        }

        private void OnGUI()
        {
            var p = puppet.Puppet;
            GUILayout.BeginArea(new Rect(8, 8, 250, Screen.height - 16), GUI.skin.box);
            scroll = GUILayout.BeginScrollView(scroll);
            GUILayout.Label("Model");
            for (var i = 0; i < models.Count; i++)
            {
                if (models[i] != null && GUILayout.Toggle(current == i, models[i].name) && current != i)
                {
                    current = i;
                    puppet.SetModules(new List<UgoeModule>());
                    puppet.Model = models[i];
                }
            }
            if (models.Count == 0) GUILayout.Label("No models. Put .ugoe files in this sample's Models folder (README).");
            if (p == null)
            {
                GUILayout.EndScrollView();
                GUILayout.EndArea();
                return;
            }

            if (p.SlotIds.Count > 0 && modules.Count > 0)
            {
                GUILayout.Label("Modules");
                foreach (var m in modules)
                {
                    if (m == null) continue;
                    var attached = false;
                    foreach (var name in p.AttachedModuleNames) attached |= name == m.Module.Name;
                    var next = GUILayout.Toggle(attached, m.name);
                    if (next && !attached) puppet.AttachModule(m);
                    if (!next && attached) puppet.DetachModule(m.Module.Name);
                }
            }
            if (p.ExpressionNames.Count > 0)
            {
                GUILayout.Label("Expressions");
                foreach (var e in p.ExpressionNames)
                {
                    if (GUILayout.Button(e)) puppet.SetExpression(e);
                }
            }
            if (p.MotionNames.Count > 0)
            {
                GUILayout.Label("Motions");
                foreach (var m in p.MotionNames)
                {
                    if (GUILayout.Button(m)) puppet.StartMotion(m, MotionPriority.Force);
                }
            }
            GUILayout.Label("Parameters");
            foreach (var info in p.Parameters)
            {
                var value = p.GetParameter(p.FindParameter(info.Id));
                GUILayout.Label($"{info.Id}: {value:0.00}");
                var next = GUILayout.HorizontalSlider(value, info.Min, info.Max);
                if (!Mathf.Approximately(next, value))
                {
                    idleTime = 0;
                    puppet.SetParameter(info.Id, next);
                }
            }
            GUILayout.EndScrollView();
            GUILayout.EndArea();
        }
    }
}
