// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;

namespace Ugoe
{
    /// <summary>
    /// 読み込んだ <c>.ugoemod</c> 1 つ (着せ替えのモジュール。<c>UgoeModule</c>)。
    /// <see cref="Puppet.AttachModule"/> で付けたモジュールは、これを破棄しても付いたまま残る (中身は共有される)。
    /// 1 つの <see cref="Module"/> は複数の <see cref="Puppet"/> に付けてよい。
    /// </summary>
    public sealed class Module : IDisposable
    {
        internal readonly ModuleHandle Handle;

        /// <summary>モジュールの名前 (付け外しの鍵)。</summary>
        public string Name { get; }

        /// <summary>付けるスロットの ID。</summary>
        public string Slot { get; }

        /// <summary>読み込みの警告。</summary>
        public LoadWarning[] Warnings { get; }

        private Module(ModuleHandle handle)
        {
            Handle = handle;
            UgoeException.Check(Native.ugoe_module_name(handle, out var name, NativeError.Current));
            Name = Utf8.Decode(name);
            UgoeException.Check(Native.ugoe_module_slot(handle, out var slot, NativeError.Current));
            Slot = Utf8.Decode(slot);
            UgoeException.Check(Native.ugoe_module_warning_count(handle, out var count, NativeError.Current));
            Warnings = new LoadWarning[count];
            for (uint i = 0; i < count; i++)
            {
                UgoeException.Check(Native.ugoe_module_warning(handle, i, out var w, NativeError.Current));
                Warnings[i] = new LoadWarning(Utf8.Decode(w.Code), Utf8.Decode(w.Message));
            }
        }

        /// <summary>
        /// <c>.ugoemod</c> のバイト列から作る。本体との整合 (スロット・パラメータ) は付けるときに検査する。
        /// 読めなければ <see cref="UgoeException"/> (<see cref="UgoeResult.Format"/>、種類 <c>"invalid_file"</c>)。
        /// </summary>
        public static Module FromBytes(byte[] bytes)
        {
            if (bytes == null) throw new ArgumentNullException(nameof(bytes));
            UgoeRuntime.EnsureCompatible();
            UgoeException.Check(Native.ugoe_module_create(bytes, (UIntPtr)bytes.Length, out var ptr, NativeError.Current));
            return Wrap(ptr);
        }

        /// <summary>
        /// 読み込みの上限 <paramref name="limits"/> を指定して、<c>.ugoemod</c> のバイト列から作る。
        /// 上限を超えたら <see cref="UgoeException"/> (<see cref="UgoeResult.LimitExceeded"/>)。ほかは <see cref="FromBytes(byte[])"/> と同じ。
        /// </summary>
        public static Module FromBytes(byte[] bytes, UgoeReadLimits limits)
        {
            if (bytes == null) throw new ArgumentNullException(nameof(bytes));
            UgoeRuntime.EnsureCompatible();
            UgoeException.Check(Native.ugoe_module_create_with_limits(bytes, (UIntPtr)bytes.Length, ref limits, out var ptr, NativeError.Current));
            return Wrap(ptr);
        }

        // 作った UgoeModule を包む。
        private static Module Wrap(IntPtr ptr)
        {
            var handle = new ModuleHandle(ptr);
            try
            {
                return new Module(handle);
            }
            catch
            {
                // 情報の読み出しに失敗したら、ファイナライザを待たずに破棄する。
                handle.Dispose();
                throw;
            }
        }

        /// <summary>警告 <paramref name="index"/> の補足の値 <paramref name="key"/>。無ければ null。</summary>
        public string GetWarningField(int index, string key)
        {
            var r = Native.ugoe_module_warning_field(Handle, (uint)index, Utf8.Encode(key), out var value, NativeError.Current);
            if (r == (int)UgoeResult.NotFound) return null;
            UgoeException.Check(r);
            return Utf8.Decode(value);
        }

        public void Dispose() => Handle.Dispose();
    }

    /// <summary>
    /// 付けるモジュールの並び (合成の順) を、C ABI の <c>ugoe_puppet_attach_module</c> と同じ規則で変える
    /// (<c>ugoe::Puppet::attach_module</c>)。ホストが並びを覚えておき、読み直すときに <see cref="Puppet.SetModules"/> に渡すため。
    /// </summary>
    internal static class ModuleOrder
    {
        /// <summary>
        /// 同じスロットか同じ名前のモジュールがあれば、最初のものの位置に <paramref name="module"/> を入れ、ほかの該当するものを除く。
        /// 無ければ最後に加える。
        /// </summary>
        internal static void Attach<T>(List<T> list, T module, Func<T, string> name, Func<T, string> slot)
        {
            bool Same(T m) => slot(m) == slot(module) || name(m) == name(module);
            var first = list.FindIndex(Same);
            if (first < 0)
            {
                list.Add(module);
                return;
            }
            list[first] = module;
            for (var i = list.Count - 1; i > first; i--)
            {
                if (Same(list[i])) list.RemoveAt(i);
            }
        }
    }

    /// <summary>ライブラリ全体の情報。</summary>
    public static class UgoeRuntime
    {
        private static bool checkedAbi;

        /// <summary>読み込んだネイティブライブラリの版 (<c>"0.1.0"</c> の形)。</summary>
        public static string Version => Utf8.Decode(Native.ugoe_version());

        /// <summary>読み込んだネイティブライブラリの C ABI の版。</summary>
        public static uint AbiVersion => Native.ugoe_abi_version();

        /// <summary>ネイティブライブラリの C ABI の版が、このバインディングと合うかを確かめる (最初の 1 回だけ)。</summary>
        internal static void EnsureCompatible()
        {
            if (checkedAbi) return;
            var abi = Native.ugoe_abi_version();
            if (abi != Native.AbiVersion)
            {
                throw new UgoeException(UgoeResult.InvalidArgument,
                    $"ugoe_ffi ABI version {abi} does not match the binding ({Native.AbiVersion})");
            }
            checkedAbi = true;
        }
    }
}
