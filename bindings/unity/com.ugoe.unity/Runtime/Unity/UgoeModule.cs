// SPDX-License-Identifier: MIT
using UnityEngine;

namespace Ugoe.Unity
{
    /// <summary>
    /// 着せ替えのモジュール (<c>.ugoemod</c>)。エディタでは <c>.ugoemod</c> のファイルを取り込むとこのアセットになる。
    /// </summary>
    public sealed class UgoeModule : ScriptableObject
    {
        [SerializeField, HideInInspector] private byte[] bytes;

        private Module module;

        /// <summary><c>.ugoemod</c> のバイト列。</summary>
        public byte[] Bytes => bytes;

        public static UgoeModule FromBytes(byte[] data, string name = "Module")
        {
            var asset = CreateInstance<UgoeModule>();
            asset.name = name;
            asset.bytes = data;
            return asset;
        }

        /// <summary>
        /// 読み込んだモジュール (初回に読み、アセットが破棄されるまで共有する)。付けたモジュールは <see cref="Module"/> を
        /// 破棄しても付いたまま残るので、複数の <see cref="UgoePuppet"/> で共有してよい。
        /// </summary>
        public Module Module
        {
            get
            {
                if (module == null || module.Handle.IsClosed) module = Module.FromBytes(bytes);
                return module;
            }
        }

        private void OnDisable()
        {
            module?.Dispose();
            module = null;
        }
    }
}
