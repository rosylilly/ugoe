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
