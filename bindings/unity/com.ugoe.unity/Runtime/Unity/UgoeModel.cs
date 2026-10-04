// SPDX-License-Identifier: MIT
using UnityEngine;

namespace Ugoe.Unity
{
    /// <summary>
    /// <c>.ugoe</c> バンドル。エディタでは <c>.ugoe</c> のファイルを取り込むとこのアセットになる (Editor/UgoeImporters.cs)。
    /// 中身はバイト列のままで、検査は <see cref="Puppet.FromBundle"/> が読むときに行う。
    /// </summary>
    [CreateAssetMenu(menuName = "ugoe/Model (empty)", fileName = "Model")]
    public sealed class UgoeModel : ScriptableObject
    {
        [SerializeField, HideInInspector] private byte[] bytes;

        /// <summary>バンドルのバイト列。</summary>
        public byte[] Bytes => bytes;

        /// <summary>バイト列から作る (実行時に読み込んだファイルなど)。</summary>
        public static UgoeModel FromBytes(byte[] data, string name = "Model")
        {
            var model = CreateInstance<UgoeModel>();
            model.name = name;
            model.bytes = data;
            return model;
        }

        /// <summary>この中身から新しい <see cref="Puppet"/> を作る。呼び出し側が破棄する。</summary>
        public Puppet CreatePuppet() => Puppet.FromBundle(bytes);

        /// <summary>
        /// 読み込みの上限 <paramref name="limits"/> を指定して、この中身から新しい <see cref="Puppet"/> を作る。呼び出し側が破棄する。
        /// <see cref="UgoePuppet"/> と取り込み (Editor/UgoeImporters.cs) は既定の上限で読む (bindings/unity/README.md#読み込みの上限)。
        /// </summary>
        public Puppet CreatePuppet(UgoeReadLimits limits) => Puppet.FromBundle(bytes, limits);
    }
}
