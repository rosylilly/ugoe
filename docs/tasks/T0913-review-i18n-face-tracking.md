---
id: T0913
title: コードレビュー: @ugoe/i18n と @ugoe/face-tracking
status: done
milestone: M9
crates: [packages, docs]
depends_on: []
assignee: claude-worker-T0913
---

# T0913: コードレビュー: @ugoe/i18n と @ugoe/face-tracking

## 目的

@ugoe/i18n と @ugoe/face-tracking のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `packages/i18n` と `packages/face-tracking` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 翻訳のカタログのキーの過不足 (日英)、複数形と引数、言語の切り替えでの再描画
  - カメラの資源の解放・推論のループの停止、third-party-notices の網羅
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0925〜T0929 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `face-tracking/src/tracker.svelte.ts:411` (`#start`) | 始める途中 (モデル・カメラ・ファイルを待つ間) に `stop()` しても、待ち終えると `running` になり回り始める (パネルは読み込み中に「止める」を出すので、止めたのにカメラが点いたままになる)。2 回続けて始めると、先のカメラのストリームを止めずに上書きし (カメラが点いたまま)、入力の処理が 2 つ回る。失敗した古い開始が、新しい入力の資源を解放することもあった | 直した (`stop()` で世代を進め、開いた資源は開き終えるまで `#start` が持ち、止められていれば解放する。JSON を読む間に別の入力を始めたときも同じ。`tracker.test.ts` を追加) |
| 中 | `face-tracking/src/tracker.svelte.ts` (`startReplay`) | 記録の読み込みの失敗を `parseRecording` の英語の文のまま画面に出していた (ADR 0029) | 直した (`errors.recordingInvalid` を足し、今の言語の文に元の文を付ける) |
| 中 | `face-tracking/src/recording.ts` | `frames` の要素や `head` が `null` だと `TypeError` (`Cannot read properties of null`) になり、どのフレームが壊れているか分からない | 直した (`frames[i] is invalid` にする。テストを追加) |
| 中 | `face-tracking` のテスト | `FaceTracker` (開始・停止・再生) のテストが無かった (Svelte のプラグインを通していないので `.svelte.ts` を読めなかった) | 直した (`vitest.config.ts` と devDependency を足し、記録の再生で開始と停止を確かめる) |
| 中 | `i18n/scripts/check-catalogs.mjs` | カタログの JSON が `null` だと `TypeError` で検査が止まり、配列は空のカタログとして通っていた | 直した (オブジェクトでなければエラーにする。テストを追加) |
| 中 | `apps/editor` (`preview/LicenseDialog.svelte`)、`apps/*/public/third-party-notices.txt` | エディタは MediaPipe の表記しか同梱せず、バンドルに入る MIT の JS ライブラリ (`@ugoe/i18n` の i18next など) の表記が無い。ビューア・スタジオは MediaPipe の節と依存の版を手で写していて、更新しても食い違いに気づけない | T0926 を起票 (範囲の外) |
| 低 | `apps/{editor,viewer,studio}` の言語の設定 | 言語の設定の保存・読み込みと、OS の言語を `setSystemLanguages()` に渡す処理が 3 つのアプリで重複している (テーマは T0909 で `@ugoe/ui` にまとめた) | T0925 を起票 (範囲の外・API の追加) |
| 低 | `i18n/README.md`、`face-tracking/README.md`・`src/i18n.ts`、`face-tracking/third-party-notices.txt` | 「OS の言語は後続のタスクで渡す」「エディタは英語、ビューアは日本語」が古い。表記の文がスタジオを挙げていない | 直した |
| 低 | `i18n/src/i18n.svelte.ts:91` (`registerCatalogs`) | 「登録し直すと置き換える」とあるが、実際は深く併合する (後のカタログに無いキーは残る) | 直した (doc コメントを実際の動きに合わせた) |
| 低 | `face-tracking/src/pose.ts:71` | `Math.min(...配列)` (要素が多いと JavaScriptCore で例外。並行のレビューからの指摘) | 見送り (配列は固定の 2 点の visibility だけで、大きくならない) |
| 低 | `face-tracking/src/tracker.svelte.ts:616` | 記録は止めるまで上限なく溜まる (1 時間で 10 万フレーム程度) | 見送り (利用者が止める操作。上限を決めるなら仕様の判断が要る) |
| 低 | `face-tracking/src/tracker.svelte.ts:244` (`refreshCameras`) | `enumerateDevices` が失敗すると、カメラは開けたのに開始全体が失敗する。カメラの名前 (`Camera 1`) は一覧を作った時の言語のまま | 見送り (起きにくい。名前は次に一覧を作り直すと今の言語になる) |
| 低 | `face-tracking/src/tracker.svelte.ts:659`、`i18n/src/format.ts:8` | 毎フレーム `DrawingUtils` を作る。`formatNumber` などは呼ぶたびに `Intl.*Format` を作る (パネルの再生位置は毎フレーム) | 見送り (測れるほどの負荷ではない。気になれば言語と書式で覚える) |
| 低 | `face-tracking/src/TrackingPanel.svelte:52` | `attachCanvas` の `$effect` に後片付けが無く、パネルを外しても canvas を持ち続ける | 見送り (パネルは外すときに `stop()` し、次のパネルが付け替える。描く先が無いので害が無い) |
| 低 | `i18n/scripts/check-catalogs.mjs:67` (`sourceStrings`) | ソースの文字列を正規表現で拾うので、コメントの中の引用符・逆引用符で対が崩れることがある | 見送り (崩れても「使われていないキー」の誤検出として表に出る。キーを見落とす向きには崩れにくい) |
| 低 | `i18n/src/i18n.svelte.ts` (i18next の設定) | `nsSeparator` が既定の `:` のままなので、`:` を含むキー (エラーのコードを組み立てたものなど) は名前空間として分かれる | 見送り (今のキーとコードに `:` は無い) |
| 低 | `i18n/src/i18n.svelte.ts:38` | `navigator.languages` (凍結した配列) を `$state` の深いプロキシに入れる | 見送り (いつも丸ごと置き換え、読むだけなので害が無い。`$state.raw` で足りる) |
| 低 | `face-tracking/scripts/mediapipe-assets.mjs:87` | モデルの取得にタイムアウトが無い (`--optional` の開発でも、ネットワークが固まると `pnpm dev` が止まる) | 見送り (名前の解決に失敗するふつうのオフラインではすぐ失敗する) |

確かめたこと (観点の追加分):

- カタログ: `just i18n` で日英のキーの過不足・複数形・引数がそろう (face-tracking は複数形を使わない)。`recordingInvalid` を足した後も通る。
- 言語の切り替え: パネルの文は `t()` をテンプレートと `$derived` で呼ぶので追従する (`i18n.svelte.test.ts` の再評価のテスト)。例外の文・カメラの名前は作った時の言語 (README に書いてある)。
- カメラの資源と推論のループ: `stop()` でトラックを止め、映像を外し、オブジェクト URL を戻し、`schedule` の止める関数を呼ぶ。始める途中の停止と重なった開始の穴は上の表のとおり直した。
- third-party-notices: MediaPipe の wasm・JS・2 つのモデルを網羅している。アプリ側の表記の穴は T0926。

### 横断

- 非同期の開始と停止の競合 (待つ間に止めても、待ち終えた処理が状態を `running` にし資源を握る) は、スタジオのマイク (T0815 で直した) とこのトラッカーの両方にあった。`await` をまたぐ開始の処理は、世代 (またはキャンセルの印) を持ち、待った後に確かめる形をほかのアプリ・パッケージでも確かめるとよい。
- 例外の文をそのまま画面に出す箇所で、ライブラリ (ここでは `parseRecording`) の英語の文が利用者に見える。画面に出す側で今の言語の前置きを付けるか、コードで返す。
- 同梱物のライセンスの表記が、アプリごとに手で書かれ、版も手で書かれている (T0926)。
- `.svelte.ts` を含むパッケージのテストには Svelte のプラグインが要る (face-tracking は純粋な関数のテストしか無かったので、`FaceTracker` を試せていなかった)。

`just ci`: 1 回目は `web-check` の中のエディタの `src/lib/preview/tracking.test.ts` が `beforeAll` の動的 import で 10 秒のタイムアウトになった
(並行のビルドでロードアベレージが 47 のとき。テストは 0 件実行)。同じテストを単独で流すと 7 件とも通り、`fmt-check` から `spec` までは通っていたので、
残りの `just web-check i18n app-build-check` を流し直して通った。
