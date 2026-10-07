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
        /// <summary>
        /// ファイルを読めなかった (壊れたバイト列、対応していない版、知らない必須の機能など)。<see cref="UgoeException.Kind"/> は
        /// モデルの読み込み (<see cref="Puppet.FromBundle(byte[])"/>・<see cref="Puppet.FromJson"/> など) では <c>"invalid_file"</c>・<c>"unsupported_version"</c>・
        /// <c>"unsupported_features"</c>・<c>"invalid_model"</c> のどれか、モジュールの読み込み (<see cref="Module.FromBytes(byte[])"/>) では
        /// <c>"invalid_file"</c> (crates/ugoe-ffi/README.md#モデルの読み込みの失敗-t0457)。
        /// </summary>
        Format = 5,
        /// <summary>
        /// モデルの検証やコンパイルに失敗した。モデルの読み込みでは <see cref="UgoeException.Kind"/> が <c>"invalid_model"</c>
        /// (<see cref="UgoeException.Fields"/> の <c>"errors"</c> は検査のエラーの数、<c>"error_codes"</c> はその種類を <c>", "</c> でつないだもの)
        /// か <c>"compile"</c>。物理の設定の差し替え (<see cref="Puppet.SetPhysicsJson"/>・<see cref="Puppet.SetBasePhysicsJson"/>) では
        /// <c>"invalid_physics"</c>、エフェクトの設定の差し替え (<see cref="Puppet.SetEffectsJson"/>) では <c>"invalid_model"</c> (補足の値は同じ)。
        /// </summary>
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
    /// ugoe の C ABI の呼び出しが失敗した。<see cref="Kind"/> と <see cref="Fields"/> は、着せ替えの失敗では wasm の
    /// <c>DressUpError</c> と (crates/ugoe-ffi/README.md#着せ替え-t0409)、モデルの読み込みの失敗では wasm の <c>LoadError</c> と
    /// (crates/ugoe-ffi/README.md#モデルの読み込みの失敗-t0457)、同じ種類と補足の値を持つ。
    /// </summary>
    public sealed class UgoeException : Exception
    {
        /// <summary>知らない値 (将来の版で増えたもの) は数のまま入る。0 以外はすべて失敗。</summary>
        public UgoeResult Result { get; }

        /// <summary>機械が読む種類 (<c>"unknown_slot"</c>・<c>"invalid_file"</c> など)。種類を持たない失敗では null。</summary>
        public string Kind { get; }

        /// <summary>
        /// 補足の値 (着せ替えでは <c>"module"</c>・<c>"slot"</c>・<c>"parameter"</c>・<c>"mask"</c> など、
        /// 読み込みの上限では <c>"limit"</c>・<c>"chunk"</c>・<c>"max"</c>、モデルの読み込みでは <c>"found"</c>・<c>"supported"</c>・
        /// <c>"features"</c>・<c>"errors"</c>・<c>"error_codes"</c> のうち持つもの)。種類ごとのキーは crates/ugoe-wasm/codes.json の
        /// <c>dress_up_error</c> と <c>load_error</c> と <c>settings_error</c> (設定の差し替えの検査の失敗) と同じ。C ABI が列挙する値をすべて入れるので、新しい版で増えたキーも入る。
        /// </summary>
        public IReadOnlyDictionary<string, string> Fields { get; }

        public UgoeException(UgoeResult result, string message, string kind = null, IReadOnlyDictionary<string, string> fields = null)
            : base(message)
        {
            Result = result;
            Kind = kind;
            Fields = fields ?? new Dictionary<string, string>();
        }

        /// <summary>
        /// 結果が失敗なら、このスレッドの <see cref="NativeError.Current"/> (失敗した呼び出しに渡したもの) から例外を作って投げる。
        /// 呼び出しと同じスレッドで、間に await を挟まずに呼ぶ。
        /// </summary>
        internal static void Check(int result)
        {
            if (result != 0) throw FromError(result, NativeError.Current);
        }

        internal static UgoeException FromError(int result, IntPtr error)
        {
            var message = Utf8.Decode(Native.ugoe_error_message(error)) ?? $"ugoe error {result}";
            var kind = Utf8.Decode(Native.ugoe_error_kind(error));
            // 補足の値は C ABI が列挙するものをすべて読む (キーの一覧をここに写さない。T0464)。
            var fields = new Dictionary<string, string>();
            var count = (ulong)Native.ugoe_error_field_count(error);
            for (ulong i = 0; i < count; i++)
            {
                var key = Utf8.Decode(Native.ugoe_error_field_key_at(error, (UIntPtr)i));
                var value = Utf8.Decode(Native.ugoe_error_field_value_at(error, (UIntPtr)i));
                if (key != null && value != null) fields[key] = value;
            }
            return new UgoeException((UgoeResult)result, message, kind, fields);
        }
    }

    /// <summary>
    /// C ABI の失敗しうる関数に渡す <c>UgoeError*</c>。マネージドなスレッドごとに 1 つ作って使い回す。
    /// C ABI の <c>UgoeError</c> はスレッドに結び付かないが、同時に複数のスレッドから使ってはならないので、スレッドごとに分ける
    /// (crates/ugoe-ffi/README.md#失敗の受け取り方-t0489)。呼び出しと <see cref="UgoeException.Check"/> は同じスレッドで続けて行うので、
    /// 失敗した呼び出しに渡したものを読める。スレッドが終わると、<see cref="ErrorHandle"/> のファイナライザが破棄する。
    /// </summary>
    internal static class NativeError
    {
        [ThreadStatic] private static ErrorHandle current;

        /// <summary>このスレッドの <c>UgoeError*</c>。初めて使うときに作る。</summary>
        internal static IntPtr Current
        {
            get
            {
                var error = current;
                if (error == null)
                {
                    error = new ErrorHandle(Native.ugoe_error_create());
                    current = error;
                }
                return error.DangerousGetHandle();
            }
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
