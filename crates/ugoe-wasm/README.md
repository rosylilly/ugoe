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
| `load` | JSON / `.ugoe` バンドルの読み込み、PNG のデコード (乗算済みアルファへ)、パラメータ値の引き継ぎ |
| `view` | フィット・ズーム・パンと、キャンバス座標 ↔ 画面座標の変換 |
| `tracking` | フェイストラッキングの入力 (ブレンドシェイプの名前と値・頭の回転) から `TrackingInput` を作る。JS への公開は `web/tracking.rs` |
| `hit` | 評価済みの要素の外接矩形とヒットテスト (エディタの選択用。三角形の判定は `ModelInstance::mesh_contains`) |
| `physics_view` | エディタの物理の表示用の問い合わせ: デフォーマの写像とヤコビ行列、粒子の今の位置 (JS への公開は `src/web/physics.rs`) |
| `dress_up` | `.ugoemod` の読み込み、付け外しに合わせたテクスチャの登録し直し (`Wardrobe`)、エラーのコード (JS への公開は `src/web/dress_up.rs`) |
| `lighting` | 光源の JSON (`set_lighting_json`) を `ugoe::render::Lighting` にする (JS への公開は `src/web/lighting.rs`) |
| `readback` | `drawable_states_json` の中身と、読み戻した画素をストレートアルファの RGBA にする処理 (JS への公開は `src/web/readback.rs`) |
| `animation` | 物理・モーション・表情・エフェクトの JSON の受け渡し、バンドルのモーション・表情の名前の解決、モデルを読み直したときの物理と段の有効無効の引き継ぎ (JS への公開は `src/web/animation.rs`) |

## `WebPuppet` の使い方

```js
import init, { WebPuppet } from "./generated/ugoe_wasm.js";
await init();
const puppet = await WebPuppet.create(canvas, "auto"); // "auto" | "webgpu" | "webgl2"
puppet.load_json(json);           // または load_bundle(bytes)。戻り値は警告の配列 (構造は load_warnings_json())
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
- トラッキングの入力は次の `update(dt)` から毎フレーム適用される (モーションの直後、表情・目パチ・呼吸の前)。
  平滑化・キャリブレーション・回転行列からの角度の計算・鏡像の入れ替えはホストが行う
  (`docs/spec/avatar-profile.md#実装の範囲`)。`clear_tracking()` の後も最後の値は残る。
- `set_motion_over_tracking(true)` の間は、`update` がトラッキングの値をモーションより **先に** 書き込む。
  モーションが動かすパラメータではモーションの値がフェードの重みでトラッキングの値に重なり、ほかのパラメータはトラッキングの値のまま。
  `set_tracking_kept(ids)` (T0722) で選んだパラメータは、優先していてもモーションの後に書き込む (トラッキングのまま。目は除く、など)。
  既定は `false` (仕様の順。トラッキングがモーションの値を置き換える) と空。どちらもモデルを読み直しても保つ。
  T0721 では wasm が `Puppet::update` の前にホストの書き込みとして入れていたが、パラメータごとに分けるために `Puppet` に移した (T0722)。
  ビューアは待機以外の優先度のモーションの再生中だけ入れる。
- `is_lost()` が `true` になったら (GPU プロセスのクラッシュなど)、新しい canvas で作り直す。

### 描画結果の読み戻しと評価の結果 (T0418)

```js
const pixels = await puppet.read_pixels(); // Uint8Array。width() × height() のストレートアルファの RGBA、行は上から下
JSON.parse(puppet.drawable_states_json()); // [{opacity, draw_order, multiply_color: [r, g, b], screen_color, mask_strength, visible}]
```

- `read_pixels` は、今の状態を canvas と同じ大きさ・ビュー・背景・光源で canvas とは **別のテクスチャ** に描き、バッファに写して読む
  (canvas には描かない)。canvas の読み戻し (`drawImage`・`toBlob`) はヘッドレスの Chrome の WebGPU で空になることがあったので、
  配布物の適合テスト (`tests/conformance`) は WebGL2・WebGPU のどちらでもこれで画像を取る。スクリーンショットにも使える。
- `drawable_states_json` は `drawable_ids()` の順。頂点は `drawable_positions`、描画の順番は `render_order` (`docs/spec/evaluation.md#8-出力`)。

### 物理・モーション・表情・エフェクト (T0309)

```js
puppet.load_bundle(bytes);             // バンドルの物理・モーション・表情も読む (物理は検査済み)
puppet.motion_names();                 // バンドルのモーションの名前
puppet.motions_json();                 // [{name, duration, looping, fade_in, fade_out, parameters, events}] (T0720)
puppet.start_motion("idle", "idle");   // 優先度は "idle" | "normal" (既定) | "force"。開始しなければ false
puppet.start_motion_json(json);        // *.motion.json の形。タイムラインのプレビュー用
sample_motion_json(json, t);           // (関数) 各カーブの時刻 t の値 (フェードなし) の JSON。タイムラインのシーク用
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
puppet.pose_json();                    // 今のポーズ (モデルの pose の形。付けたモジュールのグループと追従も入る。T0661)
puppet.part_evaluated_opacity("ArmB"); // 評価後のパーツの不透明度 (祖先を掛けた値。T0416)。パーツが無ければ undefined
puppet.update(dt);                     // Puppet::update と同じ順で 1 フレーム進める
puppet.motion_events();                // 直前の update で発火したイベントの値
puppet.hit_test_standard("Head", x, y); // Model.avatar.hit_areas の標準の名前で当たり判定
```

- 適用順とスライダー (`set_parameter`) との関係は `docs/spec/motion.md#フレーム内の適用順` と `#手で動かしたパラメータとの関係`。
- `load_json` で読み直すと、物理の設定・モーションと表情の一覧・段の有効無効を引き継ぐ (新しいモデルと合わない物理は外して警告を返す)。`load_bundle` は物理・モーション・表情をバンドルのものに置き換える。
- モーション・表情の再生状態は、モデルを読み直すと初めからになる。
- `motions_json` の `parameters` はカーブが動かすパラメータ (重なりなし)、`events` はイベントの値。`expressions_json` は未知の `op` の項目を出さない。
  `active_expressions` は同じ名前を重ねても 1 つと数える。Rust の `ExpressionManager::players` はフェードアウト中のものも含み、止めたかを
  返さないので、`WebPuppet` が表情の操作 (`set_expression`・`set_expression_json`・`add_expression`・`stop_expression`・`stop_expressions`) を写して
  持ち、再生中の表情が無い名前は除いて返す (`crate::animation::ActiveExpressions`)。モデルを読み直すと空になる。
- `set_effects_json` は同じ設定でも目パチ・呼吸を初めからやり直し、リップシンクの音量は保つ。`show_pose` の不透明度は次の `update` からフェードし、
  モーションが同じグループの要素を要求すればそちらが勝つ (`docs/spec/motion.md#エフェクト設定の差し替え`・`#ポーズ`)。C ABI では
  `ugoe_puppet_set_effects_json`・`ugoe_puppet_show_pose` (T0414)。
- `part_evaluated_opacity` は上書きを当てた自身の値に祖先の値を掛けたもの (`ModelInstance::part_opacity`、`docs/spec/evaluation.md` の 2.)。
  そのパーツのメッシュの不透明度に掛かる。必要なら評価してから読む。C ABI では `ugoe_puppet_part_evaluated_opacity` (T0416)。

### 受け手として進める (T0133)

```js
puppet.parameter_values();              // Float32Array。parameter_ids() の順の現在値
puppet.set_base_values(values);         // Float32Array をまとめて基準値として書く (値域にクランプ。評価は次の render まで遅らせる)
puppet.follow(values, dt);              // values を基準値として書き、物理とポーズだけを dt 秒進めて評価する
```

- `Puppet::set_base_values`・`Puppet::follow` を呼ぶ (`docs/spec/motion.md#受け手として進める`)。スタジオの出力ウィンドウとブラウザソースの
  受け手が、指揮役から届いた `{dt, values}` を順に適用するために使う (ADR 0028 の 3.)。
- `follow` はこの `WebPuppet` のモーション・トラッキング・表情・目パチ・呼吸・リップシンクを通さない (再生の状態も時刻も進まない)。
  物理とポーズは `set_enabled` に従う。`motion_events()` は直前の `update` のまま。
- `set_parameter` の値は前のフレームの出力とビット列で同じなら基準値にならないが、`set_base_values` と `follow` は同じでも基準値にする。
- モデルが無い・数が `parameter_ids()` と違う・NaN を含むときは、何も変えずに例外を投げる。C ABI では `ugoe_puppet_set_base_values`・`ugoe_puppet_follow`。

### エディタ向けの問い合わせ (ギズモ・物理の表示)

```js
puppet.warp_points("Cloth");              // ワープの制御点 (キャンバス座標)
puppet.rotation_frame("Head");            // [ox, oy, ax, ay, bx, by]
puppet.deformer_point("Head", 0, -40);    // デフォーマの局所座標の点 → キャンバス座標 [x, y]
puppet.deformer_jacobian("Cloth", 0.5, 0.5); // その点でのヤコビ行列 [a, b, c, d] (列優先。逆写像用)
puppet.physics_particles(0);              // リグ 0 の粒子の今の位置 [x0, y0, ...] (動的物理ならキャンバス座標)
```

- どれも今のパラメータ値 (と最後の `update`) で評価した値を返す (`src/gizmo.rs`・`src/physics_view.rs`)。T0629 (エディタの物理の画面) で追加。

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
- 光源を設定しなければ、ライティングを受けるメッシュも受けない場合と同じ色で描く。法線マップはアルファを掛けずに読む (KTX2 は警告を出して使わない)。

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
  // "compile" | "invalid_physics" | "not_attached" | "not_loaded" | "no_model" | "invalid_file" | "texture"。
  // 補足の値 (あれば): e.module・e.slot・e.parameter・e.mesh・e.attached (slot_occupied で付いているモジュール)
}
```

`code` と補足の値のうち、付け外しの失敗と `invalid_file` は SDK の `ugoe::DressUpError::code`・`fields`・`INVALID_FILE` が決め、
C ABI の `ugoe_last_error_kind` と同じ (T0412)。`not_loaded`・`no_model`・`texture` は wasm だけのもの (`src/dress_up.rs` の `error_info`)。

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
| `texture_ktx2` | `texture` (テクスチャの番号) | テクスチャが KTX2 (未対応) なので白のプレースホルダで代用した |
| `texture_missing` | `texture` | テクスチャの画像が無いので白で代用した (`read_bundle` が数を検査するので通常は起きない) |
| `normal_map_ktx2` | `texture` | 法線マップが KTX2 なので、法線マップなしで描く |
| `physics_removed` | `reason` (英語の理由) | `load_json` で読み直したとき、引き継いだ物理の設定が新しいモデルと合わないので外した |
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
- `code` と補足の値は SDK の `ugoe::LoadWarning` (`code()`・`fields()`) が決め、C ABI の `ugoe_puppet_load_warning_*` と同じ (T0412)。
  `unknown_optional_feature`・`unknown_value` は読み込みの報告 (`read_bundle_with_report` / `read_module_with_report`) の警告で、
  一覧の先頭 (テクスチャの警告より前) に並ぶ。`.ugoemod` でも出る。

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
