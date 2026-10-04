// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Ugoe
{
    /// <summary>C ABI の関数の結果 (<c>UgoeResult</c>。crates/ugoe-ffi/include/ugoe.h)。</summary>
    public enum UgoeResult
    {
        Ok = 0,
        NullPointer = 1,
        InvalidArgument = 2,
        OutOfRange = 3,
        BufferTooSmall = 4,
        /// <summary>ファイルを読めなかった (壊れたバイト列、対応していない版、知らない必須の機能など)。</summary>
        Format = 5,
        /// <summary>モデルの検証やコンパイルに失敗した。</summary>
        InvalidModel = 6,
        NotFound = 7,
        /// <summary>ugoe の内部でパニックが起きた。そのオブジェクトは破棄して作り直す。</summary>
        Panic = 8,
        /// <summary>着せ替えに失敗した (何も変えていない)。種類は <see cref="UgoeException.Kind"/>。</summary>
        DressUp = 9,
        /// <summary>
        /// 読み込みの上限を超えたので、読み込みをやめた (ファイルが壊れているとは限らない)。<see cref="UgoeException.Kind"/> は
        /// <c>"read_limit_exceeded"</c> (crates/ugoe-ffi/README.md#読み込みの上限-t0152)。
        /// </summary>
        LimitExceeded = 10,
    }

    /// <summary>
    /// ugoe の C ABI の呼び出しが失敗した。<see cref="Kind"/> と <see cref="Fields"/> は着せ替えの失敗で、
    /// wasm の <c>DressUpError</c> と同じ種類と補足の値を持つ (crates/ugoe-ffi/README.md#着せ替え-t0409)。
    /// </summary>
    public sealed class UgoeException : Exception
    {
        /// <summary>知らない値 (将来の版で増えたもの) は数のまま入る。0 以外はすべて失敗。</summary>
        public UgoeResult Result { get; }

        /// <summary>機械が読む種類 (<c>"unknown_slot"</c>・<c>"invalid_file"</c> など)。種類を持たない失敗では null。</summary>
        public string Kind { get; }

        /// <summary>
        /// 補足の値 (着せ替えでは <c>"module"</c>・<c>"slot"</c>・<c>"parameter"</c>・<c>"mesh"</c>・<c>"attached"</c>、
        /// 読み込みの上限では <c>"limit"</c>・<c>"chunk"</c>・<c>"max"</c> のうち持つもの)。
        /// </summary>
        public IReadOnlyDictionary<string, string> Fields { get; }

        public UgoeException(UgoeResult result, string message, string kind = null, IReadOnlyDictionary<string, string> fields = null)
            : base(message)
        {
            Result = result;
            Kind = kind;
            Fields = fields ?? new Dictionary<string, string>();
        }

        private static readonly string[] FieldKeys = { "module", "slot", "parameter", "mesh", "attached", "limit", "chunk", "max" };

        /// <summary>結果が失敗なら、同じスレッドの最後のエラーから例外を作って投げる。</summary>
        internal static void Check(int result)
        {
            if (result != 0) throw FromLastError(result);
        }

        internal static UgoeException FromLastError(int result)
        {
            var message = Utf8.Decode(Native.ugoe_last_error()) ?? $"ugoe error {result}";
            var kind = Utf8.Decode(Native.ugoe_last_error_kind());
            var fields = new Dictionary<string, string>();
            if (kind != null)
            {
                foreach (var key in FieldKeys)
                {
                    var value = Utf8.Decode(Native.ugoe_last_error_field(Utf8.Encode(key)));
                    if (value != null) fields[key] = value;
                }
            }
            return new UgoeException((UgoeResult)result, message, kind, fields);
        }
    }

    /// <summary>C ABI との UTF-8 の文字列のやりとり。</summary>
    internal static class Utf8
    {
        /// <summary>NUL 終端の UTF-8 のバイト列にする。途中に NUL があれば (C の側で切れてしまうので) <see cref="ArgumentException"/>。</summary>
        internal static byte[] Encode(string value)
        {
            if (value == null) throw new ArgumentNullException(nameof(value));
            if (value.IndexOf('\0') >= 0) throw new ArgumentException("the string must not contain NUL", nameof(value));
            var bytes = new byte[Encoding.UTF8.GetByteCount(value) + 1];
            Encoding.UTF8.GetBytes(value, 0, value.Length, bytes, 0);
            return bytes;
        }

        /// <summary>NUL 終端の UTF-8 の文字列を読む。NULL なら null。</summary>
        internal static string Decode(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return null;
            var len = 0;
            while (Marshal.ReadByte(ptr, len) != 0) len++;
            var bytes = new byte[len];
            Marshal.Copy(ptr, bytes, 0, len);
            return Encoding.UTF8.GetString(bytes);
        }
    }
}
