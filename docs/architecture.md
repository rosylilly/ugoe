# アーキテクチャ

## 全体像

```
                         ┌──────────────── ツール層 ────────────────┐
                         │                                          │
 PSD ──► ugoe-psd ──► ugoe-editor-core ◄── ugoe-ai (提案 = Command 列)│
          ugoe-mesher ─┘   │  Document / Command / Undo             │
                           │                                        │
           ┌───────────────┼────────────────┐                       │
           ▼               ▼                ▼                       │
      apps/editor      ugoe-cli        (将来) ugoe-mcp               │
      (Tauri + Svelte)                                              │
                         └──────────────────────────────────────────┘
                                          │ Model / .ugoe
                         ┌──────────────── ランタイム層 ──────────────┐
                         ▼                                          │
 ugoe-core ◄── ugoe-model ◄── ugoe-runtime ◄── ugoe-physics          │
                    ▲             ▲  ▲         ugoe-motion           │
                    │             │  └──────── ugoe-render (wgpu)    │
               ugoe-format        │                                 │
                    └──────────── ugoe (Puppet ファサード)            │
                                   ▲        ▲                       │
                               ugoe-ffi  ugoe-wasm                  │
                                   │        │                       │
                     Unity/Unreal/Godot   Web / エディタのビューポート │
                         └──────────────────────────────────────────┘
```

## レイヤーと依存ルール

依存は **下向きにだけ** 張れます。`scripts/check-layers.mjs` (`just layers`) が検査し、CI で強制します。

| 層 | クレート | 依存してよいもの |
| --- | --- | --- |
| L0 基盤 | `ugoe-core` | 外部の小さなクレートのみ |
| L1 データ | `ugoe-model` | core |
| L2 評価・入出力 | `ugoe-runtime`, `ugoe-format` | core, model |
| L3 機能 | `ugoe-physics`, `ugoe-motion`, `ugoe-render` | core, model, runtime |
| L4 SDK | `ugoe` | L0〜L3 |
| L5 バインディング | `ugoe-ffi`, `ugoe-wasm` | L0〜L4 |
| T1 素材処理 | `ugoe-psd`, `ugoe-mesher` | core |
| T2 エディタ中核 | `ugoe-editor-core` | ランタイム層, T1 |
| T3 自動化 | `ugoe-ai` | T2 以下 |
| T4 アプリ | `ugoe-cli`, `apps/editor` (`ugoe-editor`) | すべて |

層を分ける理由:

- **ランタイム層は配布物になる。** ゲームや Web に組み込まれるので、小さく、依存が少なく、wasm32 でビルドできなければならない。PSD パーサや AI クライアントを混ぜない。
- **`ugoe-model` はデータだけを持つ。** 「データ」と「アルゴリズム」を分けておくと、エディタ・フォーマット・ランタイムがそれぞれ独立に進められる。また、ランタイムを他言語で再実装するときにも仕様とデータ定義だけを渡せばよい。
- **物理・モーション・レンダラは互いに依存しない。** 3 つを別々のエージェントが並行して開発できる。

## 1 フレームのデータフロー

```
アプリ ──dt──► Puppet::update(dt)
                 │ 1. モーション       ──┐
                 │ 2. 表情               │ ParameterValues を書き換える
                 │ 3. 目パチ・呼吸       │ (ugoe-motion / ugoe-physics)
                 │ 4. 物理               │
                 │ 5. ポーズ・リップシンク ┘
                 ▼
            ModelInstance::update()               (ugoe-runtime)
                 │ a. キーフォーム重みの計算 (多重線形補間)
                 │ b. パーツ不透明度の伝播
                 │ c. デフォーマをトポロジカル順に評価
                 │ d. メッシュ頂点を親デフォーマ経由でキャンバス座標へ
                 │ e. グルーの適用
                 │ f. 描画順ソート
                 ▼
            drawables() / render_order()
                 │
                 ├──► ugoe-render (wgpu 参照レンダラ)
                 └──► エンジン独自レンダラ (FFI 経由で頂点バッファを受け取る)
```

## 主要な設計判断

設計判断の詳細は [adr/](adr/README.md) にあります。要点は次のとおりです。

| 判断 | ADR |
| --- | --- |
| 変形は CPU で評価し、GPU は描画にだけ使う | [0002](adr/0002-cpu-deformation-gpu-rendering.md) |
| エディタのビューポートは WebView 内の wasm (WebGPU → WebGL2 フォールバック) で描く | [0003](adr/0003-editor-viewport-in-webview.md) |
| 独自ファイル形式: オーサリング用 JSON とランタイム用バイナリ `.ugoe` | [0004](adr/0004-file-formats.md) |
| 編集操作はすべてシリアライズ可能な `Command` を通す | [0005](adr/0005-editor-command-model.md) |
| AI は提案 (Command 列) を出すだけで、適用は人間が承認する | [0006](adr/0006-ai-proposals.md) |

## 座標系

- **キャンバス座標**: 左上が原点、単位はピクセル、Y 軸は下向き。モデルデータとランタイム出力は、特に断りがなければこの座標系。
- **デフォーマ局所座標**: ワープデフォーマの子は [0,1]² の UV 空間、回転デフォーマの子は原点を中心とした回転前のピクセル座標 ([spec/evaluation.md](spec/evaluation.md))。
- **表示座標**: キャンバス座標からクリップ座標への変換は描画側 (`ugoe_render::View`) が決める。
