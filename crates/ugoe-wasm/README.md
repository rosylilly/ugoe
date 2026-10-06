# ugoe-wasm

**層:** L5 バインディング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

WebAssembly バインディング。Web ビューア・ビューアのアプリ (`apps/viewer`)・エディタのビューポートで使う。
npm のパッケージ `@ugoe/runtime` (と `<ugoe-avatar>`) は `bindings/web/` で、この JS バインディングを包んで配る (T0404)。

## 守ること

- WebGPU がなくても WebGL2 で動くこと
- ブラウザに依存するコードは `src/web.rs` と `src/web/` (wasm32 のときだけビルド) に閉じ込め、
  それ以外 (`load`・`view`・`hit`・`gizmo`・`animation`) はネイティブでテストできるように保つ

## 構成

| モジュール | 内容 |
| --- | --- |
| `web` | JS に公開する `WebPuppet`。canvas に wgpu のサーフェスを作り、クリアと描画を行う |
| `load` | JSON / `.ugoe` バンドルの読み込み (読み込みの上限 `LoadLimits` と、超えたときのコード)、PNG と KTX2 (圧縮していない 8bit の RGB / RGBA と BC1〜BC5・BC7。[ADR 0030](../../docs/adr/0030-compressed-ktx2-decoding.md)) のデコード (乗算済みアルファへ。`ugoe_format::image` を ugoe-headless と共有)、パラメータ値の引き継ぎ |
| `view` | フィット・ズーム・パンと、キャンバス座標 ↔ 画面座標の変換 |
| `tracking` | フェイストラッキングの入力 (ブレンドシェイプの名前と値・頭の回転) から `TrackingInput` を作る。JS への公開は `web/tracking.rs` |
| `hit` | 評価済みの要素の外接矩形とヒットテスト (エディタの選択用。三角形の判定は `ModelInstance::mesh_contains`) |
| `physics_view` | エディタの物理の表示用の問い合わせ: デフォーマの写像とヤコビ行列、粒子の今の位置 (JS への公開は `src/web/physics.rs`) |
| `dress_up` | `.ugoemod` の読み込み、付け外しに合わせたテクスチャの登録し直し (`Wardrobe`)、エラーのコード (JS への公開は `src/web/dress_up.rs`) |
| `lighting` | 光源の JSON (`set_lighting_json`) を `ugoe::render::Lighting` にする (JS への公開は `src/web/lighting.rs`) |
| `frame` | 描画と読み戻し (`render`・`read_pixels`) の失敗のコード (`FrameError`。JS では `RenderError`) |
| `create` | `WebPuppet.create` の失敗のコード (`CreateError`。T0475) |
| `readback` | `drawable_states_json` の中身と、読み戻した画素をストレートアルファの RGBA にする処理 (JS への公開は `src/web/readback.rs`) |
| `animation` | 物理・モーション・表情・エフェクトの JSON の受け渡し、バンドルのモーション・表情の名前の解決、モデルを読み直したときの物理と段の有効無効の引き継ぎ (JS への公開は `src/web/animation.rs`) |

## `WebPuppet` の使い方

```js
import init, { WebPuppet } from "./generated/ugoe_wasm.js";
await init();
const puppet = await WebPuppet.create(canvas, "auto"); // "auto" | "webgpu" | "webgl2"
puppet.load_json(json);           // または load_bundle(bytes)。戻り値は警告の配列 (構造は load_warnings_json())。失敗は LoadError (code 付き)
puppet.resize(canvas.clientWidth * devicePixelRatio, canvas.clientHeight * devicePixelRatio);
puppet.parameters_json();         // 値域・既定値・グループ ({"parameters": [...], "groups": [...]})
puppet.set_parameter("AngleX", 15);
puppet.render();                  // 呼ばれたときだけ描く。ループはホストが回す

// フェイストラッキング (T0703)。変換は Rust の対応付け (ugoe_motion::tracking) で行う
puppet.set_tracking(names, scores, yaw, pitch, roll); // MediaPipe のカテゴリ名と値、頭の回転 (度)
puppet.tracking_parameter_ids();  // トラッキングが書き込むパラメータ
puppet.set_enabled("eye_blink", false); // トラッキング中はモデルの目パチを止める (T0309 の段の切り替え)
puppet.set_motion_over_tracking(true); // 再生中のモーションが動かすパラメータではモーションを優先する (T0721)
puppet.set_tracking_kept(["EyeLOpen", "EyeROpen"]); // 優先しても目はトラッキングのまま (T0722。既定は空)
puppet.clear_tracking();
```

- `set_motion_over_tracking(true)` の間、`update` はトラッキングの値をモーションの前に書き込み、モーションがその上に乗る (`Puppet::set_motion_over_tracking`、`docs/spec/motion.md#トラッキング中のモーションの優先`)。表情・目パチ・呼吸は、今までどおりさらにその上に重なる。C ABI では `ugoe_puppet_set_motion_over_tracking` (T0722)。
- 画面座標は canvas の描画バッファの画素 (CSS ピクセル × `devicePixelRatio`)。
- `set_parameter` は評価を遅らせ、次の `render` や問い合わせでまとめて評価する。物理やモーションを動かすときは `update(dt)` を呼ぶ。
- モデルを読み直すと、同じ ID のパラメータの値は引き継がれる (エディタで編集・Undo のたびに読み直すため)。
- 背景は透明 (既定の `set_background` は透明) で、WebGPU・WebGL2 のどちらでも canvas を乗算済みアルファでページに重ねる。
- `"auto"` は WebGPU のアダプタを実際に要求して判定し、無ければ WebGL2 を使う。1 つの canvas は WebGPU と WebGL2 を併用できないので、作成に失敗したら新しい canvas で作り直す。
  GPU やブラウザの都合で作れないとき (canvas のコンテキストを取れない・失っているなど) は `CreateError` を投げる ([作成の失敗](#作成の失敗-t0475))。
- トラッキングの入力は次の `update(dt)` から毎フレーム適用される (モーションの直後、表情・目パチ・呼吸の前)。
  平滑化・キャリブレーション・回転行列からの角度の計算・鏡像の入れ替えはホストが行う
  (`docs/spec/avatar-profile.md#実装の範囲`)。`clear_tracking()` の後も最後の値は残る。
- `set_motion_over_tracking(true)` の間は、`update` がトラッキングの値をモーションより **先に** 書き込む。
  モーションが動かすパラメータではモーションの値がフェードの重みでトラッキングの値に重なり、ほかのパラメータはトラッキングの値のまま。
  `set_tracking_kept(ids)` (T0722) で選んだパラメータは、優先していてもモーションの後に書き込む (トラッキングのまま。目は除く、など)。
  既定は `false` (仕様の順。トラッキングがモーションの値を置き換える) と空。どちらもモデルを読み直しても保つ。
  T0721 では wasm が `Puppet::update` の前にホストの書き込みとして入れていたが、パラメータごとに分けるために `Puppet` に移した (T0722)。
  ビューアは待機以外の優先度のモーションの再生中だけ入れる。
- `is_lost()` が `true` になったら、新しい canvas で作り直す (以後の `render`・`read_pixels` は `code` が `device_lost` の `RenderError` を投げる。
  [描画と読み戻しの失敗](#描画と読み戻しの失敗-t0465))。WebGPU はデバイスの喪失 (GPU プロセスのクラッシュなど)、
  WebGL2 は canvas の `webglcontextlost` (GPU のリセット・ドライバの更新・タブを裏に回したときの回収など) で `true` になる。
  wgpu の GL のバックエンド (wgpu-hal 30) は WebGL2 の喪失を知らせず描画も失敗しないので、WebGL2 で作ったときは `WebPuppet` が
  canvas の `webglcontextlost` を聞いて印を立てる (リスナーは `free` で外す。既定の動作は止めないので、復元を待つホストは自分で
  `preventDefault` する。T0932)。知らせのイベントは出さないので、ホストは描画のたびか毎フレーム `is_lost()` を確かめる。

### 有限でない値と `dt`

JS から NaN・無限大・負の `dt` を渡したときの扱い (T0433。T0424 のレビューで確かめ、T0137・T0139・T0155・T0158 で SDK とランタイムを直した後のもの)。
ほかの入口 (SDK・C ABI・Godot・Unity) と並べた表と方針は [docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数)。
NaN の書き込みや `dt` で `update` が戻らない・パニックする (wasm ではモジュールが壊れる) ことはない。

| 入口 | NaN | 無限大 | 負の値 |
| --- | --- | --- | --- |
| `update(dt)` | 0 とみなす (何も進めない) | 0 とみなす | 0 とみなす |
| `set_parameter(id, v)` | 書かない (ID があれば `true` を返す) | 値域の端にクランプ | (値域にクランプ) |
| `set_lip_sync_level(v)` | `undefined` と同じ (口に触らない) | 0..1 にクランプ | 0 にクランプ |
| `set_tracking(...)` | 対応付けの式の値が有限でないパラメータには書き込まない | 同じ | (値の意味どおり) |
| `follow(values, dt, level, strength, parts, opacities)` | `values`・`level`・`strength`・`opacities` は例外 (何も変えない)。`dt` は 0 とみなす | `values` は値域にクランプ。`level`・`strength`・`opacities` は例外。`dt` は 0 とみなす | `dt` は 0 とみなす |
| `set_base_values(values)` | 例外 (何も変えない) | 値域にクランプ | (値域にクランプ) |
| `set_view`・`pan_by`・`zoom_at` | 何も変えない | 何も変えない | `set_view` のズームはクランプ、`zoom_at` の倍率が 0 以下なら何も変えない |
| `set_background` | 何も変えない | 0..1 にクランプ | 0 にクランプ |
| `sample_motion_json(json, t)` | 終わりの値 | `+∞` は終わり、`-∞` は始めの値 | 始めの値 |

- `dt` の規則は `Puppet::update`・`Puppet::follow` (`docs/spec/motion.md`)、パラメータの NaN は `ParameterValues::set`
  (`docs/spec/evaluation.md`、公開仕様 `spec/evaluation.md#parameter-values` の「有限でない書き込みは無視してよい」)、
  トラッキングは `docs/spec/avatar-profile.md` に従う。wasm は SDK の扱いをそのまま出し、入口で値を検査し直さない。
- **C ABI との違い:** C ABI は同じ入力を `UGOE_ERROR_INVALID_ARGUMENT` で拒む (`ugoe_puppet_update` は負・有限でない `dt`、
  `ugoe_puppet_set_parameter`・`ugoe_puppet_set_lip_sync_level` は NaN)。wasm は無視する。JS の `update`・`set_parameter`・
  `set_lip_sync_level` は戻り値を見ない `void` 相当の関数として使われ (`requestAnimationFrame` のループやスライダーの `input`)、
  ここで例外を増やすとホストのループが止まる一方、無視しても状態は壊れない (SDK が 0 や「書かない」に揃える) ため。
  C ABI は戻り値の結果コードで誤りを知らせる流儀なので拒む。`follow`・`set_base_values` は両方とも誤りにする
  (指揮役から届いた列の誤りで、受け手が黙って進めると指揮役と食い違うため)。
- 回帰のテストは `src/animation.rs` (`update_with_non_finite_dt_returns_and_stays_finite`・
  `non_finite_parameter_and_lip_sync_writes_keep_state_finite`・`motion_json_is_sampled_without_fades`)。
  `set_parameter` の中身は `load::set_parameter` にあり、ネイティブでテストする。

### 描画結果の読み戻しと評価の結果 (T0418)

```js
const pixels = await puppet.read_pixels(); // Uint8Array。width() × height() のストレートアルファの RGBA、行は上から下
JSON.parse(puppet.drawable_states_json()); // [{opacity, draw_order, multiply_color: [r, g, b], screen_color, mask_strength, visible}]
```

- `read_pixels` は、今の状態を canvas と同じ大きさ・ビュー・背景・光源で canvas とは **別のテクスチャ** に描き、バッファに写して読む
  (canvas には描かない)。canvas の読み戻し (`drawImage`・`toBlob`) はヘッドレスの Chrome の WebGPU で空になることがあったので、
  配布物の適合テスト (`tests/conformance`) は WebGL2・WebGPU のどちらでもこれで画像を取る。スクリーンショットにも使える。
- GPU を失った後と、読み戻しを待つ間に失ったときは、`code` が `device_lost` の `RenderError` で失敗する ([描画と読み戻しの失敗](#描画と読み戻しの失敗-t0465))。
- `drawable_states_json` は `drawable_ids()` の順。頂点は `drawable_positions`、描画の順番は `render_order` (`docs/spec/evaluation.md#8-出力`)。

### 物理・モーション・表情・エフェクト (T0309)

```js
puppet.load_bundle(bytes);             // バンドルの物理・モーション・表情も読む (物理は検査済み)
puppet.motion_names();                 // バンドルのモーションの名前
puppet.motions_json();                 // [{name, duration, looping, fade_in, fade_out, parameters, events}] (T0720)
puppet.start_motion("idle", "idle");   // 優先度は "idle" | "normal" (既定) | "force"。開始しなければ false
puppet.start_motion_json(json);        // *.motion.json の形。タイムラインのプレビュー用
sample_motion_json(json, t);           // (関数) 各カーブの時刻 t の値 (フェードなし) の JSON。タイムラインのシーク用
const sampler = new MotionSampler(json); // 同じモーションを毎フレーム評価するとき (カーブごとの索引を持つ。T0451)
sampler.sample(t);                     // sample_motion_json と同じ形の JSON
sampler.set_motion_json(json);         // モーションを差し替えて索引を作り直す (読めなければ例外で、今のモーションを保つ)
puppet.set_expression("smile");        // または set_expression_json(json)。stop_expressions()
puppet.add_expression("surprised");    // 使用中の表情に重ねる。stop_expression("smile") で名前を指定して止める (T0720)
puppet.expressions_json();             // [{name, fade_in, fade_out, parameters: [{parameter, value, op}]}] (T0720)
puppet.active_expressions();           // 使っている表情の名前 (開始した順。止めてフェードアウト中のものは除く。T0721)
puppet.set_lip_sync_level(0.4);        // 0..1。undefined で口に触らない
puppet.set_enabled("physics", false);  // "physics" | "eye_blink" | "breath" | "lip_sync" | "pose"
puppet.set_physics_json(json);         // 物理だけを差し替える (validate_physics で検査。undefined で外す)
puppet.set_base_physics_json(json);    // 本体の物理を差し替える。付けたモジュールと合成し直し、付け外しの後も保つ (T0416)
puppet.base_physics_json();            // 本体の物理 (モジュールを付ける前の設定)。無ければ undefined
puppet.set_effects_json(json);         // 目パチ・呼吸・リップシンクの設定 (モデルの effects の形) を差し替える
puppet.show_pose("ArmB");              // ポーズの要素 (part の ID) を表示する (T0414)。要素でなければ false
puppet.show_pose_immediately("ArmB");  // 同じだがフェードを飛ばしてすぐに表示する (T0329)
puppet.pose_json();                    // 今のポーズ (モデルの pose の形。付けたモジュールのグループと追従も入る。T0661)
puppet.pose_visible_json();            // 表示中の要素のパーツ (pose_json の groups の並びで string | null の配列。T0478)
puppet.part_evaluated_opacity("ArmB"); // 評価後のパーツの不透明度 (祖先を掛けた値。T0416)。パーツが無ければ undefined
puppet.update(dt);                     // Puppet::update と同じ順で 1 フレーム進める
puppet.motion_events();                // 直前の update で発火したイベントの値
puppet.motion_events_json();           // [{value, time, motion, number}] 発火したモーションの名前と始めた番号も (T0462)
puppet.hit_test_standard("Head", x, y); // Model.avatar.hit_areas の標準の名前で当たり判定
```

- 適用順とスライダー (`set_parameter`) との関係は `docs/spec/motion.md#フレーム内の適用順` と `#手で動かしたパラメータとの関係`。
- `load_json` で読み直すと、物理の設定・モーションと表情の一覧・段の有効無効を引き継ぐ (新しいモデルと合わない物理は外して警告を返す)。`load_bundle` は物理・モーション・表情をバンドルのものに置き換える。
- モーション・表情の再生状態は、モデルを読み直すと初めからになる。
- `motions_json` の `parameters` はカーブが動かすパラメータ (重なりなし)、`events` はイベントの値。`expressions_json` は未知の `op` の項目を出さない。
  `active_expressions` は同じ名前を重ねても 1 つと数える。Rust の `ExpressionManager::players` はフェードアウト中のものも含み、止めたかを
  返さないので、`WebPuppet` が表情の操作 (`set_expression`・`set_expression_json`・`add_expression`・`stop_expression`・`stop_expressions`) を写して
  持ち、再生中の表情が無い名前は除いて返す (`crate::animation::ActiveExpressions`)。モデルを読み直すと空になる。
- `motion_events_json` の `motion`・`number` はイベントを発火したモーションの名前と始めた番号 (`ugoe::Puppet::motion_event_sources`。
  この `WebPuppet` で開始したモーションに 1 から振り、開始しなかった要求では進まない)。クロスフェードでフェードアウト中のモーションの
  イベントはそのモーションのもので、同じ名前を始め直したものは番号で見分ける。`time` はモーションの中の秒。
  **口の形 (T0462):** `motion_events()` の戻り値 (`string[]`) は変えずに関数を足した (使っている利用者とビューアを壊さないため。
  形のある値は `*_json()` で返す既存の口に揃えた)。C ABI では `ugoe_puppet_motion_event`・`ugoe_puppet_motion_event_source`、
  `@ugoe/runtime` では `motionEvents(puppet)` (`UgoeMotionEvent[]`)。
- `set_effects_json` は同じ設定でも目パチ・呼吸を初めからやり直し、リップシンクの音量は保つ。`show_pose` の不透明度は次の `update` からフェードし、
  モーションが同じグループの要素を要求すればそちらが勝つ (`docs/spec/motion.md#エフェクト設定の差し替え`・`#ポーズ`)。C ABI では
  `ugoe_puppet_set_effects_json`・`ugoe_puppet_show_pose` (T0414)。
- `show_pose_immediately` は `show_pose` と同じに要素を選び、そのグループの不透明度を選んだ要素 1・ほかの要素 0 にする
  (`PosePlayer::show_immediately`。描く姿には次の `update` で入る)。次の `update` でモーションが同じグループの要素を要求すれば、
  そこからフェードする。**口の形 (T0329):** `show_pose` に引数を足さず別の関数にした (`show_pose` の意味と型を変えず、
  JS で引数を省いたときにどちらになるかを気にさせないため。Rust の `PosePlayer` の関数とも 1 対 1 になる)。C ABI には無い。
- `pose_visible_json` はグループごとの表示中の要素の `part` (`PosePlayer::visible_parts`。要素の無いグループは `null`)。`show_pose` で
  選んだ要素はフェードを待たずに、モーションのパーツの不透明度のカーブの要求で切り替わった要素は次の `update` の後に入る。ポーズの段を
  止めている間も表示中の要素は変わる。C ABI では `ugoe_puppet_pose_group_count`・`ugoe_puppet_pose_visible_part` (T0478)。`pose_json` の要素のパーツは C ABI では `ugoe_puppet_pose_entry_count`・`ugoe_puppet_pose_entry_part` (T0483)。
- `set_effects_json` は設定をモデルの読み込みと同じに検査する (`Puppet::try_set_effects`。有限でない値 (JSON の `1e39` は `f32` の無限大)・
  範囲の外の値・モデルにないパラメータ)。通らなければ `code` が `"invalid_model"` で `errors` (検査のエラーの数)・`error_codes`
  (検査のエラーの種類を `", "` でつないだ文字列) を持つ `Error` を投げ、何も変えない (C ABI の `ugoe_last_error_kind`・`ugoe_last_error_field`
  と同じ値。T0461・T0469)。読めない JSON の例外は `code` を持たない。
- `set_physics_json`・`set_base_physics_json` の検査の失敗は、`code` が `"invalid_physics"` で `errors`・`error_codes` を持つ `Error`
  (着せ替えの `invalid_physics` と同じ値。C ABI の `ugoe_puppet_set_physics_json`・`ugoe_puppet_set_base_physics_json` と同じ。T0469)。
  読めない JSON の例外は `code` を持たない。検査の失敗の例外は `name` が `"Error"` (`LoadError`・`DressUpError` のような専用の名前を
  持たない)。種類の一覧は [`code` の一覧](#code-の一覧-t0452) の `settings_error`。
- `set_base_physics_json` で付けているモジュールと合成できないときは、付け外しの失敗と同じ `name` が `"DressUpError"` の例外で、`code` と
  補足の値も同じ (`unknown_collider` の `module`・`rig`・`collider` など。下の「着せ替え」。C ABI の `UGOE_ERROR_DRESS_UP` と同じ種類。T0476)。
  `@ugoe/runtime` の `isDressUpError` で見分ける。種類の一覧は `code` の一覧の `dress_up_error`。
- `part_evaluated_opacity` は上書きを当てた自身の値に祖先の値を掛けたもの (`ModelInstance::part_opacity`、`docs/spec/evaluation.md` の 2.)。
  そのパーツのメッシュの不透明度に掛かる。必要なら評価してから読む。C ABI では `ugoe_puppet_part_evaluated_opacity` (T0416)。

### 受け手として進める (T0133・T0134・T0135)・指揮役の値 (T0454)

```js
puppet.parameter_values();              // Float32Array。parameter_ids() の順の現在値
puppet.set_base_values(values);         // Float32Array をまとめて基準値として書く (値域にクランプ。評価は次の render まで遅らせる)
puppet.follow(values, dt, level, strength, parts, opacities); // values を基準値として書き、物理 → リップシンク → パーツの不透明度 → 評価の順に dt 秒進める

// 指揮役の側 (T0454): 直前の update が受け手へ配る値を、follow の引数と同じ形で読む
conductor.update(dt);
const lipSync = conductor.lip_sync_applied();       // Float32Array [音量, 強さ]。当てていなければ undefined
follower.follow(conductor.values_before_physics(), dt, lipSync?.[0], lipSync?.[1],
  conductor.part_opacities_applied_parts(), conductor.part_opacities_applied_values());
```

- `Puppet::set_base_values`・`Puppet::follow` を呼ぶ (`docs/spec/motion.md#受け手として進める`)。スタジオの出力ウィンドウとブラウザソースの
  受け手が、指揮役から届いた `{dt, values, lip_sync, part_opacities}` を順に適用するために使う (ADR 0028 の 3.)。`values` は指揮役の物理の段に入る直前の値
  (`Puppet::values_before_physics`)、`level`・`strength` はリップシンクの音量と強さ (`Puppet::lip_sync_applied`。`level` を省くと当てない、
  `strength` を省くと 1)。リップシンクはこの `WebPuppet` のモデルの対象に当て、`set_lip_sync_level` の音量は使わない。
  `parts` (`Uint32Array`) と `opacities` (`Float32Array`) は指揮役のモーションのパーツの不透明度のカーブ (`Puppet::part_opacities_applied`) で、
  パーツの番号 (モデルの `parts` の順) と、値と重みを交互に並べたもの (`parts` の 2 倍の数。違えば例外)。省くとカーブなし。
  ポーズの要素へのカーブは表示の要求、ほかは不透明度の上書きとして `update` と同じに当てる (T0135)。
- `follow` はこの `WebPuppet` のモーション・トラッキング・表情・目パチ・呼吸を通さない (再生の状態も時刻も進まない)。
  物理・リップシンク・ポーズは `set_enabled` に従う。`motion_events()` は直前の `update` のまま。
- `set_parameter` の値は前のフレームの出力とビット列で同じなら基準値にならないが、`set_base_values` と `follow` は同じでも基準値にする。
- モデルが無い・数が `parameter_ids()` と違う・NaN を含むときは、何も変えずに例外を投げる。モデルに無いパーツの番号は無視する。
  C ABI では `ugoe_puppet_set_base_values`・`ugoe_puppet_follow`。
- 指揮役の側の口 `values_before_physics()`・`lip_sync_applied()`・`part_opacities_applied_parts()` / `part_opacities_applied_values()` は
  `Puppet::values_before_physics`・`lip_sync_applied`・`part_opacities_applied` (T0454)。`parameter_values()` は物理の後の値なので、物理を持つ
  モデルではこちらを配る。直前が `follow` なら渡した値 (値域にクランプしたもの) を返すので、受け手からさらに中継できる。モデルが無ければ空か
  `undefined`。C ABI では `ugoe_puppet_values_before_physics`・`ugoe_puppet_lip_sync_applied` (当てていなければ音量 -1)・
  `ugoe_puppet_part_opacities_applied`。

### エディタ向けの問い合わせ (ギズモ・物理の表示)

```js
puppet.warp_points("Cloth");              // ワープの制御点 (キャンバス座標)
puppet.rotation_frame("Head");            // [ox, oy, ax, ay, bx, by]
puppet.rotation_local_keyform("Head");    // [ox, oy, angle, scale] 補間した局所の値 (親の座標系。T1660)
puppet.deformer_point("Head", 0, -40);    // デフォーマの局所座標の点 → キャンバス座標 [x, y]
puppet.deformer_jacobian("Cloth", 0.5, 0.5); // その点でのヤコビ行列 [a, b, c, d] (列優先。逆写像用)
puppet.physics_particles(0);              // リグ 0 の粒子の今の位置 [x0, y0, ...] (動的物理ならキャンバス座標)
```

- どれも今のパラメータ値 (と最後の `update`) で評価した値を返す (`src/gizmo.rs`・`src/physics_view.rs`)。T0629 (エディタの物理の画面) で追加。
- `rotation_local_keyform` はワールドの変換ではなく、今のパラメータ値でキーフォームを補間した (ブレンドシェイプも足した) 回転デフォーマの
  局所の値 (原点は親の座標系、角度は度・時計回り正、拡大率)。キーの間でも値を返す (エディタのインスペクタがキーの間で補間した今の値を
  押せない欄に出すのに使う。`ModelInstance::rotation_local_keyform`)。回転デフォーマでない・モデルが無ければ `undefined`。

### ライティング (T0210)

```js
puppet.load_bundle(bytes);        // バンドルの法線マップ (NRML) もデコードして登録する (前のモデルの法線マップは外す)
puppet.uses_lighting();           // Model.lighting と Mesh.lit の両方が true のメッシュがあるか (光源の操作を出すかの判断に)
puppet.set_lighting_json(JSON.stringify({
  ambient: [0.3, 0.3, 0.3],       // 省略すると白 [1, 1, 1]
  lights: [
    { type: "directional", direction: [1, 1, -1] },                       // 光が進む向き。color は省略すると白
    { type: "point", position: [512, 300, 200], color: [1.5, 1.2, 1], range: 900 }, // range は省略すると減衰しない
  ],
}));
puppet.set_lighting_json(undefined); // 既定 (環境光が白で光源なし) に戻す
```

- 規則は `docs/spec/rendering.md#ライティング`。座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (手前が +z) で、ビュー (ズーム・パン) の影響を受けない。
- 光源はホストの状態 (背景色と同じ) で、モデルを読み直しても保つ。次の `render` から使う。形の誤り・向きが 0・`range` が正でない・負の色はエラー。
- 光源を設定しなければ、ライティングを受けるメッシュも受けない場合と同じ色で描く。法線マップはアルファを掛けずに読む (デコードしない形式の KTX2 は警告を出して使わない)。

### 着せ替え (T0123)

```js
puppet.load_bundle(bytes);                   // スロットを持つ本体
const name = puppet.load_module(modBytes);   // .ugoemod を読む (付けはしない)。戻り値はモジュールの名前
puppet.attach_module(name);                  // 付ける。同じスロットか同じ名前のモジュールは差し替える
puppet.detach_module(name);                  // 外す
puppet.set_modules(["LongHair", "Dress"]);   // 付ける組をまとめて決める (合成の順)。[] で本体だけ
puppet.unload_module(name);                  // 読み込んだモジュールを捨てる (付けていれば外す)
puppet.attached_modules();                   // 付けている名前 (合成の順)
JSON.parse(puppet.modules_json());           // [{name, slot, requires, attached, textures, warnings, warning_details}] (読み込んだ順)
JSON.parse(puppet.slots_json());             // 本体のスロット [{id, anchor, draw_order, replaces, parameters, ...}]
JSON.parse(puppet.avatar_slots_json());      // 本体のアバタープロファイルがアプリに公開するスロットの ID ["Hair", ...]。
                                             // プロファイルが無ければ null (公開していないスロットは利用者に見せない。T0715)
JSON.parse(puppet.detached_modules_json());  // 直前の load_json / load_bundle で付け直せずに外したもの
                                             // [{code: "module_detached", module, slot, error: {code, message, module?, slot?, ...}, warning}]
try {
  puppet.attach_module("Hat");
} catch (e) {
  // e.name === "DressUpError"。e.code は "unknown_slot" | "missing_slot" | "missing_parameter" |
  // "parameter_not_exposed" | "avatar_level" | "slot_occupied" | "unknown_mask" | "unknown_collider" |
  // "pose_not_exposed" | "texture_out_of_range" | "invalid_composition" | "invalid_module_name" | "duplicate_module" |
  // "compile" | "invalid_physics" | "not_attached" | "not_loaded" | "no_model" | "invalid_file" | "texture" |
  // "device_lost" (GPU を失った後の付け外し。T0457)
  // (load_module が読み込みの上限を超えたときは "read_limit_exceeded" | "image_limit_exceeded"。下の「読み込みの上限」)。
  // 補足の値 (あれば): e.module・e.slot・e.parameter・e.mesh・e.attached (slot_occupied で付いているモジュール)・
  // e.mask (unknown_mask)・e.rig・e.collider (unknown_collider)・e.part (pose_not_exposed)・
  // e.texture (texture_out_of_range。数)・e.required・e.actual (avatar_level。数。本体がプロファイルを持たなければ actual は無い)・
  // e.errors (invalid_physics・invalid_composition の検査のエラーの数)・
  // e.error_codes (同じく検査のエラーの種類を ", " でつないだ文字列。例: "unknown_reference, duplicate_id"。T0168)。キーの意味は crates/ugoe/README.md#着せ替えの失敗の補足の値 (T0157)
  // TypeScript の型は @ugoe/runtime の UgoeDressUpError・isDressUpError (一覧の検査は下の「code の一覧」)
}
```

`code` と補足の値のうち、付け外しの失敗と `invalid_file` は SDK の `ugoe::DressUpError::code`・`fields`・`INVALID_FILE` が決め、
C ABI の `ugoe_last_error_kind` と同じ (T0412)。`set_base_physics_json` で付けているモジュールと合成できないときも、同じ `DressUpError` を投げる (T0476)。`not_loaded`・`no_model`・`texture`・`device_lost` は wasm だけのもの (`src/dress_up.rs` の `error_info`)。
GPU のデバイスを失った (`is_lost()`) 後の `load_module`・`attach_module`・`detach_module`・`set_modules`・`unload_module` は、何も変えずに
`code` が `device_lost` の `DressUpError` を投げる (T0457。それまではコードを持たない `Error`)。新しい canvas で作り直す。

- 規則は `docs/spec/modules.md#puppet-での着せ替え`。付け外しで、パラメータの値・ポーズ・トラッキング・モーションは続きから動き、物理はリセットされる。失敗したときは何も変えない。
- テクスチャ (と法線マップ) は本体、モジュールの順に並ぶ。付け外しで番号がずれたものは登録し直し、使わなくなった番号の画像は 1x1 の白に置き換えて捨てる。
- `load_json` / `load_bundle` でモデルを読み直すと、付けていたモジュールを付け直す (付けられないものは外して警告に入れる)。
  外したものは `detached_modules_json()` でも読める (T0124): `error` は付けるときの `DressUpError` と同じ `code` と補足の値に、
  英語の文 `message` を添えたもの。`warning` は戻り値の警告の一覧に入る文 (`module `X` was detached: ...`) と同じ文字列。
  一覧は次の `load_json` / `load_bundle` まで変わらない (外したものが無ければ `[]`)。
- **戻り値の文の警告は残す** (T0124 で決めた)。戻り値の `string[]` はほかの警告 (物理を外した・テクスチャを代用したなど) と
  同じ一覧で、ログに出すだけの組み込み先はそのままでよく、外すと知らせが黙って消えるため。文面は安定した値として扱わない。
  利用者向けに言い換えるときは `error.code` を使い、その要素の `warning` と同じ文字列を一覧の中で置き換える
  (ビューアは T0125 から `load_warnings_json()` で読む。エディタのプレビューは `DressUp.loaded` で知らせ、文の警告はコンソールに出すだけ)。
  外したものは読み込みのほかの警告と一緒に `load_warnings_json()` にも入る ([読み込みの警告](#読み込みの警告-t0125))。
  物理は本体の設定を引き継ぎ、モジュールのリグは付け直しで加わる。読み込んだモジュールはモデルを変えても残る。
- 同じ名前のモジュールを `load_module` し直すと、付けていれば新しい版に付け替える (エディタで書き出し直したとき)。
- モジュールを付けている間に `set_physics_json` で渡した設定は、次の付け外しで本体の設定から作り直される。保たないことは T0414 で決めた
  (合成した設定は本体の分に戻せないため。`docs/spec/modules.md#puppet-での着せ替え`)。
- 付けている間も保つ設定は `set_base_physics_json` で本体の設定として渡す (`Puppet::set_base_physics`、T0416)。付けているモジュールの組と
  合成し直し、この後の付け外しでもこの設定から合成する。本体のモデルと合成したモデルに対して検査し、失敗すれば何も変えない。
  モデルを読み直すと (`load_json`)、本体の設定が引き継がれる。C ABI では `ugoe_puppet_set_base_physics_json`・`ugoe_puppet_base_physics_json`。

### 読み込みの警告 (T0125)

```js
const warnings = puppet.load_bundle(bytes);        // 文の警告 (英語) の配列
JSON.parse(puppet.load_warnings_json());           // 同じ警告の構造 (同じ順・同じ数): [{code, warning, ...}]
```

| `code` | 補足の値 | いつ |
| --- | --- | --- |
| `texture_ktx2` | `texture` (テクスチャの番号) | テクスチャが、デコードしない形式 (ETC2・ASTC・Basis Universal など。圧縮していない 8bit の RGB / RGBA と BC1〜BC5・BC7 以外) の KTX2 なので白のプレースホルダで代用した |
| `texture_missing` | `texture` | テクスチャの画像が無いので白で代用した (`read_bundle` が数を検査するので通常は起きない) |
| `normal_map_ktx2` | `texture` | 法線マップが、デコードしない形式の KTX2 なので、法線マップなしで描く |
| `physics_removed` | `reason` (英語の理由)・`errors`・`error_codes` (理由が検査のエラーなら、その数と種類。着せ替えの `invalid_physics` と同じ形。T0168) | `load_json` で読み直したとき、引き継いだ物理の設定が新しいモデルと合わないので外した |
| `module_detached` | `module`・`slot`・`error` (`detached_modules_json()` の要素と同じ) | 付けていたモジュールを付け直せないので外した |
| `unknown_optional_feature` | `feature` (機能の ID) | 知らない任意の機能を宣言したファイル (新しい版で書かれたもの)。フォールバックで読んだ (T0412) |
| `unknown_value` | `path` (チャンクとその中の場所) | 知らない列挙の値をフォールバックで読んだ (T0412) |

- `warning` は戻り値の配列の同じ位置の文と同じ文字列。一覧は次の `load_json` / `load_bundle` が成功するまで変わらない (無ければ `[]`)。
  知らない `code` (将来の種類) は `warning` をそのまま出せばよい。
- **1 つの一覧にまとめた** (T0125 で決めた)。戻り値の配列と 1 対 1 に並ぶので、利用者向けに言い換える組み込み先は
  戻り値の代わりにこれだけを読めばよい。T0124 の `detached_modules_json()` は残す (外したモジュールだけを読む
  組み込み先のため。要素に `code: "module_detached"` を足しただけで、ほかは変えていない)。
- **戻り値の文の警告は残す** (T0124 と同じ理由: ログに出すだけの組み込み先はそのままでよく、外すと知らせが黙って消える)。
  文面は安定した値として扱わず、言い換えは `code` で行う。
- `.ugoemod` の読み込み (`load_module`) の警告 (テクスチャの代用) は `modules_json()` の各要素の `warnings` (文) と
  `warning_details` (同じ順の構造。`code` と補足の値は上と同じ) にある。
- 使う側: ビューアは日本語 (`packages/player/src/loadWarnings.ts`)、エディタのプレビューは英語 (`apps/editor/src/lib/preview/loadWarnings.ts`)。
- TypeScript の型と読む補助は npm の `@ugoe/runtime` にある (`UgoeLoadWarning`・`loadWarnings`・`UgoeDressUpError`・`isDressUpError`。
  `bindings/web/src/warnings.ts`、T0438)。`code` や補足の値を増やしたら、そちらの和型も同じ PR で直す ([`code` の一覧](#code-の一覧-t0452) の検査が落ちる)。
- `code` と補足の値は SDK の `ugoe::LoadWarning` (`code()`・`fields()`) が決め、C ABI の `ugoe_puppet_load_warning_*` と同じ (T0412)。
  `unknown_optional_feature`・`unknown_value` は読み込みの報告 (`read_bundle_with_report` / `read_module_with_report`) の警告で、
  一覧の先頭 (テクスチャの警告より前) に並ぶ。`.ugoemod` でも出る。

### 読み込みの上限 (T0152)

```js
const limits = JSON.stringify({
  max_chunk_decompressed: 64 * 1024 * 1024, // 圧縮したチャンク 1 つの展開後 (バイト)。既定 256 MiB
  max_total_decompressed: 256 * 1024 * 1024, // 1 つのファイルで展開するバイトの合計。既定 1 GiB
  max_image_side: 4096,                      // デコードする画像 (PNG・KTX2) の幅と高さ (画素)。既定 16384
});
try {
  puppet.load_bundle(bytes, limits);   // 省くと既定値。書かなかった項目も既定値
  puppet.load_module(modBytes, limits);
} catch (e) {
  // load_bundle は e.name === "LoadError"、load_module は "DressUpError"
  if (e.code === "read_limit_exceeded") console.warn(`${e.chunk}: ${e.limit} > ${e.max}`);
  if (e.code === "image_limit_exceeded") console.warn(`${e.image} ${e.texture}: ${e.width}x${e.height} > ${e.max}`);
}
```

| `code` | 補足の値 | いつ |
| --- | --- | --- |
| `read_limit_exceeded` | `limit` (`"chunk_decompressed"` \| `"total_decompressed"`)・`chunk` (チャンクのタグ。`"TEXR"` など)・`max` (バイト数。数) | 圧縮したチャンクの展開が上限を超えそうになった。展開の途中でやめる (先に確保しない) |
| `image_limit_exceeded` | `image` (`"texture"` \| `"normal_map"`)・`texture` (テクスチャの番号)・`width`・`height`・`max` (画素。どれも数) | 画像 (PNG・KTX2) の幅か高さが `max_image_side` を超える。画素を確保する前にやめる |

- 上限はファイル形式の決まりではなく、組み込む側の方針 (`docs/spec/file-formats.md#読み込みの上限`)。ファイルが壊れているとは限らない
  (上限を上げれば読めることがある) ので、`invalid_file` とは分けた。超えたときは何も変えない (前のモデル・モジュールのまま)。
- `read_limit_exceeded` の種類と補足の値は SDK の `ugoe::limits::ReadLimitExceeded` が決め、C ABI (`UGOE_ERROR_LIMIT_EXCEEDED` と
  `ugoe_last_error_kind`・`ugoe_last_error_field`) と同じ。C ABI の補足の値は文字列で、wasm は読み込みの警告と同じく数は数にする。
  `image_limit_exceeded` は wasm だけ (C ABI は画像をデコードしない)。`src/load.rs` の `LoadError::info`。
- `load_bundle` のほかの失敗も同じ `LoadError` で、`code` を持つ ([読み込みの失敗](#読み込みの失敗-t0434))。
- 上限の JSON の形の誤り (知らない項目・負の値・小数) は `invalid limits: ...` の例外 (綴りの誤りで上限が黙って効かなくならないように)。
- `load_json` は展開も画像のデコードもしないので、上限を取らない。
- デバイスの扱える大きさ (`check_texture_sizes`) は上限とは別で、デコードの後にレンダラが検査する。
- `@ugoe/runtime` では `loadModel(puppet, data, { limits: { maxTotalDecompressed, ... } })` と `isLimitError` (`bindings/web/src/limits.ts`)。

### 読み込みの失敗 (T0434)

```js
try {
  puppet.load_bundle(bytes); // load_json(json) も同じ
} catch (e) {
  // e.name === "LoadError"。e.message は英語の文 (安定した値ではない)。言い換えは e.code で行う
  if (e.code === "unsupported_version") console.warn(`version ${e.found} > ${e.supported}`);
}
```

`load_json` / `load_bundle` の失敗は、すべて `name` が `"LoadError"` で `code` と補足の値を持つ `Error`。失敗したときは何も変えない
(前のモデルのまま)。補足の値は、番号と大きさが数、ほかは文字列 (読み込みの警告・着せ替えの失敗と同じ)。

| `code` | 補足の値 | いつ |
| --- | --- | --- |
| `invalid_file` | なし | ファイルを読めない (ugoe のファイルでない・壊れている・JSON の構文や型の誤り・チャンクの誤りなど)。細かい理由は文にだけある |
| `unsupported_version` | `found` (ファイルの版)・`supported` (読める版) | ファイルの版 (スキーマかコンテナ) が新しすぎる |
| `unsupported_features` | `features` (知らない必須の機能の ID を `", "` でつないだ文字列) | 知らない必須の機能を宣言している |
| `invalid_model` | `errors` (検査のエラーの数)・`error_codes` (検査のエラーの種類を `", "` でつないだ文字列。着せ替えの `invalid_physics` と同じ形。T0469) | モデル・物理・モーション・表情が検査に通らない。個々のエラーの中身は文にだけある |
| `compile` | なし | 検査は通ったが評価できない構造 |
| `read_limit_exceeded` | [読み込みの上限](#読み込みの上限-t0152) | 展開の上限を超えた |
| `image_limit_exceeded` | [読み込みの上限](#読み込みの上限-t0152) | PNG の一辺が `max_image_side` を超えた |
| `invalid_texture` | `image` (`"texture"` \| `"normal_map"`)・`texture` (テクスチャの番号) | 画像 (PNG・KTX2) をデコードできない (壊れている。読まない形式の KTX2 は失敗でなく `texture_ktx2` などの警告) |
| `texture_too_large` | `image`・`texture`・`width`・`height`・`max` (デバイスの一辺の上限。画素) | 画像がデバイス (`max_texture_dimension_2d`) で扱える大きさを超える |
| `texture` | なし | テクスチャをレンダラに登録できない (着せ替えの `texture` と同じ) |
| `device_lost` | なし | GPU のデバイスを失った (`is_lost()`)。新しい canvas で作り直す |

- コードは公開の約束として扱い、名前を変えない (意味が変わるなら新しいコードにする)。知らない `code` (将来の種類) は `message` を出せばよい。
- ファイルとモデルの失敗 (`invalid_file`・`unsupported_version`・`unsupported_features`・`invalid_model`・`compile`・`read_limit_exceeded`) の
  種類と補足の値は SDK の `ugoe::LoadError` (`code()`・`fields()`) が決め、C ABI の `ugoe_puppet_create_from_*` の `ugoe_last_error_kind` と
  同じ値 (T0457。[crates/ugoe-ffi/README.md](../ugoe-ffi/README.md#モデルの読み込みの失敗-t0457)。C ABI では数も 10 進の文字列)。
  画像とレンダラの失敗 (`image_limit_exceeded`・`invalid_texture`・`texture_too_large`・`texture`・`device_lost`) は wasm だけ
  (C ABI は画像をデコードせず、レンダラを持たない)。対応は `src/load.rs` の `LoadError::info` とそのテスト (`every_load_error_has_a_code`)。
- `.ugoemod` の読み込み (`load_module`) の失敗は `DressUpError` のまま: 上限を超えたものだけ上の 2 つのコードで、ほかは `invalid_file`
  (C ABI と同じ)。
- 上限の JSON の形の誤り (`invalid limits: ...`) は `code` を持たない `Error` (組み込みの誤りで、利用者に見せる文ではない)。
  `WebPuppet.create` の失敗は [`CreateError`](#作成の失敗-t0475)。
- 使う側: ビューアとスタジオは `@ugoe/player` が今の言語に訳す (`packages/player/src/loadError.ts`)。TypeScript の型は npm の
  `@ugoe/runtime` の `UgoeLoadError`・`isLoadError` (`bindings/web/src/limits.ts`)。コードを増やしたら、どちらも同じ PR で直す。

### 描画と読み戻しの失敗 (T0465)

```js
try {
  puppet.render(); // read_pixels() の Promise の失敗も同じ
} catch (e) {
  // GPU を失った後は e.name === "RenderError"、e.code === "device_lost"。新しい canvas で作り直す
  if (e.name === "RenderError" && e.code === "device_lost") recreate();
}
```

`render`・`read_pixels` の失敗のうち、利用者に知らせるものは `name` が `"RenderError"` で `code` を持つ `Error` (読み込みの `LoadError`・
着せ替えの `DressUpError` と同じ形。それまではコードを持たない `Error`)。何も描かず、何も変えない。

| `code` | 補足の値 | いつ |
| --- | --- | --- |
| `device_lost` | なし | GPU のデバイスを失った (`is_lost()`)。`read_pixels` は読み戻しを待つ間に失ったときも。新しい canvas で作り直す |

- ほかの描画の失敗 (canvas のテクスチャを取れない・レンダラの失敗・読み戻しの形式が無い・読み戻しの時間切れ) は、組み込みの誤りか一時的なもので、
  コードを持たない `Error` のまま。知らない `code` (将来の種類) は `message` を出せばよい。
- 文 (`message`) は読み込みと着せ替えの `device_lost` と同じ `the GPU device was lost` (安定した値ではない)。種類は `src/frame.rs` の `FrameError`。
- 使う側: `@ugoe/player` の `describeRenderError` (`packages/player/src/loadError.ts`) が今の言語に訳す (読み込みの `device_lost` と同じ文)。
  TypeScript の型は npm の `@ugoe/runtime` の `UgoeRenderError`・`isRenderError` (`bindings/web/src/render-errors.ts`)。

### 作成の失敗 (T0475)

```js
try {
  puppet = await WebPuppet.create(canvas, "webgl2");
} catch (e) {
  // GPU やブラウザの都合で作れない: e.name === "CreateError"、e.code === "context_unavailable" など。
  // 時間を置いて新しい canvas で作り直せば作れることがある
}
```

`WebPuppet.create` の失敗のうち、GPU やブラウザの都合で作れなかったものは、`name` が `"CreateError"` で `code` を持つ `Error`
(読み込みの `LoadError`・描画の `RenderError` と同じ形)。補足の値は持たない。どれも panic (wasm の trap。以後の呼び出しが壊れる) にはならない。

| `code` | いつ |
| --- | --- |
| `backend_unavailable` | `"webgpu"` を求めたが、このブラウザに WebGPU が無い |
| `context_unavailable` | canvas から GPU のコンテキストを取れない (`getContext` が `null` を返した・取れた WebGL2 のコンテキストが既に失われている。canvas を別の種類のコンテキストで使っている・WebGL のコンテキストの数の上限・GPU プロセスの不調など) |
| `adapter_unavailable` | GPU のアダプタを取れない |
| `device_unavailable` | GPU のデバイスを作れない |

- 失ったコンテキストの確かめ: wgpu の GL のバックエンド (wgpu-hal 30) は `getContext` で受け取った WebGL2 のコンテキストを確かめずに
  glow (0.17) に渡し、glow は失ったコンテキストの `getSupportedExtensions()` が返す `null` を `unwrap` して panic する
  (`glow-0.17.0/src/web_sys.rs` の `build_extensions!`)。そこで WebGL2 で作るときは、サーフェスを作った直後 (アダプタを求める前) に
  同じ canvas の `getContext("webgl2")` (wgpu が受け取ったものと同じコンテキスト) の `isContextLost()` を確かめ、失っていれば `context_unavailable` を投げる。
- 作成の途中の喪失 (T0944): GPU プロセスが落ちると、同期的に進む作成の途中 (glow の初期化・シェーダーのリンクの後など) でもコンテキストを失う。
  glow と wgpu-hal は、失ったコンテキストが返す値のいくつかを `unwrap` するので (`getSupportedExtensions()` の `null`・`getParameter(VERSION)` の
  `null`・`getUniformBlockIndex()` の `INVALID_INDEX`)、WebGL2 で作るときはそのコンテキストのオブジェクトの上でこの 3 つを包み、panic する値を
  panic しない値に置き換える (`src/web/lost_context.rs`。生きているコンテキストでは返らない値だけなので、振る舞いは変わらない)。GPU プロセスが落ちた
  直後は、既定の値が返っても `isContextLost()` が偽のままのことがあるので、失ったかどうかでは見分けない。作成の最後にもう一度
  `isContextLost()` を確かめ、途中で失っていれば (別の段の失敗も含めて) `context_unavailable` を投げる (`webglcontextlost` は後で届くので、
  作れたことにすると `is_lost()` がまだ偽の WebPuppet を渡してしまう)。確かめ方は [docs/testing.md](../../docs/testing.md#puppetsurface-のブラウザのテスト)。
- 知らない `backend` (`unknown backend ...`) と、起きないはずの失敗 (canvas のサーフェスが形式を 1 つも持たない・`webglcontextlost` を聞けない) は、
  組み込みの誤りなので `code` を持たない `Error` のまま。
- 種類は `src/create.rs` の `CreateError`。使う側: `@ugoe/player` の `PuppetSurface` と `<ugoe-avatar>` は、作り直しの途中の失敗を
  「GPU がまだ戻っていない」として待って試し直し、作り直しの回数に数える ([docs/ui.md](../../docs/ui.md#webpuppet-の寿命))。
  画面の文は `@ugoe/player` の `describeCreateError` (エディタは `viewport.createError.*`) が今の言語に訳す (T0941)。
  TypeScript の型は npm の `@ugoe/runtime` の `UgoeCreateError`・`isCreateError` (`bindings/web/src/create-errors.ts`)。

### `code` の一覧 (T0452)

読み込みの警告 (`load_warning`)・着せ替えの失敗 (`dress_up_error`)・読み込みの失敗 (`load_error`)・描画と読み戻しの失敗 (`render_error`。T0465)・
作成の失敗 (`create_error`。T0475)・設定の差し替え (`set_effects_json`・`set_physics_json`・`set_base_physics_json`) の検査の失敗 (`settings_error`。T0469) の
`code` と (`set_base_physics_json` の合成の失敗は `DressUpError` なので `dress_up_error` に入る。T0476)、種類ごとの補足の値のキーの一覧を `codes.json` に置く。`src/codes.rs` のテストが、すべての種類の見本 (`LoadWarning`・`WardrobeError`・`LoadError`・`FrameError`・`CreateError`・`AnimationError` と、その中の
`DressUpError`・`ComposeError`・`CompileError`) から一覧を作って比べる。種類を足すと、テストの網羅の `match` (`guard`) がコンパイルできなくなるので、
見本を足してから `UGOE_UPDATE_CODES=1 cargo test -p ugoe-wasm codes` で書き直す。

`@ugoe/runtime` のテスト (`bindings/web/src/codes.test.ts`) は同じ `codes.json` を TS の `code` の配列と補足の値の型と比べるので、
Rust と TS の片方だけに種類やキーを足すと、どちらかのテストが落ちる (どちらも `just ci` で流れる)。
`FormatError`・`RenderError` の種類は網羅しない (ほかは `invalid_file`・`texture` にまとまる)。`ugoe::LoadError::code`・`LoadError::info` で
新しい `code` を割り当てたら、`src/codes.rs` に見本を足す。

## ビルド

```sh
cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked  # 初回だけ
just wasm        # apps/editor/src/lib/viewport/generated/ に出力 (gitignore 済み)
just wasm apps/viewer/src/lib/wasm/generated  # ビューア用 (apps/viewer の `pnpm wasm`)
just wasm-check  # wasm32 向けの clippy
```

wasm-bindgen-cli の版は `Cargo.lock` の `wasm-bindgen` と一致させる必要がある (`just wasm` が検査する)。

npm のパッケージ `@ugoe/runtime` の wasm は、`just web-runtime` が大きさを優先した別の設定 (opt-level "z"・fat LTO・関数名の節を外す)
でビルドする (`bindings/web/README.md#wasm-の大きさ`)。`just wasm` (エディタ・ビューア) の設定は変えていない。

## 仕様

- [docs/editor.md#ビューポート](../../docs/editor.md#ビューポート)
- [docs/adr/0003-editor-viewport-in-webview.md](../../docs/adr/0003-editor-viewport-in-webview.md)
- [docs/spec/rendering.md](../../docs/spec/rendering.md) (乗算済みアルファ、クリアはホストの責務)

## 関連タスク

- T0402 (`docs/tasks/T0402-*.md`)
- T0210 (ライティング: 法線マップの登録と光源の JSON)
- T0123 (着せ替え: `.ugoemod` の読み込みと付け外し)
- T0418 (描画結果の読み戻しと評価の結果の取り出し)
- T0152 (読み込みの上限)
- T0434 (読み込みの失敗のコード)
- T0457 (読み込みの失敗の種類を SDK で決めて C ABI と揃える・着せ替えの `device_lost`)
- T0465 (描画と読み戻しの `RenderError` の `device_lost`)
