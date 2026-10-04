---
id: T0914
title: コードレビュー: @ugoe/player
status: done
milestone: M9
crates: [packages, docs]
depends_on: []
assignee: claude-worker-T0914
---

# T0914: コードレビュー: @ugoe/player

## 目的

@ugoe/player のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `packages/player` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - wasm と WebGL の資源の解放、再生のループの停止、3 つのアプリからの使われ方とのずれ
  - モーション・トラッキング・ライティングの状態の持ち方
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0930〜T0934 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/player.svelte.ts` `attach`・`#createPuppet` | WebPuppet を作っている間に `detach` や次の `attach` があると、遅れてできた WebPuppet が今のものを上書きし、前のものと canvas が解放されない。`detach` の後でも `requestAnimationFrame` のループが始まる (ビューアとスタジオの onMount と後片付けが重なると起きる。エディタ T0672 と同じ形) | 直した (`#attachToken` で古い作成を捨てる。`attach` は先に `detach` する。テストを追加) |
| 高 | `src/player.svelte.ts` `#tick`・`#recover` | wgpu の GL は WebGL2 のコンテキストの喪失を `is_lost` で知らせないので、WebGL2 で描いているとビューポートが真っ白のまま戻らない | 直した (canvas の `webglcontextlost` で canvas ごと作り直す。`requestAnimationFrame` が止まっていても作り直す。テストを追加。ugoe-wasm 側の口は T0697) |
| 中 | `src/player.svelte.ts` `load` | 開けなかったファイル (読み込みの失敗・モデルでないファイル) でも `#file` を書き換えるので、次に GPU を作り直すと、表示していたモデルではなく開けなかったファイルを読み直してモデルが消える | 直した (読み込めたときだけ `#file` を変える。テストを追加) |
| 中 | `src/player.svelte.ts` `resize` | devicePixelRatio だけが変わったとき (倍率の違う画面へ移した) は ResizeObserver が知らせないので、描画バッファが古い倍率のままでぼやける (エディタ T0672 と同じ) | 直した (Player が `matchMedia` で倍率の変化を聞き、`resize` し直す。ビューアとスタジオの両方に効く。テストを追加) |
| 中 | `src/player.svelte.ts` `#recover` | 作り直しが例外で抜けると `#recovering` が真のまま残り、二度と作り直さない。作り直している間に `detach` されても値を書き戻そうとする | 直した (`try`/`finally` と `#attachToken` の確認) |
| 中 | `src/player.svelte.ts` | Player (約 1300 行) に、作成と破棄のほかのテストが無い (着せ替えの組を戻す流れ・モーションの繰り返しと待機・受け手のモードの適用・光源の確定) | T0931 を起票 |
| 中 | `src/player.svelte.ts`・`apps/editor/src/lib/viewport/controller.svelte.ts` | WebPuppet の作成 (WebGL2 への切り替え)・喪失からの作り直し・古い作成の破棄・倍率の追従を、エディタと Player がそれぞれ持ち、同じ誤りが両方に入っていた | T0932 を起票 |
| 低 | `src/player.svelte.ts` `#follow` | 「受け手の口が無い」エラーが英語の直書き (ADR 0029) | 直した (`errors.noFollowerApi` をカタログに足した) |
| 低 | `src/animation.ts`・`lighting.ts`・`dressUpMemory.ts`・`rememberedModels.ts` | localStorage のキーが `ugoe-viewer.*` に固定 (README の「接頭辞をアプリが渡せるようにする」が未実装で TODO の印も無い)。「try の中で引いて読めなければ既定」の小さな関数がファイルごとに重複 | T0930 を起票 |
| 低 | `src/player.svelte.ts` `#recallIdleMotion` | 受け手のモードでも、覚えた待機モーションの選択を最近使ったものに書き直す (受け手は待機モーションを使わない) | T0930 に含めた |
| 低 | `src/player.svelte.ts` `#advance`・`#syncValues` | 毎フレーム `Set` と配列を作り、パラメータごとに `puppet.parameter(id)` で wasm の境界を越える | 見送り (ビューアの UI の層で、今の大きさのモデルでは問題にならない。まとめて読む口が要れば ugoe-wasm 側の変更) |
| 低 | `src/player.svelte.ts` `#afterMotionUpdate` | 繰り返しの `start_motion` だけ例外を受けていない (`#startIdle` は受ける) | 見送り (始められたモーションの名前なので投げない。`#tick` は先に次のフレームを頼むのでループは止まらない) |
| 低 | `src/player.svelte.ts` `detach` | `detach` の後も `status` が `ready` のまま | 見送り (使う側は detach の後に Player を捨てる。再 attach すると作り直しで更新される) |
| 低 | `src/lighting.test.ts` | テストの名前に日本語と英語が混ざる | 見送り (好みの範囲) |

確かめたこと (受け入れ条件の追加分):

- wasm と WebGL の資源の解放: `detach`・作り直しで `free` と canvas の削除、`webglcontextlost` の聞き取りの解除を行う。`capture` の
  `read_pixels` は約束が資源を持つ (`crates/ugoe-wasm/src/web/readback.rs`) ので、待っている間に解放しても壊れない。
- 再生のループの停止: `detach` で `cancelAnimationFrame`。古い `attach` の後でループを始めないようにした。
- 3 つのアプリからの使われ方: ビューア (`apps/viewer/src/lib/Viewport.svelte`) とスタジオの出力 (`apps/studio/src/Output.svelte`) は
  onMount で `attach`、後片付けで `detach`。エディタは Player を使わず自前の ViewportController を持つ (T0932)。スタジオの受け手は
  光源を `preview` で当て、`load` の前の `commitLighting` は受け手では覚えない。ずれは見つからなかった。
- 状態の持ち方: モーション (`currentMotion`・`#currentPriority`・待機)、トラッキング (`#pendingTracking`・`#lastTracking` を読み直しで渡し直す)、
  ライティング (最後に使った設定・モデルごとの設定・ドラッグの途中) は、モデルの読み直しと GPU の作り直しで整合する。
  受け手のモードのフレームは、作り直している間は待たせ、作り直した後で順に適用する。

テスト: `src/player.test.ts` は `@ugoe/face-tracking` を `vi.mock` する (画面の部品と MediaPipe まで読むと遅く、`document` を
置いた後に読むと bits-ui が `window` を引く)。偽の WebPuppet は Proxy で、`then` は返さない (約束の値として解くため)。

`just ci`: 通った (rebase main の後)。

### 横断

- wgpu の GL のバックエンドは WebGL2 のコンテキストの喪失を `is_lost` で知らせない。WebPuppet を使う部品はすべて canvas の
  `webglcontextlost` を聞く必要がある (エディタ T0672・Player T0914。ugoe-wasm の口は T0697)。`bindings/web` の `<ugoe-avatar>` も同じはず。
- 非同期の開始 (wasm の読み込み・WebPuppet の作成・カメラやマイクの開始) と停止が重なると、遅れて終わった開始が今のものを上書きして
  資源が残る。開始ごとに番号を持ち、待った後で確かめる形を決まりにする (エディタ T0672・Player T0914・スタジオのマイク T0815・face-tracking T0913)。
- devicePixelRatio だけの変化は ResizeObserver が知らせない。canvas に描く部品はすべて `matchMedia` で聞く。
- localStorage を「try の中で引いて読めなければ既定」で読む小さな関数が、パッケージとアプリごとに重複している (player・studio の settings.ts など)。

