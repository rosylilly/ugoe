// SPDX-License-Identifier: MIT
// ugoe_ffi (crates/ugoe-ffi/include/ugoe.h) の P/Invoke の宣言。Unity に依存しない。
//
// - 構造体はすべて blittable にする (C の bool は byte で受ける)。IL2CPP でもそのまま渡せる。
// - 文字列は UTF-8 の NUL 終端のバイト列で渡す (Utf8.Encode)。返る const char* は IntPtr で受ける。
// - 失敗しうる関数は最後の引数に UgoeError* (IntPtr error) を取る。呼ぶ側は NativeError.Current (このスレッドの UgoeError) を渡し、
//   同じスレッドのまま UgoeException.Check で結果を確かめる (間に await を挟まない)。
// - 関数を足したら、ヘッダ (crates/ugoe-ffi/include/ugoe.h) の宣言と型を合わせる。
// - ヘッダの関数のうち ugoe_puppet_positions (内部のバッファの借用) だけは宣言しない。C# では借りても配列に写すことになり、
//   ugoe_puppet_copy_positions と同じなので (README の「C ABI との対応」)。
using System;
using System.Runtime.InteropServices;

namespace Ugoe
{
    internal static class Native
    {
#if UNITY_IOS && !UNITY_EDITOR
        // iOS は静的ライブラリをアプリにリンクする。
        internal const string Library = "__Internal";
#else
        internal const string Library = "ugoe_ffi";
#endif

        /// <summary>このバインディングが前提とする C ABI の版 (ヘッダの <c>UGOE_ABI_VERSION</c>)。</summary>
        internal const uint AbiVersion = 2;

        internal const int TextureNone = 0;
        internal const int TexturePng = 1;
        internal const int TextureKtx2 = 2;

        // --- 構造体 (ugoe.h と同じ並び) ---

        [StructLayout(LayoutKind.Sequential)]
        internal struct DrawableInfo
        {
            public IntPtr Id;
            public uint Texture;
            public uint VertexCount;
            public uint IndexCount;
            public byte Culling;
            public byte MaskOnly;
            public uint MaskCount;
            public uint MaskMode;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Mask
        {
            public uint Drawable;
            public byte Invert;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct DrawableState
        {
            public float Opacity;
            public float DrawOrder;
            public float MultiplyR, MultiplyG, MultiplyB;
            public float ScreenR, ScreenG, ScreenB;
            public uint BlendMode;
            public float MaskStrength;
            public byte Visible;
            public byte Changes;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Texture
        {
            public uint Format;
            public IntPtr Data;
            public UIntPtr Len;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Warning
        {
            public IntPtr Code;
            public IntPtr Message;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Light
        {
            public uint Kind;
            public float X, Y, Z;
            public float R, G, B;
            public float Range;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct MotionEvent
        {
            public float Time;
            public IntPtr Value;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct MotionEventSource
        {
            public IntPtr Motion;
            public ulong Number;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct PartOpacity
        {
            public float Value;
            public float Weight;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct PartOpacityInput
        {
            public uint Part;
            public float Value;
            public float Weight;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Canvas
        {
            public float Width;
            public float Height;
            public float OriginX;
            public float OriginY;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct ParameterInfo
        {
            public float Min;
            public float Max;
            public float Default;
        }

        /// <summary><c>UgoeLightingUniform</c> の float の数 (ambient 4 + count と padding 4 + 光源 8 × 8)。</summary>
        internal const int LightingUniformFloats = 72;

        /// <summary><c>UgoeTrackingInput</c> の float の数 (頭 3 + 体 3 + 肩 2 + ブレンドシェイプ 52)。</summary>
        internal const int TrackingInputFloats = 60;

        // --- 版とエラー ---

        [DllImport(Library)] internal static extern uint ugoe_abi_version();
        [DllImport(Library)] internal static extern IntPtr ugoe_version();
        [DllImport(Library)] internal static extern IntPtr ugoe_error_create();
        [DllImport(Library)] internal static extern void ugoe_error_destroy(IntPtr error);
        [DllImport(Library)] internal static extern IntPtr ugoe_error_message(IntPtr error);
        [DllImport(Library)] internal static extern IntPtr ugoe_error_kind(IntPtr error);
        [DllImport(Library)] internal static extern IntPtr ugoe_error_field(IntPtr error, byte[] key);
        [DllImport(Library)] internal static extern UIntPtr ugoe_error_field_count(IntPtr error);
        [DllImport(Library)] internal static extern IntPtr ugoe_error_field_key_at(IntPtr error, UIntPtr index);
        [DllImport(Library)] internal static extern IntPtr ugoe_error_field_value_at(IntPtr error, UIntPtr index);

        // --- 作成と破棄 ---

        [DllImport(Library)] internal static extern int ugoe_puppet_create_from_bundle(byte[] bytes, UIntPtr len, out IntPtr puppet, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_create_from_bundle_with_limits(byte[] bytes, UIntPtr len, ref UgoeReadLimits limits, out IntPtr puppet, IntPtr error);
        [DllImport(Library)] internal static extern UgoeReadLimits ugoe_read_limits_default();
        [DllImport(Library)] internal static extern int ugoe_puppet_create_from_json(byte[] json, UIntPtr len, out IntPtr puppet, IntPtr error);
        [DllImport(Library)] internal static extern void ugoe_puppet_destroy(IntPtr puppet);
        [DllImport(Library)] internal static extern int ugoe_puppet_canvas(PuppetHandle puppet, out Canvas canvas, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_update(PuppetHandle puppet, float dt, IntPtr error);

        // --- パラメータ ---

        [DllImport(Library)] internal static extern int ugoe_puppet_parameter_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_parameter_id(PuppetHandle puppet, uint index, out IntPtr id, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_parameter_index(PuppetHandle puppet, byte[] id, out uint index, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_parameter_info(PuppetHandle puppet, uint index, out ParameterInfo info, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_get_parameter(PuppetHandle puppet, uint index, out float value, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_parameter(PuppetHandle puppet, uint index, float value, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_parameters_by_id(PuppetHandle puppet, IntPtr[] ids, float[] values, UIntPtr count, out UIntPtr applied, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_lip_sync_level(PuppetHandle puppet, float value, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_stage_enabled(PuppetHandle puppet, uint stage, byte enabled, IntPtr error);

        // --- 受け手 ---

        [DllImport(Library)] internal static extern int ugoe_puppet_set_base_values(PuppetHandle puppet, float[] values, UIntPtr count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_follow(PuppetHandle puppet, float[] values, UIntPtr count, float lipSyncLevel, float lipSyncStrength, PartOpacityInput[] partOpacities, UIntPtr partOpacityCount, float dt, IntPtr error);

        // --- 指揮役 (T0454) ---

        [DllImport(Library)] internal static extern int ugoe_puppet_values_before_physics(PuppetHandle puppet, [Out] float[] buffer, UIntPtr capacity, out UIntPtr count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_lip_sync_applied(PuppetHandle puppet, out float level, out float strength, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_part_opacities_applied(PuppetHandle puppet, [Out] PartOpacityInput[] buffer, UIntPtr capacity, out UIntPtr count, IntPtr error);

        // --- 描画可能要素 ---

        [DllImport(Library)] internal static extern int ugoe_puppet_drawable_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_drawable_info(PuppetHandle puppet, uint index, out DrawableInfo info, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_drawable_mask(PuppetHandle puppet, uint index, uint mask, out Mask info, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_drawable_state(PuppetHandle puppet, uint index, out DrawableState state, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_copy_positions(PuppetHandle puppet, uint index, [Out] float[] buffer, UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_drawable_uvs(PuppetHandle puppet, uint index, out IntPtr data, out UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_drawable_indices(PuppetHandle puppet, uint index, out IntPtr data, out UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_drawable_lit(PuppetHandle puppet, uint index, out byte lit, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_render_order(PuppetHandle puppet, out IntPtr data, out UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_render_order_changed(PuppetHandle puppet, out byte changed, IntPtr error);

        // --- テクスチャ ---

        [DllImport(Library)] internal static extern int ugoe_puppet_texture_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_texture(PuppetHandle puppet, uint index, out Texture texture, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_normal_map(PuppetHandle puppet, uint index, out Texture texture, IntPtr error);
        [DllImport(Library)] internal static extern UgoeImageLimits ugoe_image_limits_default();
        [DllImport(Library)] internal static extern int ugoe_image_decode(uint format, byte[] data, UIntPtr len, ref UgoeImageLimits limits, byte premultiply, out IntPtr image, IntPtr error);
        [DllImport(Library)] internal static extern void ugoe_image_destroy(IntPtr image);
        [DllImport(Library)] internal static extern int ugoe_image_size(IntPtr image, out uint width, out uint height, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_image_pixels(IntPtr image, out IntPtr data, out UIntPtr len, IntPtr error);

        // --- モーション・表情 ---

        [DllImport(Library)] internal static extern int ugoe_puppet_motion_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_motion_name(PuppetHandle puppet, uint index, out IntPtr name, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_start_motion(PuppetHandle puppet, uint index, uint priority, out byte started, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_expression_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_expression_name(PuppetHandle puppet, uint index, out IntPtr name, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_expression(PuppetHandle puppet, uint index, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_stop_expressions(PuppetHandle puppet, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_motion_event_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_motion_event(PuppetHandle puppet, uint index, out MotionEvent motionEvent, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_motion_event_source(PuppetHandle puppet, uint index, out MotionEventSource source, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_effects_json(PuppetHandle puppet, byte[] json, UIntPtr len, IntPtr error);

        // --- パーツ・ポーズ ---

        [DllImport(Library)] internal static extern int ugoe_puppet_part_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_part_id(PuppetHandle puppet, uint index, out IntPtr id, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_part_index(PuppetHandle puppet, byte[] id, out uint index, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_part_opacity(PuppetHandle puppet, uint index, out PartOpacity opacity, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_part_evaluated_opacity(PuppetHandle puppet, uint index, out float opacity, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_show_pose(PuppetHandle puppet, byte[] part, out byte shown, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_pose_group_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_pose_visible_part(PuppetHandle puppet, uint group, out uint part, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_pose_entry_count(PuppetHandle puppet, uint group, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_pose_entry_part(PuppetHandle puppet, uint group, uint entry, out uint part, IntPtr error);

        // --- 物理 ---

        [DllImport(Library)] internal static extern int ugoe_puppet_set_physics_json(PuppetHandle puppet, byte[] json, UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_base_physics_json(PuppetHandle puppet, byte[] json, UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_physics_json(PuppetHandle puppet, [Out] byte[] buffer, UIntPtr capacity, out UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_base_physics_json(PuppetHandle puppet, [Out] byte[] buffer, UIntPtr capacity, out UIntPtr len, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_reset_physics(PuppetHandle puppet, IntPtr error);

        // --- 着せ替え ---

        [DllImport(Library)] internal static extern int ugoe_module_create(byte[] bytes, UIntPtr len, out IntPtr module, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_module_create_with_limits(byte[] bytes, UIntPtr len, ref UgoeReadLimits limits, out IntPtr module, IntPtr error);
        [DllImport(Library)] internal static extern void ugoe_module_destroy(IntPtr module);
        [DllImport(Library)] internal static extern int ugoe_module_name(ModuleHandle module, out IntPtr name, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_module_slot(ModuleHandle module, out IntPtr slot, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_module_warning_count(ModuleHandle module, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_module_warning(ModuleHandle module, uint index, out Warning warning, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_module_warning_field(ModuleHandle module, uint index, byte[] key, out IntPtr value, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_attach_module(PuppetHandle puppet, ModuleHandle module, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_detach_module(PuppetHandle puppet, byte[] name, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_modules(PuppetHandle puppet, IntPtr[] modules, UIntPtr count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_attached_module_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_attached_module_name(PuppetHandle puppet, uint index, out IntPtr name, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_slot_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_slot_id(PuppetHandle puppet, uint index, out IntPtr id, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_changed_textures(PuppetHandle puppet, out IntPtr data, out UIntPtr len, IntPtr error);

        // --- 当たり判定 ---

        [DllImport(Library)] internal static extern int ugoe_puppet_hit_area_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_hit_area_id(PuppetHandle puppet, uint index, out IntPtr id, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_hit_areas_at(PuppetHandle puppet, float x, float y, [Out] uint[] buffer, UIntPtr capacity, out UIntPtr count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_hit_test(PuppetHandle puppet, byte[] id, float x, float y, out byte hit, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_standard_hit_area(PuppetHandle puppet, byte[] name, out uint index, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_hit_test_standard(PuppetHandle puppet, byte[] name, float x, float y, out byte hit, IntPtr error);

        // --- ライティング ---

        [DllImport(Library)] internal static extern int ugoe_lighting_uniform(float[] ambient, Light[] lights, UIntPtr count, [Out] float[] uniform, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_uses_lighting(PuppetHandle puppet, out byte uses, IntPtr error);

        // --- トラッキング ---

        [DllImport(Library)] internal static extern int ugoe_puppet_set_tracking(PuppetHandle puppet, float[] input, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_tracking_targets(PuppetHandle puppet, [Out] uint[] buffer, UIntPtr capacity, out UIntPtr count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_motion_over_tracking(PuppetHandle puppet, byte enabled, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_motion_over_tracking(PuppetHandle puppet, out byte enabled, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_set_tracking_kept(PuppetHandle puppet, uint[] indices, UIntPtr count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_blend_shape_name(uint index, out IntPtr name, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_blend_shape_index(byte[] name, out uint index, IntPtr error);

        // --- 読み込みの警告 ---

        [DllImport(Library)] internal static extern int ugoe_puppet_load_warning_count(PuppetHandle puppet, out uint count, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_load_warning(PuppetHandle puppet, uint index, out Warning warning, IntPtr error);
        [DllImport(Library)] internal static extern int ugoe_puppet_load_warning_field(PuppetHandle puppet, uint index, byte[] key, out IntPtr value, IntPtr error);
    }

    /// <summary><c>UgoePuppet*</c> の寿命。</summary>
    internal sealed class PuppetHandle : SafeHandle
    {
        internal PuppetHandle(IntPtr handle) : base(IntPtr.Zero, true) => SetHandle(handle);

        public override bool IsInvalid => handle == IntPtr.Zero;

        protected override bool ReleaseHandle()
        {
            Native.ugoe_puppet_destroy(handle);
            return true;
        }
    }

    /// <summary><c>UgoeError*</c> の寿命 (<see cref="NativeError"/>)。</summary>
    internal sealed class ErrorHandle : SafeHandle
    {
        internal ErrorHandle(IntPtr handle) : base(IntPtr.Zero, true) => SetHandle(handle);

        public override bool IsInvalid => handle == IntPtr.Zero;

        protected override bool ReleaseHandle()
        {
            Native.ugoe_error_destroy(handle);
            return true;
        }
    }

    /// <summary><c>UgoeModule*</c> の寿命。</summary>
    internal sealed class ModuleHandle : SafeHandle
    {
        internal ModuleHandle(IntPtr handle) : base(IntPtr.Zero, true) => SetHandle(handle);

        public override bool IsInvalid => handle == IntPtr.Zero;

        protected override bool ReleaseHandle()
        {
            Native.ugoe_module_destroy(handle);
            return true;
        }
    }
}
