---
id: T0026
title: M9 の後続のタスクの「起票の候補」をまとめて起票する (2 回目)
status: done
milestone: M9
crates: [docs]
depends_on: []
assignee: claude-worker-T0026
---

# T0026: M9 の後続のタスクの「起票の候補」をまとめて起票する (2 回目)

## 目的

M9 の後続のタスクを並列で進めたワーカーは、新しいタスクを起票せずに「起票の候補」をメモに書いた (番号の衝突を避けるため)。
オーケストレーターが集めた候補の一覧を読み、重複をまとめ、既存のタスクに含まれるものは除いて、タスクとして起票する。

## 受け入れ条件

- [x] オーケストレーターの一覧 (下の「参考」) と、M9 の done のタスクのメモの「起票の候補」を読んだ
- [x] 同じことを言っている候補を 1 つのタスクにまとめ、既存のタスク (todo のもの) に含まれる候補はそのタスクのメモに書き足して起票しない
- [x] 残りを docs/tasks/README.md の形でタスクにした (milestone は M9。受け入れ条件は検証できる形。depends_on を正しく。`crates:` は狭く)
- [x] 公開 API の変更や仕様の判断が要るものは、タスクの本文に「判断が要る」と書いた
- [x] 番号: docs/tasks/README.md の番号帯の規則に従う (エディタは T1607〜、ほかは空いている番号)
- [x] 起票したタスクの一覧 (ID・題名・元の候補) をメモに書いた
- [x] `node scripts/tasks.mjs` が通る (未知の依存が無い)
- [x] `just ci` が通る

## 参考

- docs/tasks/README.md、docs/workflow/multi-agent.md#タスクの書き方-オーケストレーター向け

## メモ

- T0019 (1 回目) の後に、M9 の後続のタスクのワーカーが出した候補。番号の帯の 2 つ目の範囲の規則は T0019 で docs/tasks/README.md に足してある。

### やったこと

- オーケストレーターの一覧 (`candidates-for-T0026.md`、85 行) と、M9 の done のタスクのメモの「起票の候補」を読んだ。
  一覧に無いが done のタスクのメモにあった候補 (T0671 のツリーと法線マップの節・T0672 の `DressUp.load`・T0724 の「このアプリで開く」・
  T0835 の `-32010`・T0926 の 2 つ (main に後から入った)・T0021 の `[profile.dev]`・T0222 の 3 つ・T0693 の `GlueOverlay`) も扱った。
- 候補ごとに今のコードを見て、すでに直っているものを除いた (下の「起票しないもの」)。
- 番号は各帯の最大の番号の次から付けた (T0019 と同じ。帯の途中の空き番号は、閉じた・まとめたタスクの名残なので使っていない)。エディタは T1607〜T1616。

### 起票したタスク

| ID | 題名 | 元の候補 |
| --- | --- | --- |
| T0027 | CI に入っていない検査を足す (script-check・cargo doc の警告・ugoe-ai と ugoe-mcp の windows-gnu の clippy) | T0435・T0017 (cargo doc)・T0013 (windows-gnu) |
| T0028 | bindings/godot の build.mjs を mbx の PATH なしで流しても target/ に書けるようにする | T0456 (mbx) |
| T0029 | e2e: エディタの提案のプレビューと「Export Module…」の筋書き、見本を開く手順の時間切れの調査 | T0023・T0022 (時間切れ) |
| T0030 | ルートの [profile.dev] のデバッグ情報を line-tables-only にするか決める (判断が要る) | T0021 |
| T0031 | npm のライセンスの例外 (NPM_EXCEPTIONS と LICENSE_OVERRIDES) を 1 か所にまとめる | T0926 |
| T0164 | 公開仕様と spec-check の残り (検証するカーブの範囲・目パチの段の上限・拒まれる例の名前・文字列の enum の形。判断が要る) | T0326 (3 つ)・T0017 (enum) |
| T0165 | ParameterValuesError の LipSyncNaN・PartOpacityNaN を有限でない値の名前にする (判断が要る) | T0010 (改名) |
| T0166 | 光源の型を GPU に依存しない場所へ移し、C ABI と GDExtension から wgpu を外す (判断が要る) | T0156 |
| T0167 | ugoe-runtime の ComposeError に code() を足し、SDK と editor-core の合成のエラーのコードをそこから作る | T0163 (ComposeError) |
| T0168 | 着せ替えのエラーの補足に、検証のエラーのコードの一覧と、取り除いた物理の元のコードを載せる (判断が要る) | T0163 (DressUpError::fields)・T0015 (carry_over) |
| T0169 | assets/compat の未来の版のファイルに、未知の VertexOwner を含む物理を足す | T0140 (assets/compat) |
| T0170 | ugoe-format の check_motion・check_expression を公開し、editor-core の写しを置き換える | T1606 |
| T0223 | 参照 (ugoe render) と wasm で圧縮した KTX2 をデコードするか決める (依存の選定と ADR。判断が要る) | T0222 (圧縮した KTX2) |
| T0224 | render_png で片方だけ大きさを指定したときの上限と、ugoe mcp の --gpu-timeout (判断が要る) | T0221 (2 つ) |
| T0327 | TrackingMap が並びの違う ParameterValues を受け続けるときの毎フレームの引き直しをなくす (判断が要る) | T0325 |
| T0457 | C ABI のモデルの読み込みの失敗にも種類を付けて wasm の LoadError と揃え、wasm の着せ替えの GPU の喪失にコードを付ける | T0434 (C ABI・device_lost) |
| T0458 | Unity: 読み込みの上限を指定する口と、エラーの補足の値のキーを C ABI に揃える | T0152・T0455 (Unity の上限)・T0157 (FieldKeys)・T0452 (FieldKeys と codes.json) |
| T0459 | Godot: 読み込みの上限を指定する口・attach_module の 2 度読み・口の入力をやめる口 | T0152 (Godot)・T0456 (2 度読み)・T0454 (set_lip_sync_level) |
| T0460 | Godot: 描画を頂点だけの書き換え (mesh_surface_update_vertex_region) にするかを測って決める | T0450 (2 つ) |
| T0461 | エフェクトの設定を差し替える set_effects_json (C ABI・wasm・Godot) で設定を検証する | T0010 (set_effects_json) |
| T0462 | モーションのイベントを発火したモーション (名前と番号) を C ABI・wasm・Unity・Godot から読む (判断が要る) | T0821 |
| T0463 | @ugoe/runtime: MotionSampler を出すかと、本物の wasm で *_json() の出力のキーを型と比べる | T0451 (公開)・T0452 (*_json のキー) |
| T0556 | ugoe-mcp: HTTP の誤りの答え (408・413・431) の後に接続をすぐ閉じず、Windows のクライアントが答えを失わないようにする (優先度高め) | T0013 (lingering close) |
| T0557 | ugoe-mcp: ツールのエラーの構造 (code・args)・Undo の可否・確認の取り消しの種類を出す (判断が要る) | T0554 (structuredContent)・T1605 (can_undo)・T1603 (HostError::Cancelled) |
| T0558 | PSD の読み込みと自動メッシュのエラーに構造とコードを付け、エディタで訳す (判断が要る) | T0554 (PsdError の上限)・T0163 (MeshError・PsdError のコード) |
| T0559 | ugoe-cli: AI のコマンドに Ctrl+C での取り消しと時間の上限のオプションを付ける | T0541 (CLI) |
| T0560 | AI の法線マップとテクスチャの追加・差し替えで、PNG の一辺の上限を入口で確かめる (判断が要る) | T0162 (2 つ) |
| T0561 | 格子のセルと線形の添字の計算を 1 か所にまとめ、module_rig の引数を文脈の構造体にまとめる | T0549 (2 つ) |
| T0732 | ビューアを OS の「このアプリで開く」に対応させる (fileAssociations と macOS の RunEvent::Opened) | T0724 |
| T0836 | スタジオ: 受け手の読み込みの失敗を残らず知らせる (仮想カメラ・背景の画像・KTX2・GPU のエラーの理由) | T0825 (2 つ)・T0832・T0222 (スタジオ) |
| T0837 | スタジオ: パペットの組み立て (Conductor::present) を舞台のロックの外で行い、操作ウィンドウに読み込み中を出す | T0834 (2 つ) |
| T0838 | スタジオ: ファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞る | T0024 (スタジオ) |
| T0839 | スタジオの操作 API の -32010 の error.data に args と ADR 0029 のコードを足すか決める (判断が要る) | T0835 |
| T0934 | 読み込みの警告・エラーの訳の表を codes.json と比べるテストと、KTX2 の警告の訳文の直し | T0452 (訳の表)・T0222 (訳文) |
| T0935 | エディタと Player でも WebGL2 の喪失で復元を待つか、続けて失敗したときの諦め方を決める (判断が要る) | T0932 (喪失) |
| T0936 | PuppetSurface を本物の wasm とブラウザで確かめる自動テスト (判断が要る: 置き場所) | T0932 (自動テスト) |
| T0937 | アプリの localStorage の扱いを揃える (ビューアとスタジオの設定・テストの MemoryStorage・版の違う保存値。判断が要る) | T0930 (2 つ)・T0011 (ModelMemory の版) |
| T1607 | editor-core: プロジェクトとモジュールの読み込み・指紋でも、通常のファイルだけを大きさの上限付きで読む | T0011 (editor-core) |
| T1608 | エディタのコマンドで、物理の未知の出力先・対象を受け付けるか拒むかを揃える (判断が要る) | T0140 (コマンド) |
| T1609 | エディタ: モジュールの適用のエラーに直し方の案内を出し、keep_look の作り直しの警告を捨てない | T0678・T0667 |
| T1610 | エディタ: タイムラインの再生・シークを wasm の MotionSampler に切り替える | T0451 (タイムライン) |
| T1611 | エディタ: ビューポートとプレビューの読み込みの失敗を wasm の LoadError のコードから訳す | T0434 (エディタ) |
| T1612 | エディタ: スロットの欄・カーブのキーの時刻・グルーの組を doc.send で送り、パネルのドラッグの送り直しを点検する | T0695 (2 つ)・T0693 (2 つ)・T0699 (パネルのドラッグ) |
| T1613 | エディタ: 光源のドラッグと、読めないモデルで部品が止まらないことを部品の単位でテストする | T0699 (LightingOverlay)・T0698 |
| T1614 | エディタ: ツリーが畳んだ祖先を開き直す・法線マップの節の取り直し・着せ替えの比べる途中の結果を直す | T0671 (2 つ)・T0672 (DressUp.load) |
| T1615 | エディタ: 分類の結果 (ラベル・左右) を直し、食い違いを「これでよい」と認めて印を消す (判断が要る) | T0546 |
| T1616 | エディタ: ファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞る | T0024 (エディタ) |

依存は、同じファイルを触るタスクの衝突を避けるために付けたものがある (T0168→T0167、T0170→T1607、T0460→T0459、T0462→T0461、T0557→T0556、
T0837→T0836、T0838→T0837、T0839→T0838、T0934→T1611、T0936→T0935、T1613→T1612)。

### 既存のタスクに含めたもの (起票しない)

- T0025 (blocked、置き場所の判断待ち) のメモに書き足した: T0687 の 3 つ (一時ファイルの名前と rename の前に閉じる・`mcpb::Bundle::write` の `sync_all`・
  `save_project` をロックの外へ (T0025 で済んだ))、T0025 の `save_project_as`、T0730 の GPL の共有クレート (T0025 の案 (c) にあたる)、T0011 のアプリの共有クレート (案 (b) と同じ)。
- T0673 (todo、M10) のメモに書き足した: T0926 のアプリ全体のライセンスの画面で `third-party-notices.txt` を読む。
- T0674 (todo、M10) のメモに書き足した: T0920 のエディタの自前の部品の `ariaLabel` を `@ugoe/ui` の名前に揃える (任意)。
- T0010 の「AGENTS.md の『NaN と無限大』から docs/portability.md#ホストから来る数 へリンク」は 1 行なので、このタスクで AGENTS.md に足した (`crates:` の外の小さな変更)。

### 起票しないもの (すでに済んでいる・見送り)

- T0829 (ほかのアプリの `cfg(windows)`・`cfg(macos)` を `CHECKS` に): T0013 で `scripts/cross-check.mjs` の `apps` に入った。
- T0022 (e2e のエディタにプレビューの画像からのトラッキング): T0023 で入った (`lib/tracking.mjs` の `checkImageTracking`)。
- T0699→T0698 (`FakeDocument` に `parsedModel`): T0698 で入った。T0015・T0023・T0831 の `ToolOverlay.test.ts` の失敗も T0698 で直った。
- T0827 (スタジオの `set_virtual_camera_settings`・`start_virtual_camera` の `Result<_, String>`): T0835 で `CommandError` になった。
- T0730 (エディタの保存のダイアログのフィルタ名)・T0672 (`MODULE_FILTER` などのフィルタの翻訳): T0690 で `filePaths.ts` とカタログ (`fileMenu.filters.*`) に寄せた。
- T0015 の「`ProjectError::InvalidModule` の文を `describe_errors` に・`format.invalid_motion`/`invalid_expression` のコード・`message_codes` が `validation_errors()` で引数を作る」:
  T0163・T1606 で入った。T0170 のメモに書いた。
- 集計の「C# の `UgoeResult` に 10 が無い」(T0152): T0455 で `UgoeResult.LimitExceeded = 10` が入った。残りの上限の口は T0458。
- T0157 の bindings/web の `UgoeDressUpErrorFields` の新しいキー: T0452 で入った (`warnings.ts`)。Unity の側は T0458。
- T0541 の「`send_cancellable` が応答待ちごとにスレッドを作る」: ureq に中断の手段ができるのを待つ。T0559 のメモに書いた。
- T0454 の「web の高い層 (`<ugoe-avatar>`・プレイヤー) で指揮役になる API」: 候補自身が「用途があれば」としている。利用者の要望が出たら起票する。
- T0424 の `set_tracking` に決まった順の `Float32Array` を受ける口・T0439 の `<ugoe-avatar>` の `state`: T0019 と同じく見送り (必要が出たら)。

### 検査

- `git rebase main` (main は c459f63d、T0926 のマージの後) の後に `node scripts/tasks.mjs` (未知の依存なし・番号の重複なし) と `just ci` が通った。
