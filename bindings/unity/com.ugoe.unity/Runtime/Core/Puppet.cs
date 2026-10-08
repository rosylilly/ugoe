// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Ugoe
{
    /// <summary>
    /// 1 体分のモデルの動作単位 (<c>ugoe::Puppet</c> を C ABI 越しに使う)。Unity に依存しない。
    /// 1 つの <see cref="Puppet"/> を複数のスレッドから同時に使ってはならない。
    /// </summary>
    /// <remarks>
    /// 描画可能要素の情報 (<see cref="Drawables"/>) は作ったときと着せ替えの後に取り直す。
    /// 着せ替えに成功すると描画可能要素の数・並び・ID が変わるので、ホストは描画の準備をやり直す
    /// (<see cref="DressUpVersion"/> が増える)。
    /// </remarks>
    public sealed class Puppet : IDisposable
    {
        private readonly PuppetHandle handle;
        private DrawableInfo[] drawables;
        private ParameterInfo[] parameters;
        private Dictionary<string, int> parameterIndex;

        private Puppet(PuppetHandle handle)
        {
            this.handle = handle;
            UgoeException.Check(Native.ugoe_puppet_canvas(handle, out var c, NativeError.Current));
            Canvas = new CanvasInfo(c.Width, c.Height, c.OriginX, c.OriginY);
            MotionNames = ReadNames(NameList.Motions);
            ExpressionNames = ReadNames(NameList.Expressions);
            SlotIds = ReadNames(NameList.Slots);
            UgoeException.Check(Native.ugoe_puppet_load_warning_count(handle, out var warningCount, NativeError.Current));
            LoadWarnings = new LoadWarning[warningCount];
            for (uint i = 0; i < warningCount; i++)
            {
                UgoeException.Check(Native.ugoe_puppet_load_warning(handle, i, out var w, NativeError.Current));
                LoadWarnings[i] = new LoadWarning(Utf8.Decode(w.Code), Utf8.Decode(w.Message));
            }
            Reload();
        }

        /// <summary><c>.ugoe</c> バンドルのバイト列から作る (物理・モーション・表情・テクスチャも読む)。</summary>
        public static Puppet FromBundle(byte[] bytes)
        {
            if (bytes == null) throw new ArgumentNullException(nameof(bytes));
            UgoeRuntime.EnsureCompatible();
            UgoeException.Check(Native.ugoe_puppet_create_from_bundle(bytes, (UIntPtr)bytes.Length, out var ptr, NativeError.Current));
            return Wrap(ptr);
        }

        /// <summary>
        /// 読み込みの上限 <paramref name="limits"/> を指定して、<c>.ugoe</c> バンドルのバイト列から作る。
        /// 上限を超えたら <see cref="UgoeException"/> (<see cref="UgoeResult.LimitExceeded"/>)。ほかは <see cref="FromBundle(byte[])"/> と同じ。
        /// </summary>
        public static Puppet FromBundle(byte[] bytes, UgoeReadLimits limits)
        {
            if (bytes == null) throw new ArgumentNullException(nameof(bytes));
            UgoeRuntime.EnsureCompatible();
            UgoeException.Check(Native.ugoe_puppet_create_from_bundle_with_limits(bytes, (UIntPtr)bytes.Length, ref limits, out var ptr, NativeError.Current));
            return Wrap(ptr);
        }

        /// <summary>モデルの JSON から作る。テクスチャはすべて「画像なし」になる。</summary>
        public static Puppet FromJson(string json)
        {
            if (json == null) throw new ArgumentNullException(nameof(json));
            UgoeRuntime.EnsureCompatible();
            var bytes = System.Text.Encoding.UTF8.GetBytes(json);
            UgoeException.Check(Native.ugoe_puppet_create_from_json(bytes, (UIntPtr)bytes.Length, out var ptr, NativeError.Current));
            return Wrap(ptr);
        }

        // 作った UgoePuppet を包む。情報の読み出しに失敗したら、ファイナライザを待たずに破棄する。
        private static Puppet Wrap(IntPtr ptr)
        {
            var handle = new PuppetHandle(ptr);
            try
            {
                return new Puppet(handle);
            }
            catch
            {
                handle.Dispose();
                throw;
            }
        }

        public void Dispose() => handle.Dispose();

        public bool IsDisposed => handle.IsClosed;

        public CanvasInfo Canvas { get; }

        /// <summary>読み込みの警告 (読み込みは成功しているが、表示が劣化しうること)。</summary>
        public LoadWarning[] LoadWarnings { get; }

        /// <summary>読み込みの警告 <paramref name="index"/> の補足の値 (<c>"texture"</c>・<c>"feature"</c>・<c>"path"</c>)。無ければ null。</summary>
        public string GetLoadWarningField(int index, string key)
        {
            var r = Native.ugoe_puppet_load_warning_field(handle, (uint)index, Utf8.Encode(key), out var value, NativeError.Current);
            if (r == (int)UgoeResult.NotFound) return null;
            UgoeException.Check(r);
            return Utf8.Decode(value);
        }

        /// <summary>1 フレーム進める。<paramref name="dt"/> は前のフレームからの経過秒数。</summary>
        public void Update(float dt) => UgoeException.Check(Native.ugoe_puppet_update(handle, dt, NativeError.Current));

        /// <summary>
        /// 直前の <see cref="Update"/> (か <see cref="Follow"/>) で見た目が変わったか (<c>ugoe_puppet_changed</c>。
        /// docs/spec/evaluation.md#見た目の変化)。false なら、そのフレームの <see cref="FrameSync.Pull"/> と頂点の書き換えを省いてよい。
        /// 比べる相手は最後に true を返したときの出力で、許容誤差を下回る違い (収まりかけた物理の揺れ) は変わっていないとする。
        /// 省いた後に読むときは、変更フラグは直前の <see cref="Update"/> との違いしか表さないので <see cref="FrameSync.PullAll"/> で読む。
        /// </summary>
        public bool Changed()
        {
            UgoeException.Check(Native.ugoe_puppet_changed(handle, out var changed, NativeError.Current));
            return changed != 0;
        }

        // --- パラメータ ---

        /// <summary>パラメータの一覧 (ID・範囲・既定値)。着せ替えで変わることがある。</summary>
        public IReadOnlyList<ParameterInfo> Parameters => parameters;

        /// <summary>ID からパラメータの番号を引く。無ければ -1。</summary>
        public int FindParameter(string id) => id != null && parameterIndex.TryGetValue(id, out var i) ? i : -1;

        public float GetParameter(int index)
        {
            UgoeException.Check(Native.ugoe_puppet_get_parameter(handle, (uint)index, out var value, NativeError.Current));
            return value;
        }

        /// <summary>値を設定する (範囲に丸める)。次の <see cref="Update"/> から、モーションなどを重ねる基準の値になる。</summary>
        public void SetParameter(int index, float value) =>
            UgoeException.Check(Native.ugoe_puppet_set_parameter(handle, (uint)index, value, NativeError.Current));

        /// <summary>ID で設定する。知らない ID なら false。</summary>
        public bool SetParameter(string id, float value)
        {
            var index = FindParameter(id);
            if (index < 0) return false;
            SetParameter(index, value);
            return true;
        }

        /// <summary>ID でまとめて設定する。知らない ID は飛ばし、設定した数を返す。</summary>
        public int SetParameters(IReadOnlyList<string> ids, IReadOnlyList<float> values)
        {
            if (ids.Count != values.Count) throw new ArgumentException("ids and values must have the same length");
            var ptrs = new IntPtr[ids.Count];
            var vals = new float[values.Count];
            try
            {
                for (var i = 0; i < ids.Count; i++)
                {
                    var bytes = Utf8.Encode(ids[i]);
                    ptrs[i] = Marshal.AllocHGlobal(bytes.Length);
                    Marshal.Copy(bytes, 0, ptrs[i], bytes.Length);
                    vals[i] = values[i];
                }
                UgoeException.Check(Native.ugoe_puppet_set_parameters_by_id(handle, ptrs, vals, (UIntPtr)ptrs.Length, out var applied, NativeError.Current));
                return (int)applied;
            }
            finally
            {
                foreach (var p in ptrs)
                {
                    if (p != IntPtr.Zero) Marshal.FreeHGlobal(p);
                }
            }
        }

        /// <summary>リップシンクの音量 (0〜1)。null で入力をやめる (口のパラメータに触らなくなる)。</summary>
        public void SetLipSyncLevel(float? level) =>
            UgoeException.Check(Native.ugoe_puppet_set_lip_sync_level(handle, level ?? -1f, NativeError.Current));

        /// <summary>段を動かすか (既定はすべて動かす)。例: トラッキング中は目パチを止める。</summary>
        public void SetStageEnabled(UpdateStage stage, bool enabled) =>
            UgoeException.Check(Native.ugoe_puppet_set_stage_enabled(handle, (uint)stage, enabled ? (byte)1 : (byte)0, NativeError.Current));

        // --- 受け手 ---

        /// <summary>
        /// パラメータの値 (<see cref="Parameters"/> の順で同じ数) を、すべて基準値として書く (範囲に丸める)。
        /// <see cref="SetParameter(int, float)"/> と違い、前のフレームの出力と同じ値でも基準値にする。
        /// 数が違うか NaN を含めば <see cref="UgoeResult.InvalidArgument"/> (何も変えない)。
        /// </summary>
        public void SetBaseValues(float[] values)
        {
            if (values == null) throw new ArgumentNullException(nameof(values));
            UgoeException.Check(Native.ugoe_puppet_set_base_values(handle, values, (UIntPtr)values.Length, NativeError.Current));
        }

        /// <summary>
        /// 受け手として <paramref name="dt"/> 秒進める (<c>ugoe::Puppet::follow</c>、docs/spec/motion.md#受け手として進める)。
        /// <paramref name="values"/> (<see cref="Parameters"/> の順で同じ数) を基準値として書き、物理 → リップシンク →
        /// パーツの不透明度 → 評価の順に進める。モーション・トラッキング・表情・目パチ・呼吸は通さない (指揮役で済んでいる)。
        /// </summary>
        /// <param name="values">指揮役の物理の段に入る直前のパラメータの値。</param>
        /// <param name="lipSyncLevel">指揮役のリップシンクの音量。null ならリップシンクを当てない。</param>
        /// <param name="lipSyncStrength">指揮役のリップシンクの強さ。</param>
        /// <param name="partOpacities">指揮役のモーションのパーツの不透明度のカーブ (null か空なら無し)。</param>
        /// <param name="dt">経過秒数 (0 以上の有限の値)。</param>
        /// <remarks>
        /// 値の数が違う・NaN を含む・音量が NaN・強さかパーツの不透明度の値と重みが有限でない・<paramref name="dt"/> が負か
        /// 有限でないときは <see cref="UgoeResult.InvalidArgument"/> (何も変えない)。モーションのイベント (<see cref="MotionEvents"/>)
        /// は直前の <see cref="Update"/> のまま。
        /// </remarks>
        public void Follow(float[] values, float? lipSyncLevel, float lipSyncStrength, IReadOnlyList<PartOpacityInput> partOpacities,
            float dt)
        {
            if (values == null) throw new ArgumentNullException(nameof(values));
            var count = partOpacities?.Count ?? 0;
            Native.PartOpacityInput[] parts = null;
            if (count > 0)
            {
                parts = new Native.PartOpacityInput[count];
                for (var i = 0; i < count; i++)
                {
                    var p = partOpacities[i];
                    // 負の番号は「モデルに無い番号」として C ABI に無視させる。
                    parts[i] = new Native.PartOpacityInput { Part = p.Part < 0 ? uint.MaxValue : (uint)p.Part, Value = p.Value, Weight = p.Weight };
                }
            }
            UgoeException.Check(Native.ugoe_puppet_follow(handle, values, (UIntPtr)values.Length, lipSyncLevel ?? -1f,
                lipSyncLevel.HasValue ? lipSyncStrength : 0f, parts, (UIntPtr)count, dt, NativeError.Current));
        }

        // --- 指揮役 (T0454) ---

        /// <summary>
        /// 直前の <see cref="Update"/> (か <see cref="Follow"/>) で、物理の段に入る直前のパラメータの値 (<see cref="Parameters"/> の順。
        /// <c>ugoe::Puppet::values_before_physics</c>)。指揮役が受け手へ配り、受け手は <see cref="Follow"/> の values にそのまま渡す。
        /// 物理の段を止めていても同じ位置の値。<see cref="Update"/> の前は作ったときの値。
        /// </summary>
        public float[] ValuesBeforePhysics()
        {
            var buffer = new float[Parameters.Count];
            var r = Native.ugoe_puppet_values_before_physics(handle, buffer, (UIntPtr)buffer.Length, out var count, NativeError.Current);
            if (r != (int)UgoeResult.BufferTooSmall)
            {
                UgoeException.Check(r);
                return buffer;
            }
            buffer = new float[(int)count];
            UgoeException.Check(Native.ugoe_puppet_values_before_physics(handle, buffer, (UIntPtr)buffer.Length, out _, NativeError.Current));
            return buffer;
        }

        /// <summary>
        /// 直前の <see cref="Update"/> (か <see cref="Follow"/>) でリップシンクの段が当てた音量と強さ (<c>ugoe::Puppet::lip_sync_applied</c>)。
        /// 段を止めていたか音量が無ければ null。受け手は <see cref="Follow"/> に <c>applied?.Level</c> と <c>applied?.Strength ?? 0</c> を渡す。
        /// </summary>
        public LipSyncInput? LipSyncApplied()
        {
            UgoeException.Check(Native.ugoe_puppet_lip_sync_applied(handle, out var level, out var strength, NativeError.Current));
            return level < 0 ? (LipSyncInput?)null : new LipSyncInput(level, strength);
        }

        /// <summary>
        /// 直前の <see cref="Update"/> (か <see cref="Follow"/>) で、モーションのパーツの不透明度のカーブが値を求めたパーツ
        /// (カーブのあるパーツだけ。<c>ugoe::Puppet::part_opacities_applied</c>)。受け手は <see cref="Follow"/> の partOpacities にそのまま渡す。
        /// </summary>
        public PartOpacityInput[] PartOpacitiesApplied()
        {
            var r = Native.ugoe_puppet_part_opacities_applied(handle, null, UIntPtr.Zero, out var count, NativeError.Current);
            if (r != (int)UgoeResult.BufferTooSmall) UgoeException.Check(r);
            var buffer = new Native.PartOpacityInput[(int)count];
            if (buffer.Length > 0)
            {
                UgoeException.Check(Native.ugoe_puppet_part_opacities_applied(handle, buffer, (UIntPtr)buffer.Length, out count, NativeError.Current));
            }
            var result = new PartOpacityInput[(int)count];
            for (var i = 0; i < result.Length; i++) result[i] = new PartOpacityInput((int)buffer[i].Part, buffer[i].Value, buffer[i].Weight);
            return result;
        }

        // --- 描画可能要素 ---

        /// <summary>描画可能要素の評価で変わらない情報 (番号順)。着せ替えの後に取り直される。</summary>
        public IReadOnlyList<DrawableInfo> Drawables => drawables;

        /// <summary>着せ替えに成功するたびに増える。描画の準備をやり直すかの判断に使う。</summary>
        public int DressUpVersion { get; private set; }

        /// <summary>テクスチャ座標 (u, v の組。0〜1、画像の左上が原点)。</summary>
        public float[] GetUvs(int index)
        {
            UgoeException.Check(Native.ugoe_puppet_drawable_uvs(handle, (uint)index, out var ptr, out var len, NativeError.Current));
            var result = new float[(int)len];
            if (result.Length > 0) Marshal.Copy(ptr, result, 0, result.Length);
            return result;
        }

        /// <summary>三角形の頂点番号 (3 つで 1 つの三角形)。</summary>
        public ushort[] GetIndices(int index)
        {
            UgoeException.Check(Native.ugoe_puppet_drawable_indices(handle, (uint)index, out var ptr, out var len, NativeError.Current));
            var raw = new short[(int)len];
            if (raw.Length > 0) Marshal.Copy(ptr, raw, 0, raw.Length);
            var result = new ushort[raw.Length];
            Buffer.BlockCopy(raw, 0, result, 0, raw.Length * sizeof(short));
            return result;
        }

        /// <summary>
        /// 直前の <see cref="Update"/> で評価した、すべての描画可能要素の状態・描画順・頂点の位置を 1 回の呼び出しで写す
        /// (<c>ugoe_puppet_copy_frame</c>)。配列の長さは描画可能要素の数 (<paramref name="positions"/> は頂点の位置の float の数) 以上。
        /// <paramref name="changedPositionsOnly"/> なら、頂点の位置は変更フラグの立った要素の範囲だけを写す。描画順が変わったかを返す。
        /// </summary>
        internal bool CopyFrame(Native.DrawableState[] states, int[] renderOrder, Native.PositionRange[] ranges, float[] positions,
            bool changedPositionsOnly)
        {
            // 写す先の配列を、呼び出しの間だけ固定する (このアセンブリは unsafe を使わない)。
            var pins = new[]
            {
                GCHandle.Alloc(states, GCHandleType.Pinned),
                GCHandle.Alloc(renderOrder, GCHandleType.Pinned),
                GCHandle.Alloc(ranges, GCHandleType.Pinned),
                GCHandle.Alloc(positions, GCHandleType.Pinned),
            };
            try
            {
                var buffers = new Native.FrameBuffers
                {
                    States = pins[0].AddrOfPinnedObject(),
                    RenderOrder = pins[1].AddrOfPinnedObject(),
                    PositionRanges = pins[2].AddrOfPinnedObject(),
                    DrawableCapacity = (UIntPtr)Math.Min(states.Length, Math.Min(renderOrder.Length, ranges.Length)),
                    Positions = pins[3].AddrOfPinnedObject(),
                    PositionsCapacity = (UIntPtr)positions.Length,
                };
                UgoeException.Check(Native.ugoe_puppet_copy_frame(handle, ref buffers, changedPositionsOnly ? (byte)1 : (byte)0,
                    out var copy, NativeError.Current));
                return copy.RenderOrderChanged != 0;
            }
            finally
            {
                foreach (var pin in pins) pin.Free();
            }
        }

        // --- テクスチャ ---

        public int TextureCount
        {
            get
            {
                UgoeException.Check(Native.ugoe_puppet_texture_count(handle, out var count, NativeError.Current));
                return (int)count;
            }
        }

        /// <summary>テクスチャの画像 (エンコードされたまま。PNG は乗算していないアルファ)。</summary>
        public EncodedTexture GetTexture(int index)
        {
            UgoeException.Check(Native.ugoe_puppet_texture(handle, (uint)index, out var t, NativeError.Current));
            return ToEncoded(t);
        }

        /// <summary>テクスチャの法線マップ。無ければ <see cref="EncodedTextureFormat.None"/>。PNG はアルファを掛けずにデコードする。</summary>
        public EncodedTexture GetNormalMap(int index)
        {
            UgoeException.Check(Native.ugoe_puppet_normal_map(handle, (uint)index, out var t, NativeError.Current));
            return ToEncoded(t);
        }

        private static EncodedTexture ToEncoded(in Native.Texture t)
        {
            var format = t.Format <= 2 ? (EncodedTextureFormat)t.Format : EncodedTextureFormat.None;
            if (format == EncodedTextureFormat.None || t.Data == IntPtr.Zero) return new EncodedTexture(EncodedTextureFormat.None, null);
            var data = new byte[(int)t.Len];
            Marshal.Copy(t.Data, data, 0, data.Length);
            return new EncodedTexture(format, data);
        }

        // --- ライティング ---

        /// <summary>ライティングを受けるメッシュがあるか。光源の操作を出すかの判断に使う。</summary>
        public bool UsesLighting
        {
            get
            {
                UgoeException.Check(Native.ugoe_puppet_uses_lighting(handle, out var uses, NativeError.Current));
                return uses != 0;
            }
        }

        // --- モーション・表情 ---

        public IReadOnlyList<string> MotionNames { get; }

        public IReadOnlyList<string> ExpressionNames { get; }

        /// <summary>モーションを開始する。低い優先度で開始しなかったら false。</summary>
        public bool StartMotion(int index, MotionPriority priority = MotionPriority.Normal)
        {
            UgoeException.Check(Native.ugoe_puppet_start_motion(handle, (uint)index, (uint)priority, out var started, NativeError.Current));
            return started != 0;
        }

        /// <summary>名前でモーションを開始する。知らない名前なら <see cref="ArgumentException"/>。</summary>
        public bool StartMotion(string name, MotionPriority priority = MotionPriority.Normal) =>
            StartMotion(IndexOf(MotionNames, name, "motion"), priority);

        public void SetExpression(int index) => UgoeException.Check(Native.ugoe_puppet_set_expression(handle, (uint)index, NativeError.Current));

        public void SetExpression(string name) => SetExpression(IndexOf(ExpressionNames, name, "expression"));

        /// <summary>使用中の表情をすべてフェードアウトさせる。</summary>
        public void StopExpressions() => UgoeException.Check(Native.ugoe_puppet_stop_expressions(handle, NativeError.Current));

        /// <summary>
        /// 直前の <see cref="Update"/> で発火したモーションのイベント (開始した順のモーションごとに、発火した順)。
        /// 発火したモーションの名前と始めた番号 (<see cref="MotionEvent.Motion"/>・<see cref="MotionEvent.Number"/>) も持つ。
        /// </summary>
        public MotionEvent[] MotionEvents()
        {
            UgoeException.Check(Native.ugoe_puppet_motion_event_count(handle, out var count, NativeError.Current));
            // イベントの無いフレーム (ほとんど) では確保しない。
            var result = count == 0 ? Array.Empty<MotionEvent>() : new MotionEvent[count];
            for (uint i = 0; i < count; i++)
            {
                UgoeException.Check(Native.ugoe_puppet_motion_event(handle, i, out var e, NativeError.Current));
                UgoeException.Check(Native.ugoe_puppet_motion_event_source(handle, i, out var source, NativeError.Current));
                result[i] = new MotionEvent(e.Time, Utf8.Decode(e.Value), Utf8.Decode(source.Motion), source.Number);
            }
            return result;
        }

        /// <summary>
        /// 目パチ・呼吸・リップシンクの設定を JSON (モデルの <c>effects</c> と同じ形) で差し替える。省略したフィールドは既定値。
        /// 目パチと呼吸は初めから (同じ設定でも) になり、リップシンクの音量は保つ (docs/spec/motion.md#エフェクト設定の差し替え)。
        /// 読めなければ <see cref="UgoeResult.Format"/>、モデルの読み込みと同じ検査 (有限でない値・範囲の外の値・モデルにないパラメータ) を
        /// 通らなければ <see cref="UgoeResult.InvalidModel"/> (種類は <c>invalid_model</c>、補足の値 <c>errors</c>・<c>error_codes</c>。どちらも何も変えない)。
        /// </summary>
        public void SetEffectsJson(string json)
        {
            if (json == null) throw new ArgumentNullException(nameof(json));
            var bytes = System.Text.Encoding.UTF8.GetBytes(json);
            UgoeException.Check(Native.ugoe_puppet_set_effects_json(handle, bytes, (UIntPtr)bytes.Length, NativeError.Current));
        }

        // --- パーツ・ポーズ ---

        /// <summary>パーツの ID (モデルの並び)。着せ替えで変わることがある。</summary>
        public IReadOnlyList<string> PartIds { get; private set; }

        /// <summary>ID からパーツの番号を引く。無ければ -1。</summary>
        public int FindPart(string id)
        {
            if (id == null) return -1;
            var r = Native.ugoe_puppet_part_index(handle, Utf8.Encode(id), out var index, NativeError.Current);
            if (r == (int)UgoeResult.NotFound) return -1;
            UgoeException.Check(r);
            return (int)index;
        }

        /// <summary>直前の <see cref="Update"/> がパーツ <paramref name="index"/> に書いた不透明度の上書き。</summary>
        public PartOpacity GetPartOpacity(int index)
        {
            UgoeException.Check(Native.ugoe_puppet_part_opacity(handle, (uint)index, out var o, NativeError.Current));
            return new PartOpacity(o.Value, o.Weight);
        }

        /// <summary>
        /// 直前の評価 (<see cref="Update"/>・<see cref="Follow"/>・着せ替え) でのパーツ <paramref name="index"/> の不透明度。
        /// 上書きを当てた自身の値に祖先の値を掛けたもので、そのパーツのメッシュの不透明度に掛かる (docs/spec/evaluation.md)。
        /// </summary>
        public float GetPartEvaluatedOpacity(int index)
        {
            UgoeException.Check(Native.ugoe_puppet_part_evaluated_opacity(handle, (uint)index, out var opacity, NativeError.Current));
            return opacity;
        }

        /// <summary>
        /// ポーズで、パーツ <paramref name="part"/> を持つ要素をそのグループの表示中の要素にする (次の <see cref="Update"/> から
        /// フェードで近づく。そのフレームにモーションが同じグループの要素を要求すれば要求が勝つ。docs/spec/motion.md#ポーズ)。
        /// どの要素のパーツでもない (リンクのパーツ・知らない ID) なら何も変えずに false。
        /// </summary>
        public bool ShowPose(string part)
        {
            if (part == null) throw new ArgumentNullException(nameof(part));
            UgoeException.Check(Native.ugoe_puppet_show_pose(handle, Utf8.Encode(part), out var shown, NativeError.Current));
            return shown != 0;
        }

        /// <summary>
        /// ポーズのグループごとに表示中の要素のパーツの ID (グループの並び。着せ替えで付けたモジュールのグループを含む)。要素の無いグループは null。
        /// <see cref="ShowPose"/> で選んだ要素はすぐに、モーションのパーツの不透明度のカーブの要求で切り替わった要素は次の
        /// <see cref="Update"/> の後に入る (docs/spec/motion.md#ポーズ)。
        /// </summary>
        public string[] GetVisiblePoseParts()
        {
            UgoeException.Check(Native.ugoe_puppet_pose_group_count(handle, out var count, NativeError.Current));
            var result = new string[count];
            for (uint i = 0; i < count; i++)
            {
                var r = Native.ugoe_puppet_pose_visible_part(handle, i, out var part, NativeError.Current);
                if (r == (int)UgoeResult.NotFound) continue;
                UgoeException.Check(r);
                result[i] = PartIds[(int)part];
            }
            return result;
        }

        /// <summary>
        /// ポーズの定義: グループごとの要素のパーツの ID (グループの並びは <see cref="GetVisiblePoseParts"/> と同じで、着せ替えで
        /// 付けたモジュールのグループを含む。要素はモデルの並びで、先頭が初めに表示する要素。リンクのパーツは含まない)。
        /// ポーズを選ぶ欄を作るために使う (docs/spec/motion.md#ポーズ)。要素の無いグループは空の配列。
        /// </summary>
        public string[][] GetPoseEntryParts()
        {
            UgoeException.Check(Native.ugoe_puppet_pose_group_count(handle, out var count, NativeError.Current));
            var result = new string[count][];
            for (uint i = 0; i < count; i++)
            {
                UgoeException.Check(Native.ugoe_puppet_pose_entry_count(handle, i, out var entries, NativeError.Current));
                result[i] = new string[entries];
                for (uint j = 0; j < entries; j++)
                {
                    UgoeException.Check(Native.ugoe_puppet_pose_entry_part(handle, i, j, out var part, NativeError.Current));
                    result[i][j] = PartIds[(int)part];
                }
            }
            return result;
        }

        // --- 物理 ---

        /// <summary>
        /// 物理の設定を JSON (<c>*.physics.json</c> の形) で差し替える。null なら物理を外す。読めなければ
        /// <see cref="UgoeResult.Format"/>、モデルに合わなければ <see cref="UgoeResult.InvalidModel"/> (種類は着せ替えと同じ
        /// <c>invalid_physics</c>、補足の値 <c>errors</c>・<c>error_codes</c>。何も変えない)。
        /// 着せ替えの付け外しをすると、差し替えた設定は失われる。
        /// </summary>
        public void SetPhysicsJson(string json)
        {
            var bytes = json == null ? null : System.Text.Encoding.UTF8.GetBytes(json);
            UgoeException.Check(Native.ugoe_puppet_set_physics_json(handle, bytes, (UIntPtr)(bytes?.Length ?? 0), NativeError.Current));
        }

        /// <summary>
        /// 本体の物理の設定を JSON (<see cref="SetPhysicsJson"/> と同じ形) で差し替える。null なら外す。着せ替えのモジュールを
        /// 付けていれば、モジュールの組と合成し直した設定に差し替え、この後の付け外しでもこの設定から合成する
        /// (docs/spec/modules.md#puppet-での着せ替え)。読めなければ <see cref="UgoeResult.Format"/>、合わなければ
        /// <see cref="UgoeResult.InvalidModel"/> (種類と補足の値は <see cref="SetPhysicsJson"/> と同じ)、合成できなければ <see cref="UgoeResult.DressUp"/> (何も変えない)。
        /// </summary>
        public void SetBasePhysicsJson(string json)
        {
            var bytes = json == null ? null : System.Text.Encoding.UTF8.GetBytes(json);
            UgoeException.Check(Native.ugoe_puppet_set_base_physics_json(handle, bytes, (UIntPtr)(bytes?.Length ?? 0), NativeError.Current));
        }

        /// <summary>今の物理の設定の JSON。物理が無ければ null。</summary>
        public string PhysicsJson() => ReadPhysicsJson(false);

        /// <summary>
        /// 本体の物理の設定の JSON (モジュールのリグを含まない。付けたことがなければ <see cref="PhysicsJson"/> と同じ)。
        /// 物理が無ければ null。モデルを読み直すときに本体の設定を引き継ぐために使う。
        /// </summary>
        public string BasePhysicsJson() => ReadPhysicsJson(true);

        // IL2CPP で P/Invoke の関数をデリゲートにしないよう、bool で呼び分ける。
        private string ReadPhysicsJson(bool basePhysics)
        {
            UIntPtr len;
            var r = basePhysics
                ? Native.ugoe_puppet_base_physics_json(handle, null, UIntPtr.Zero, out len, NativeError.Current)
                : Native.ugoe_puppet_physics_json(handle, null, UIntPtr.Zero, out len, NativeError.Current);
            if (r != (int)UgoeResult.BufferTooSmall) UgoeException.Check(r);
            if ((int)len == 0) return null;
            var buffer = new byte[(int)len];
            UgoeException.Check(basePhysics
                ? Native.ugoe_puppet_base_physics_json(handle, buffer, (UIntPtr)buffer.Length, out len, NativeError.Current)
                : Native.ugoe_puppet_physics_json(handle, buffer, (UIntPtr)buffer.Length, out len, NativeError.Current));
            return System.Text.Encoding.UTF8.GetString(buffer, 0, (int)len);
        }

        /// <summary>物理の粒子を初期姿勢に戻す (次の <see cref="Update"/> で静止姿勢に置き直す)。</summary>
        public void ResetPhysics() => UgoeException.Check(Native.ugoe_puppet_reset_physics(handle, NativeError.Current));

        // --- 着せ替え ---

        /// <summary>本体のモデルのスロットの ID。</summary>
        public IReadOnlyList<string> SlotIds { get; }

        /// <summary>付けているモジュールの名前 (合成の順)。</summary>
        public IReadOnlyList<string> AttachedModuleNames { get; private set; }

        /// <summary>
        /// 直前の着せ替えで画像 (と法線マップ) を設定し直すテクスチャの番号 (昇順)。
        /// <see cref="TextureCount"/> 以上の番号の画像は、もう使われない。
        /// </summary>
        public IReadOnlyList<int> ChangedTextures { get; private set; } = Array.Empty<int>();

        /// <summary>
        /// モジュールを付ける。同じスロットか同じ名前のモジュールが付いていれば差し替える。
        /// 失敗したら <see cref="UgoeException"/> (<see cref="UgoeResult.DressUp"/>) を投げ、何も変えない。
        /// </summary>
        public void AttachModule(Module module)
        {
            UgoeException.Check(Native.ugoe_puppet_attach_module(handle, module.Handle, NativeError.Current));
            AfterDressUp();
        }

        /// <summary>名前でモジュールを外す。付いていなければ種類 <c>"not_attached"</c> の <see cref="UgoeException"/>。</summary>
        public void DetachModule(string name)
        {
            UgoeException.Check(Native.ugoe_puppet_detach_module(handle, Utf8.Encode(name), NativeError.Current));
            AfterDressUp();
        }

        /// <summary>付けるモジュールの組を決める (合成の順)。空なら本体だけに戻す。</summary>
        public void SetModules(IReadOnlyList<Module> modules)
        {
            var ptrs = new IntPtr[modules.Count];
            var added = new bool[modules.Count];
            try
            {
                for (var i = 0; i < modules.Count; i++)
                {
                    modules[i].Handle.DangerousAddRef(ref added[i]);
                    ptrs[i] = modules[i].Handle.DangerousGetHandle();
                }
                UgoeException.Check(Native.ugoe_puppet_set_modules(handle, ptrs, (UIntPtr)ptrs.Length, NativeError.Current));
            }
            finally
            {
                for (var i = 0; i < modules.Count; i++)
                {
                    if (added[i]) modules[i].Handle.DangerousRelease();
                }
            }
            AfterDressUp();
        }

        private void AfterDressUp()
        {
            UgoeException.Check(Native.ugoe_puppet_changed_textures(handle, out var ptr, out var len, NativeError.Current));
            var changed = new int[(int)len];
            if (changed.Length > 0) Marshal.Copy(ptr, changed, 0, changed.Length);
            ChangedTextures = changed;
            Reload();
            DressUpVersion++;
        }

        // --- 当たり判定 ---

        public IReadOnlyList<string> HitAreaIds { get; private set; }

        /// <summary>キャンバス座標の点を含む当たり判定の番号 (手前から順)。直前の <see cref="Update"/> の形で判定する。</summary>
        public int[] HitAreasAt(float x, float y)
        {
            var r = Native.ugoe_puppet_hit_areas_at(handle, x, y, null, UIntPtr.Zero, out var count, NativeError.Current);
            if (r != (int)UgoeResult.BufferTooSmall) UgoeException.Check(r);
            var buffer = new uint[(int)count];
            if (buffer.Length > 0)
            {
                UgoeException.Check(Native.ugoe_puppet_hit_areas_at(handle, x, y, buffer, (UIntPtr)buffer.Length, out count, NativeError.Current));
            }
            var result = new int[(int)count];
            for (var i = 0; i < result.Length; i++) result[i] = (int)buffer[i];
            return result;
        }

        /// <summary>ID の当たり判定が点を含むか。知らない ID なら <see cref="UgoeException"/> (<see cref="UgoeResult.NotFound"/>)。</summary>
        public bool HitTest(string id, float x, float y)
        {
            UgoeException.Check(Native.ugoe_puppet_hit_test(handle, Utf8.Encode(id), x, y, out var hit, NativeError.Current));
            return hit != 0;
        }

        /// <summary>標準の名前 (<c>"Head"</c>・<c>"Body"</c>) に対応する当たり判定の番号。無ければ -1。</summary>
        public int StandardHitArea(string name)
        {
            var r = Native.ugoe_puppet_standard_hit_area(handle, Utf8.Encode(name), out var index, NativeError.Current);
            if (r == (int)UgoeResult.NotFound) return -1;
            UgoeException.Check(r);
            return (int)index;
        }

        /// <summary>標準の名前の当たり判定が点を含むか。対応が無ければ false。</summary>
        public bool HitTestStandard(string name, float x, float y)
        {
            UgoeException.Check(Native.ugoe_puppet_hit_test_standard(handle, Utf8.Encode(name), x, y, out var hit, NativeError.Current));
            return hit != 0;
        }

        // --- トラッキング ---

        /// <summary>フェイストラッキングの入力を設定する。null で入力をやめる (最後に書いた値は残る)。</summary>
        public void SetTracking(TrackingInput input) =>
            UgoeException.Check(Native.ugoe_puppet_set_tracking(handle, input?.ToNative(), NativeError.Current));

        /// <summary>トラッキングが書き込むパラメータの番号 (昇順)。</summary>
        public int[] TrackingTargets()
        {
            var r = Native.ugoe_puppet_tracking_targets(handle, null, UIntPtr.Zero, out var count, NativeError.Current);
            if (r != (int)UgoeResult.BufferTooSmall) UgoeException.Check(r);
            var buffer = new uint[(int)count];
            if (buffer.Length > 0)
            {
                UgoeException.Check(Native.ugoe_puppet_tracking_targets(handle, buffer, (UIntPtr)buffer.Length, out count, NativeError.Current));
            }
            var result = new int[(int)count];
            for (var i = 0; i < result.Length; i++) result[i] = (int)buffer[i];
            return result;
        }

        /// <summary>
        /// トラッキング中に、再生中のモーションが動かすパラメータではモーションを優先するか (既定は false。
        /// docs/spec/motion.md#トラッキング中のモーションの優先)。
        /// </summary>
        public bool MotionOverTracking
        {
            get
            {
                UgoeException.Check(Native.ugoe_puppet_motion_over_tracking(handle, out var enabled, NativeError.Current));
                return enabled != 0;
            }
            set => UgoeException.Check(Native.ugoe_puppet_set_motion_over_tracking(handle, value ? (byte)1 : (byte)0, NativeError.Current));
        }

        /// <summary>
        /// <see cref="MotionOverTracking"/> でもトラッキングのままにするパラメータの番号 (目は除く、など)。null か空で無しに戻す。
        /// 範囲外の番号があれば <see cref="UgoeResult.OutOfRange"/> (何も変えない)。
        /// </summary>
        public void SetTrackingKept(IReadOnlyList<int> indices)
        {
            var count = indices?.Count ?? 0;
            var raw = count > 0 ? new uint[count] : null;
            for (var i = 0; i < count; i++)
            {
                if (indices[i] < 0) throw new ArgumentOutOfRangeException(nameof(indices), indices[i], "parameter index must not be negative");
                raw[i] = (uint)indices[i];
            }
            UgoeException.Check(Native.ugoe_puppet_set_tracking_kept(handle, raw, (UIntPtr)count, NativeError.Current));
        }

        // --- 内部 ---

        // 着せ替えで変わるもの (描画可能要素・パラメータ・当たり判定・パーツ・付けているモジュール) を読み直す。
        private void Reload()
        {
            UgoeException.Check(Native.ugoe_puppet_drawable_count(handle, out var count, NativeError.Current));
            drawables = new DrawableInfo[count];
            for (uint i = 0; i < count; i++)
            {
                UgoeException.Check(Native.ugoe_puppet_drawable_info(handle, i, out var info, NativeError.Current));
                var masks = new MaskRef[info.MaskCount];
                for (uint m = 0; m < info.MaskCount; m++)
                {
                    UgoeException.Check(Native.ugoe_puppet_drawable_mask(handle, i, m, out var mask, NativeError.Current));
                    masks[m] = new MaskRef((int)mask.Drawable, mask.Invert != 0);
                }
                UgoeException.Check(Native.ugoe_puppet_drawable_lit(handle, i, out var lit, NativeError.Current));
                drawables[i] = new DrawableInfo(Utf8.Decode(info.Id), (int)info.Texture, (int)info.VertexCount,
                    (int)info.IndexCount, info.Culling != 0, info.MaskOnly != 0,
                    info.MaskMode == 1 ? MaskMode.Intersection : MaskMode.Union, masks, lit != 0);
            }

            UgoeException.Check(Native.ugoe_puppet_parameter_count(handle, out var paramCount, NativeError.Current));
            parameters = new ParameterInfo[paramCount];
            parameterIndex = new Dictionary<string, int>((int)paramCount);
            for (uint i = 0; i < paramCount; i++)
            {
                UgoeException.Check(Native.ugoe_puppet_parameter_id(handle, i, out var id, NativeError.Current));
                UgoeException.Check(Native.ugoe_puppet_parameter_info(handle, i, out var p, NativeError.Current));
                var name = Utf8.Decode(id);
                parameters[i] = new ParameterInfo(name, p.Min, p.Max, p.Default);
                parameterIndex[name] = (int)i;
            }

            HitAreaIds = ReadNames(NameList.HitAreas);
            PartIds = ReadNames(NameList.Parts);
            AttachedModuleNames = ReadNames(NameList.AttachedModules);
        }

        private enum NameList
        {
            Motions,
            Expressions,
            Slots,
            HitAreas,
            Parts,
            AttachedModules,
        }

        // 名前の一覧を読む (IL2CPP で P/Invoke の関数をデリゲートにしないよう、switch で呼び分ける)。
        private string[] ReadNames(NameList list)
        {
            uint n;
            int r;
            switch (list)
            {
                case NameList.Motions: r = Native.ugoe_puppet_motion_count(handle, out n, NativeError.Current); break;
                case NameList.Expressions: r = Native.ugoe_puppet_expression_count(handle, out n, NativeError.Current); break;
                case NameList.Slots: r = Native.ugoe_puppet_slot_count(handle, out n, NativeError.Current); break;
                case NameList.HitAreas: r = Native.ugoe_puppet_hit_area_count(handle, out n, NativeError.Current); break;
                case NameList.Parts: r = Native.ugoe_puppet_part_count(handle, out n, NativeError.Current); break;
                default: r = Native.ugoe_puppet_attached_module_count(handle, out n, NativeError.Current); break;
            }
            UgoeException.Check(r);
            var result = new string[n];
            for (uint i = 0; i < n; i++)
            {
                IntPtr ptr;
                switch (list)
                {
                    case NameList.Motions: r = Native.ugoe_puppet_motion_name(handle, i, out ptr, NativeError.Current); break;
                    case NameList.Expressions: r = Native.ugoe_puppet_expression_name(handle, i, out ptr, NativeError.Current); break;
                    case NameList.Slots: r = Native.ugoe_puppet_slot_id(handle, i, out ptr, NativeError.Current); break;
                    case NameList.HitAreas: r = Native.ugoe_puppet_hit_area_id(handle, i, out ptr, NativeError.Current); break;
                    case NameList.Parts: r = Native.ugoe_puppet_part_id(handle, i, out ptr, NativeError.Current); break;
                    default: r = Native.ugoe_puppet_attached_module_name(handle, i, out ptr, NativeError.Current); break;
                }
                UgoeException.Check(r);
                result[i] = Utf8.Decode(ptr);
            }
            return result;
        }

        private static int IndexOf(IReadOnlyList<string> names, string name, string what)
        {
            for (var i = 0; i < names.Count; i++)
            {
                if (names[i] == name) return i;
            }
            throw new ArgumentException($"unknown {what}: {name}", nameof(name));
        }
    }
}
