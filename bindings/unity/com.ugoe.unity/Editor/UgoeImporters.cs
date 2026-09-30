// SPDX-License-Identifier: MIT
using System;
using System.IO;
using Ugoe.Unity;
using UnityEditor.AssetImporters;
using UnityEngine;

namespace Ugoe.Editor
{
    /// <summary><c>.ugoe</c> を <see cref="UgoeModel"/> として取り込む。読めるかを確かめ、警告を出す。</summary>
    [ScriptedImporter(1, "ugoe")]
    public sealed class UgoeModelImporter : ScriptedImporter
    {
        public override void OnImportAsset(AssetImportContext ctx)
        {
            var bytes = File.ReadAllBytes(ctx.assetPath);
            var model = UgoeModel.FromBytes(bytes, Path.GetFileNameWithoutExtension(ctx.assetPath));
            ctx.AddObjectToAsset("model", model);
            ctx.SetMainObject(model);
            try
            {
                using (var puppet = Puppet.FromBundle(bytes))
                {
                    foreach (var w in puppet.LoadWarnings) ctx.LogImportWarning($"{w}");
                }
            }
            catch (UgoeException e)
            {
                ctx.LogImportError($"could not read {ctx.assetPath}: {e.Message}");
            }
            catch (Exception e) when (e is DllNotFoundException || e is EntryPointNotFoundException)
            {
                ctx.LogImportWarning($"the ugoe native library is not available, so {ctx.assetPath} was not checked: {e.Message}");
            }
        }
    }

    /// <summary><c>.ugoemod</c> を <see cref="UgoeModule"/> として取り込む。</summary>
    [ScriptedImporter(1, "ugoemod")]
    public sealed class UgoeModuleImporter : ScriptedImporter
    {
        public override void OnImportAsset(AssetImportContext ctx)
        {
            var bytes = File.ReadAllBytes(ctx.assetPath);
            var module = UgoeModule.FromBytes(bytes, Path.GetFileNameWithoutExtension(ctx.assetPath));
            ctx.AddObjectToAsset("module", module);
            ctx.SetMainObject(module);
            try
            {
                using (var m = Module.FromBytes(bytes))
                {
                    foreach (var w in m.Warnings) ctx.LogImportWarning($"{w}");
                }
            }
            catch (UgoeException e)
            {
                ctx.LogImportError($"could not read {ctx.assetPath}: {e.Message}");
            }
            catch (Exception e) when (e is DllNotFoundException || e is EntryPointNotFoundException)
            {
                ctx.LogImportWarning($"the ugoe native library is not available, so {ctx.assetPath} was not checked: {e.Message}");
            }
        }
    }
}
