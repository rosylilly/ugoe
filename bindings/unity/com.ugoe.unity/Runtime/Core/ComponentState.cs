// SPDX-License-Identifier: MIT
// UgoePuppet (Runtime/Unity) の状態の扱いのうち、Unity に依存しない判断: 読み直すか・読み込みの途中の失敗・
// 変更フラグから送り直す範囲・マスクを描き直す条件・描き直しの印と LateUpdate の順。
// Unity 無しで確かめるため (tests/ComponentStateTests.cs)。
using System;
using System.Collections.Generic;

namespace Ugoe
{
    /// <summary>キャンバスの 1 ピクセルを何分の 1 ユニットにするか (<c>UgoePuppet.PixelsPerUnit</c>) の範囲。</summary>
    internal static class PixelScale
    {
        internal const float Min = 1e-3f;
        internal const float Default = 100;

        /// <summary>
        /// 正の有限の値にする。<see cref="Min"/> より小さい値 (0 と負を含む) は <see cref="Min"/>、NaN と無限大は
        /// <paramref name="fallback"/> (それも使えなければ <see cref="Default"/>)。
        /// </summary>
        internal static float Sanitize(float value, float fallback)
        {
            if (float.IsNaN(value) || float.IsInfinity(value))
            {
                return fallback >= Min && !float.IsInfinity(fallback) ? fallback : Default;
            }
            return Math.Max(value, Min);
        }
    }

    /// <summary>
    /// 読み込んだときのモデル・モジュールの並び・<c>pixelsPerUnit</c> を覚えておき、インスペクタでの変更で読み直すかを決める。
    /// <typeparamref name="TModel"/> と <typeparamref name="TModule"/> は Unity のアセットで、<see cref="object.Equals(object)"/> で比べる。
    /// </summary>
    internal sealed class LoadedSource<TModel, TModule>
    {
        private TModel model;
        private readonly List<TModule> modules = new List<TModule>();
        private float pixelsPerUnit = float.NaN;

        /// <summary>読み込んだ (読み直した) ときに呼ぶ。</summary>
        internal void Remember(TModel loadedModel, IReadOnlyList<TModule> loadedModules, float loadedPixelsPerUnit)
        {
            model = loadedModel;
            pixelsPerUnit = loadedPixelsPerUnit;
            RememberModules(loadedModules);
        }

        /// <summary>着せ替えでモジュールの並びだけが変わったときに呼ぶ (付けたものは読み直さない)。</summary>
        internal void RememberModules(IReadOnlyList<TModule> list)
        {
            modules.Clear();
            for (var i = 0; i < list.Count; i++) modules.Add(list[i]);
        }

        /// <summary>
        /// 読み直すか。モデル・モジュールの並び (順も合成の順なので比べる)・<c>pixelsPerUnit</c> のどれかが変わったら true。
        /// <c>pixelsPerUnit</c> は頂点の変換を変えるので、頂点の動かない要素も送り直すために読み直す (T0426)。
        /// ほかの変更 (パラメータなど) は読み直さずに設定し直す。
        /// </summary>
        internal bool NeedsReload(TModel current, IReadOnlyList<TModule> currentModules, float currentPixelsPerUnit)
        {
            if (!EqualityComparer<TModel>.Default.Equals(current, model)) return true;
            // NaN (まだ読み込んでいない) は何とも等しくない。
            if (!(currentPixelsPerUnit == pixelsPerUnit)) return true;
            if (currentModules.Count != modules.Count) return true;
            for (var i = 0; i < modules.Count; i++)
            {
                if (!EqualityComparer<TModule>.Default.Equals(currentModules[i], modules[i])) return true;
            }
            return false;
        }
    }

    /// <summary>読み込みのどこで失敗したか (<see cref="PuppetLoad.Run"/>)。</summary>
    internal enum LoadStage
    {
        /// <summary>本体を作れない。何も表示しない。</summary>
        Create,
        /// <summary>モジュールを読めない (壊れた <c>.ugoemod</c>)。そのモジュールだけを飛ばす。</summary>
        ReadModule,
        /// <summary>モジュールを付けられない。本体だけを表示する。</summary>
        AttachModules,
        /// <summary>付けた後の準備 (パラメータ・テクスチャ・描画) の途中。何も表示しない。</summary>
        Finish,
    }

    /// <summary>読み込みの手順と、途中で失敗したときに何を残すか。</summary>
    internal static class PuppetLoad
    {
        /// <summary>
        /// 本体を作り、読めるモジュールを付け、<paramref name="finish"/> で表示の準備をする。
        /// 失敗は <paramref name="error"/> に知らせる (番号は <see cref="LoadStage.ReadModule"/> ではモジュールの番号、ほかは -1)。
        /// 本体か準備に失敗したら、作りかけの <see cref="Puppet"/> を破棄して null を返す
        /// (残すと、毎フレームの評価が作りかけの描画で失敗し続ける。T0426)。
        /// </summary>
        /// <param name="read">モジュールを読む。null を返したものは飛ばす (外したアセットなど)。</param>
        internal static Puppet Run<TModule>(Func<Puppet> create, IReadOnlyList<TModule> modules, Func<TModule, Module> read,
            Action<Puppet> finish, Action<LoadStage, int, Exception> error)
        {
            Puppet puppet;
            try
            {
                puppet = create();
            }
            catch (Exception e)
            {
                error(LoadStage.Create, -1, e);
                return null;
            }
            var attach = new List<Module>();
            for (var i = 0; i < modules.Count; i++)
            {
                try
                {
                    var module = read(modules[i]);
                    if (module != null) attach.Add(module);
                }
                catch (Exception e)
                {
                    error(LoadStage.ReadModule, i, e);
                }
            }
            if (attach.Count > 0)
            {
                try
                {
                    puppet.SetModules(attach);
                }
                catch (Exception e)
                {
                    error(LoadStage.AttachModules, -1, e);
                }
            }
            try
            {
                finish(puppet);
            }
            catch (Exception e)
            {
                puppet.Dispose();
                error(LoadStage.Finish, -1, e);
                return null;
            }
            return puppet;
        }
    }

    /// <summary>
    /// 評価の後で、まとめたメッシュ 1 つ (<see cref="DrawBatch"/>) に送り直す範囲 (docs/spec/rendering.md の Skipping unchanged work)。
    /// 毎フレーム確保しないよう、バッチごとに 1 つ持って使い回す。
    /// </summary>
    internal sealed class BatchUpload
    {
        /// <summary>頂点の色と頂点属性で渡す値の変更フラグ。</summary>
        internal const DrawableChanges ColorChanges =
            DrawableChanges.Opacity | DrawableChanges.MultiplyColor | DrawableChanges.ScreenColor | DrawableChanges.MaskStrength;

        /// <summary>要素ごとの (<see cref="DrawBatch.Members"/> と同じ並び)、頂点を写し直すか。</summary>
        internal bool[] MemberPositions { get; private set; } = Array.Empty<bool>();
        /// <summary>要素ごとの、色と頂点属性を写し直すか。</summary>
        internal bool[] MemberColors { get; private set; } = Array.Empty<bool>();
        /// <summary>メッシュの頂点 (とキャンバス座標) を送るか。</summary>
        internal bool Vertices { get; private set; }
        /// <summary>メッシュの色と頂点属性を送るか。</summary>
        internal bool Colors { get; private set; }
        /// <summary>三角形を書き直すか。</summary>
        internal bool Indices { get; private set; }

        /// <summary>送り直す範囲を決める。</summary>
        /// <param name="positionsChanged">要素ごとの、頂点位置を取り出したか (<see cref="FrameSync.PositionsChanged"/>)。</param>
        /// <param name="full">すべてを送るか (作った直後とバッチを作り直した後。頂点の数が変わる)。</param>
        /// <param name="allColors">すべての要素の色と頂点属性を送るか (マスクのページの配置が変わったとき)。</param>
        internal void Plan(DrawBatch batch, IReadOnlyList<DrawableInfo> drawables, IReadOnlyList<bool> positionsChanged,
            IReadOnlyList<DrawableState> states, bool full, bool allColors)
        {
            var members = batch.Members;
            if (MemberPositions.Length != members.Length)
            {
                MemberPositions = new bool[members.Length];
                MemberColors = new bool[members.Length];
            }
            allColors |= full;
            bool vertices = full, colors = allColors, indices = full;
            for (var k = 0; k < members.Length; k++)
            {
                var i = members[k];
                var changes = states[i].Changes;
                var positions = full || positionsChanged[i];
                if (positions)
                {
                    vertices = true;
                    // 裏面カリングする要素は、頂点が動くと表を向く三角形が変わる。
                    if (drawables[i].Culling) indices = true;
                }
                var color = allColors || (changes & ColorChanges) != 0;
                if (color) colors = true;
                if ((changes & DrawableChanges.Visibility) != 0) indices = true;
                MemberPositions[k] = positions;
                MemberColors[k] = color;
            }
            Vertices = vertices;
            Colors = colors;
            Indices = indices;
        }
    }

    /// <summary>評価の後で、バッチとマスクを作り直す・描き直すかの条件。</summary>
    internal static class SyncRules
    {
        /// <summary>
        /// バッチを決め直すか: まだ決めていない・描画順が変わった・まとめるかの設定が変わった・合成モードが変わった
        /// (<see cref="DrawBatchPlan.NeedsRebuild"/>)。
        /// </summary>
        internal static bool NeedsBatchRebuild(DrawBatchPlan plan, bool renderOrderChanged, bool batching, bool batchingUsed,
            IReadOnlyList<DrawableState> states) =>
            plan == null || renderOrderChanged || batching != batchingUsed || plan.NeedsRebuild(states);

        /// <summary>
        /// マスクの内容が変わりうるか。マスク用メッシュかマスクを読む要素の頂点が動いた (内容は読む要素の範囲に描く) か、
        /// マスクを読むマスク用メッシュの効き具合が変わった (入れ子の内容に効く) とき。
        /// </summary>
        internal static bool MaskContentChanged(MaskPlan plan, IReadOnlyList<bool> positionsChanged, IReadOnlyList<DrawableState> states)
        {
            for (var i = 0; i < positionsChanged.Count; i++)
            {
                var source = plan.IsMaskSource[i];
                var reads = plan.ContentOf[i] >= 0;
                if (positionsChanged[i] && (source || reads)) return true;
                if ((states[i].Changes & DrawableChanges.MaskStrength) != 0 && source && reads) return true;
            }
            return false;
        }

        /// <summary>
        /// マスクを描き直すか。マスクがあり、内容が変わりうる・解像度 (<paramref name="scale"/>) が前に描いたとき
        /// (<paramref name="lastScale"/>) と違う・ページが失われた (デバイスの作り直しなど) のどれか。
        /// </summary>
        internal static bool NeedsMaskRedraw(bool hasMasks, bool contentChanged, float scale, float lastScale, bool lost) =>
            hasMasks && (contentChanged || lost || !Approximately(scale, lastScale));

        // Unity の Mathf.Approximately と同じ比べ方。
        private static bool Approximately(float a, float b) =>
            Math.Abs(b - a) < Math.Max(1e-6f * Math.Max(Math.Abs(a), Math.Abs(b)), float.Epsilon * 8);
    }

    /// <summary><see cref="FrameState.LateUpdate"/> が呼ぶ、Unity の側の処理 (<c>UgoePuppet</c> が実装する)。</summary>
    internal interface IFrameHost
    {
        /// <summary>評価する <see cref="Puppet"/> があるか。</summary>
        bool HasPuppet { get; }
        /// <summary>インスペクタでの変更で読み直すか (<see cref="LoadedSource{TModel, TModule}.NeedsReload"/>)。</summary>
        bool NeedsReload();
        /// <summary>シーンに保存したパラメータの値を設定し直す。</summary>
        void ApplySavedParameters();
        /// <summary>読み直す (読み込みの後に <see cref="FrameState.Loaded"/> を呼ぶ)。</summary>
        void Load();
        /// <summary>マスクのページが失われたか (デバイスの作り直しなど)。</summary>
        bool MasksLost { get; }
        /// <summary>時間を進めるか (<c>playing</c> で、再生中か <c>playInEditor</c>)。</summary>
        bool Running { get; }
        /// <summary>このフレームで進める時間 (<c>Time.deltaTime × speed</c>)。</summary>
        float DeltaTime { get; }
        /// <summary>時間を進めて評価し、送る (評価の後に <see cref="FrameState.Evaluated"/> を呼ぶ)。</summary>
        void Advance(float dt);
        /// <summary>光源を材質に設定する (設定の後に <see cref="FrameState.LightsApplied"/> を呼ぶ)。</summary>
        void ApplyLights();
        /// <summary>中間のテクスチャを使うなら、描き直す必要があるときに描く (<see cref="FrameState.TakeIntermediate"/>)。</summary>
        void RenderIntermediate();
    }

    /// <summary>
    /// <c>UgoePuppet</c> の描き直しの印 (読み直す・評価し直す・光源を設定し直す・中間のテクスチャを描き直す) と、
    /// LateUpdate の中の順。印は出来事 (パラメータの変更・インスペクタでの変更・評価など) のメソッドで立て、
    /// <see cref="LateUpdate"/> がそれを見て <see cref="IFrameHost"/> を順に呼ぶ。
    /// </summary>
    internal sealed class FrameState
    {
        /// <summary>次の LateUpdate で読み直すか。</summary>
        internal bool NeedsLoad { get; private set; } = true;
        /// <summary>インスペクタで変えたものを、次の LateUpdate で反映するか (OnValidate の中では子を作れない)。</summary>
        internal bool Validated { get; private set; }
        /// <summary>時間を進めていなくても、次の LateUpdate で評価し直すか (パラメータなどを変えた)。</summary>
        internal bool Dirty { get; private set; }
        /// <summary>次の LateUpdate で光源を材質に設定し直すか。</summary>
        internal bool LightsDirty { get; private set; } = true;
        /// <summary>中間のテクスチャを描き直すか。</summary>
        internal bool IntermediateDirty { get; private set; }
        /// <summary>中間のテクスチャを使っているか (まとめたメッシュは直接描かない)。</summary>
        internal bool IntermediateUsed { get; private set; }

        // --- 出来事 ---

        /// <summary>コンポーネントが有効か無効になった (無効の間に片付けたので、次に読み直す)。</summary>
        internal void Enabled() => NeedsLoad = true;

        /// <inheritdoc cref="Enabled"/>
        internal void Disabled() => NeedsLoad = true;

        /// <summary>インスペクタで値が変わった (OnValidate)。光源も設定し直す。</summary>
        internal void InspectorChanged()
        {
            Validated = true;
            LightsDirty = true;
        }

        /// <summary>読み込みを始めた (成功したかに関わらず、次のフレームでは読み直さない)。</summary>
        internal void Loaded() => NeedsLoad = false;

        /// <summary>描画を作り直した (読み込みと着せ替えの後)。光源を設定し直し、評価し直す。</summary>
        internal void Built()
        {
            LightsDirty = true;
            Dirty = true;
        }

        /// <summary>パラメータ・表情・ポーズなど、評価の結果を変えるものを設定した。</summary>
        internal void Changed() => Dirty = true;

        /// <summary>環境光か光源を変えた。</summary>
        internal void LightsChanged() => LightsDirty = true;

        /// <summary>頂点の変換 (<c>pixelsPerUnit</c>) を変えた。読み込んでいれば、すべてを送り直すために読み直す。</summary>
        internal void PixelScaleChanged(bool hasPuppet)
        {
            if (hasPuppet) NeedsLoad = true;
        }

        /// <summary>評価して送った。中間のテクスチャを描き直す。</summary>
        internal void Evaluated()
        {
            Dirty = false;
            IntermediateDirty = true;
        }

        /// <summary>光源を設定した。材質があって設定したなら、中間のテクスチャを描き直す。</summary>
        internal void LightsApplied(bool materialsChanged)
        {
            LightsDirty = false;
            if (materialsChanged) IntermediateDirty = true;
        }

        /// <summary>中間のテクスチャを使うかを決める。切り替わったら true (描き直す)。</summary>
        internal bool SetIntermediateUsed(bool use)
        {
            if (use == IntermediateUsed) return false;
            IntermediateUsed = use;
            IntermediateDirty = true;
            return true;
        }

        /// <summary>中間のテクスチャを作った (中身は空なので描き直す)。</summary>
        internal void IntermediateCreated() => IntermediateDirty = true;

        /// <summary>
        /// 中間のテクスチャを描き直すか。印が立っているか、テクスチャが失われた (<paramref name="lost"/>) とき true で、印を下ろす。
        /// </summary>
        internal bool TakeIntermediate(bool lost)
        {
            if (!IntermediateDirty && !lost) return false;
            IntermediateDirty = false;
            return true;
        }

        // --- LateUpdate ---

        /// <summary>
        /// LateUpdate の順: インスペクタでの変更を反映する (読み直すか、パラメータを設定し直す) → 読み直す →
        /// (読み込んでいれば) マスクが失われていたら評価し直す印を立てる → 時間を進めるか印が立っていれば評価する →
        /// 光源を設定し直す → 中間のテクスチャを描く。
        /// </summary>
        internal void LateUpdate(IFrameHost host)
        {
            if (Validated)
            {
                Validated = false;
                // モデルかモジュールか頂点の変換 (pixelsPerUnit) が変わったときだけ読み直す。パラメータは設定し直す。
                if (host.NeedsReload()) NeedsLoad = true;
                else if (host.HasPuppet)
                {
                    host.ApplySavedParameters();
                    Dirty = true;
                }
            }
            if (NeedsLoad) host.Load();
            if (!host.HasPuppet) return;
            // マスクのページが失われた (デバイスの作り直しなど) ら、止まっていても描き直す。
            if (host.MasksLost) Dirty = true;
            var run = host.Running;
            if (run || Dirty) host.Advance(run ? ClampDeltaTime(host.DeltaTime) : 0f);
            if (LightsDirty) host.ApplyLights();
            host.RenderIntermediate();
        }

        /// <summary>
        /// 進める時間を 0 以上にする。逆再生はできない (C ABI は負の dt を断る) ので、負の速さは止めているのと同じにする。
        /// NaN も 0 (Unity の <c>Mathf.Max(dt, 0)</c> と同じ)。
        /// </summary>
        internal static float ClampDeltaTime(float dt) => dt > 0 ? dt : 0f;
    }
}
