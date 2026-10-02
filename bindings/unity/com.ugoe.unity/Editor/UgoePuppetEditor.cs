// SPDX-License-Identifier: MIT
using Ugoe.Unity;
using UnityEditor;
using UnityEngine;

namespace Ugoe.Editor
{
    /// <summary>
    /// <see cref="UgoePuppet"/> のインスペクタ。既定の項目に加えて、読み込んだモデルのパラメータのスライダー
    /// (動かした値はシーンに保存する)、表情とモーションのボタンを出す。
    /// </summary>
    [CustomEditor(typeof(UgoePuppet))]
    public sealed class UgoePuppetEditor : UnityEditor.Editor
    {
        private bool showParameters = true;

        public override void OnInspectorGUI()
        {
            DrawDefaultInspector();
            var target = (UgoePuppet)this.target;
            var puppet = target.Puppet;
            if (puppet == null) return;

            EditorGUILayout.Space();
            EditorGUILayout.LabelField("Drawables", puppet.Drawables.Count.ToString());
            EditorGUILayout.LabelField("Draw calls", target.DrawCalls.ToString());
            if (QualitySettings.activeColorSpace == ColorSpace.Linear)
            {
                if (target.UsesIntermediate)
                {
                    EditorGUILayout.HelpBox("Linear color space: the model is blended in an intermediate texture to match the ugoe reference renderer.", MessageType.Info);
                }
                else
                {
                    EditorGUILayout.HelpBox(UgoePuppet.LinearColorSpaceWarning, MessageType.Warning);
                }
            }
            if (puppet.UsesLighting) EditorGUILayout.HelpBox("This model uses lighting (ambient light and lights above).", MessageType.Info);

            showParameters = EditorGUILayout.Foldout(showParameters, "Parameters", true);
            if (showParameters)
            {
                foreach (var p in puppet.Parameters)
                {
                    var index = puppet.FindParameter(p.Id);
                    var value = puppet.GetParameter(index);
                    EditorGUI.BeginChangeCheck();
                    var next = EditorGUILayout.Slider(p.Id, value, p.Min, p.Max);
                    if (EditorGUI.EndChangeCheck())
                    {
                        Undo.RecordObject(target, "Set ugoe parameter");
                        target.SetSavedParameter(p.Id, next);
                        EditorUtility.SetDirty(target);
                        EditorApplication.QueuePlayerLoopUpdate();
                    }
                }
            }

            if (puppet.ExpressionNames.Count > 0)
            {
                EditorGUILayout.LabelField("Expressions");
                using (new EditorGUILayout.HorizontalScope())
                {
                    foreach (var e in puppet.ExpressionNames)
                    {
                        if (GUILayout.Button(e)) target.SetExpression(e);
                    }
                }
            }
            if (puppet.MotionNames.Count > 0 && Application.isPlaying)
            {
                EditorGUILayout.LabelField("Motions");
                using (new EditorGUILayout.HorizontalScope())
                {
                    foreach (var m in puppet.MotionNames)
                    {
                        if (GUILayout.Button(m)) target.StartMotion(m, MotionPriority.Force);
                    }
                }
            }
        }
    }
}
