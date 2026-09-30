# ugoe-wasm

**層:** L5 バインディング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

WebAssembly バインディング。Web ビューア・ビューアのアプリ (`apps/viewer`)・エディタのビューポートで使う。

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
| `animation` | 物理・モーション・表情・エフェクトの JSON の受け渡し、バンドルのモーション・表情の名前の解決、モデルを読み直したときの物理と段の有効無効の引き継ぎ (JS への公開は `src/web/animation.rs`) |

## `WebPuppet` の使い方

```js
import init, { WebPuppet } from "./generated/ugoe_wasm.js";
await init();
const puppet = await WebPuppet.create(canvas, "auto"); // "auto" | "webgpu" | "webgl2"
puppet.load_json(json);           // または load_bundle(bytes)。戻り値は警告の配列
puppet.resize(canvas.clientWidth * devicePixelRatio, canvas.clientHeight * devicePixelRatio);
puppet.parameters_json();         // 値域・既定値・グループ ({"parameters": [...], "groups": [...]})
puppet.set_parameter("AngleX", 15);
puppet.render();                  // 呼ばれたときだけ描く。ループはホストが回す

// フェイストラッキング (T0703)。変換は Rust の対応付け (ugoe_motion::tracking) で行う
puppet.set_tracking(names, scores, yaw, pitch, roll); // MediaPipe のカテゴリ名と値、頭の回転 (度)
puppet.tracking_parameter_ids();  // トラッキングが書き込むパラメータ
puppet.set_enabled("eye_blink", false); // トラッキング中はモデルの目パチを止める (T0309 の段の切り替え)
puppet.clear_tracking();
```

- 画面座標は canvas の描画バッファの画素 (CSS ピクセル × `devicePixelRatio`)。
- `set_parameter` は評価を遅らせ、次の `render` や問い合わせでまとめて評価する。物理やモーションを動かすときは `update(dt)` を呼ぶ。
- モデルを読み直すと、同じ ID のパラメータの値は引き継がれる (エディタで編集・Undo のたびに読み直すため)。
- `"auto"` は WebGPU のアダプタを実際に要求して判定し、無ければ WebGL2 を使う。1 つの canvas は WebGPU と WebGL2 を併用できないので、作成に失敗したら新しい canvas で作り直す。
- トラッキングの入力は次の `update(dt)` から毎フレーム適用される (モーションの直後、表情・目パチ・呼吸の前)。
  平滑化・キャリブレーション・回転行列からの角度の計算・鏡像の入れ替えはホストが行う
  (`docs/spec/avatar-profile.md#実装の範囲`)。`clear_tracking()` の後も最後の値は残る。
- `is_lost()` が `true` になったら (GPU プロセスのクラッシュなど)、新しい canvas で作り直す。

### 物理・モーション・表情・エフェクト (T0309)

```js
puppet.load_bundle(bytes);             // バンドルの物理・モーション・表情も読む (物理は検査済み)
puppet.motion_names();                 // バンドルのモーションの名前
puppet.start_motion("idle", "idle");   // 優先度は "idle" | "normal" (既定) | "force"。開始しなければ false
puppet.start_motion_json(json);        // *.motion.json の形。タイムラインのプレビュー用
sample_motion_json(json, t);           // (関数) 各カーブの時刻 t の値 (フェードなし) の JSON。タイムラインのシーク用
puppet.set_expression("smile");        // または set_expression_json(json)。stop_expressions()
puppet.set_lip_sync_level(0.4);        // 0..1。undefined で口に触らない
puppet.set_enabled("physics", false);  // "physics" | "eye_blink" | "breath" | "lip_sync" | "pose"
puppet.set_physics_json(json);         // 物理だけを差し替える (validate_physics で検査。undefined で外す)
puppet.update(dt);                     // Puppet::update と同じ順で 1 フレーム進める
puppet.motion_events();                // 直前の update で発火したイベントの値
puppet.hit_test_standard("Head", x, y); // Model.avatar.hit_areas の標準の名前で当たり判定
```

- 適用順とスライダー (`set_parameter`) との関係は `docs/spec/motion.md#フレーム内の適用順` と `#手で動かしたパラメータとの関係`。
- `load_json` で読み直すと、物理の設定・モーションと表情の一覧・段の有効無効を引き継ぐ (新しいモデルと合わない物理は外して警告を返す)。`load_bundle` は物理・モーション・表情をバンドルのものに置き換える。
- モーション・表情の再生状態は、モデルを読み直すと初めからになる。

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
JSON.parse(puppet.modules_json());           // [{name, slot, requires, attached, textures, warnings}] (読み込んだ順)
JSON.parse(puppet.slots_json());             // 本体のスロット [{id, anchor, draw_order, replaces, parameters, ...}]
JSON.parse(puppet.detached_modules_json());  // 直前の load_json / load_bundle で付け直せずに外したもの
                                             // [{module, slot, error: {code, message, module?, slot?, ...}, warning}]
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

- 規則は `docs/spec/modules.md#puppet-での着せ替え`。付け外しで、パラメータの値・ポーズ・トラッキング・モーションは続きから動き、物理はリセットされる。失敗したときは何も変えない。
- テクスチャ (と法線マップ) は本体、モジュールの順に並ぶ。付け外しで番号がずれたものは登録し直し、使わなくなった番号の画像は 1x1 の白に置き換えて捨てる。
- `load_json` / `load_bundle` でモデルを読み直すと、付けていたモジュールを付け直す (付けられないものは外して警告に入れる)。
  外したものは `detached_modules_json()` でも読める (T0124): `error` は付けるときの `DressUpError` と同じ `code` と補足の値に、
  英語の文 `message` を添えたもの。`warning` は戻り値の警告の一覧に入る文 (`module `X` was detached: ...`) と同じ文字列。
  一覧は次の `load_json` / `load_bundle` まで変わらない (外したものが無ければ `[]`)。
- **戻り値の文の警告は残す** (T0124 で決めた)。戻り値の `string[]` はほかの警告 (物理を外した・テクスチャを代用したなど) と
  同じ一覧で、ログに出すだけの組み込み先はそのままでよく、外すと知らせが黙って消えるため。文面は安定した値として扱わない。
  利用者向けに言い換えるときは `detached_modules_json()` の `code` を使い、その要素の `warning` と同じ文字列を一覧から除く
  (ビューアの `describeLoadWarnings`。エディタのプレビューは `DressUp.loaded` で知らせ、文の警告はコンソールに出すだけ)。
  物理は本体の設定を引き継ぎ、モジュールのリグは付け直しで加わる。読み込んだモジュールはモデルを変えても残る。
- 同じ名前のモジュールを `load_module` し直すと、付けていれば新しい版に付け替える (エディタで書き出し直したとき)。
- モジュールを付けている間に `set_physics_json` で渡した設定は、次の付け外しで本体の設定から作り直される。

## ビルド

```sh
cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked  # 初回だけ
just wasm        # apps/editor/src/lib/viewport/generated/ に出力 (gitignore 済み)
just wasm apps/viewer/src/lib/wasm/generated  # ビューア用 (apps/viewer の `pnpm wasm`)
just wasm-check  # wasm32 向けの clippy
```

wasm-bindgen-cli の版は `Cargo.lock` の `wasm-bindgen` と一致させる必要がある (`just wasm` が検査する)。

## 仕様

- [docs/editor.md#ビューポート](../../docs/editor.md#ビューポート)
- [docs/adr/0003-editor-viewport-in-webview.md](../../docs/adr/0003-editor-viewport-in-webview.md)
- [docs/spec/rendering.md](../../docs/spec/rendering.md) (乗算済みアルファ、クリアはホストの責務)

## 関連タスク

- T0402 (`docs/tasks/T0402-*.md`)
- T0210 (ライティング: 法線マップの登録と光源の JSON)
- T0123 (着せ替え: `.ugoemod` の読み込みと付け外し)
