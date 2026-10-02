---
id: T0809
title: スタジオ: 仮想カメラ (macOS の Camera Extension)
status: done
milestone: M8
crates: [apps/studio, docs, Cargo.toml, Cargo.lock]
depends_on: [T0807]
assignee: claude-worker-T0809
---

# T0809: スタジオ: 仮想カメラ (macOS の Camera Extension)

## 目的

T0807 の macOS 版。CoreMediaIO の Camera Extension (System Extension) で出す。

## 受け入れ条件

- [x] Camera Extension と、スタジオからの画の受け渡し (T0807 と同じ形)
- [x] 署名・公証・entitlement の手順を docs/release.md に書く (配布そのものは Release のマイルストーンで行う)
- [x] macOS の実機での確認は利用者に残す (手順を書く)
- [x] `just ci` が通る

## 参考

- T0807 のメモ

## メモ

### やったこと (2026-10-02)

- **Camera Extension (`apps/studio/vcam/macos/`、Swift):** 機器「ugoe studio」(UID 固定) に、出す側と受ける側 (「ugoe studio input」) の
  2 つのストリーム。書式は 1280×720・1920×1080 の NV12 ('420v'、BT.709)、30・60 fps。出す側が動いている間だけ、フレームレートごとに
  受ける側から 1 枚取り出し、最新の画を出す (無ければ黒。大きさが違えば VideoToolbox で拡大縮小)。`Info.plist` (Mach サービスは
  `<チーム ID>.app.ugoe.studio.camera`) と `Extension.entitlements` (サンドボックスとアプリグループ)。
- **スタジオからの受け渡し (`apps/studio/vcam/src/cmio.rs`):** objc2 の束縛で、プロセスで最初に始めるときに `OSSystemExtensionRequest` で
  拡張を有効にし (許可待ち・「アプリケーション」フォルダの外・失敗を理由のコードに)、CoreMediaIO で機器と受ける側を探して
  IOSurface の `CVPixelBuffer` → `CMSampleBuffer` を `CMSimpleQueue` に積む。キューが埋まっていれば `Delivery::NoConsumer`。
  画素の形は Windows と同じ NV12 (`pixel_layout`)。アプリの束に拡張が無ければ `Unsupported` (今の配布物と `just studio` は変わらない)。
  エラーのコード `macosTooOld`・`needsApproval`・`notInApplications`・`extensionFailed` と en/ja の文言、操作ウィンドウの対応。
- **ビルド:** `pnpm vcam:macos` (`scripts/build-vcam-macos.mjs`。xcrun swiftc で arm64 / x86_64 → lipo → 束を組み、プロファイルを入れて署名)、
  `src-tauri/tauri.macos-vcam.conf.json` (`--config` で渡す。`bundle.macOS.files` で拡張とスタジオのプロファイルを束に入れる)、
  `src-tauri/Entitlements.vcam.plist` (System Extension を入れる権限を足したもの)。既定の `tauri.conf.json` と `Entitlements.plist` は変えていない。
- **docs:** docs/studio.md の「macOS (Camera Extension)」(しくみ・コード・ビルド・開発中の確かめ方 (SIP・`systemextensionsctl developer on`・
  ログ)・未確認の 8 項目)、docs/release.md の「macOS の仮想カメラ」(用意する証明書・App ID・アプリグループ・プロファイル、entitlement、
  作る・署名を確かめる・公証する・利用者の手順)、apps/studio/README.md。

### 確かめたこと

- `just ci`。`cargo clippy -p ugoe-studio-vcam --target aarch64-apple-darwin -- -D warnings` と `x86_64-apple-darwin` (`rustup target add` した)。
  Windows (`cargo xwin clippy --target x86_64-pc-windows-msvc`) と Linux の vcam も変わらず通る。
- `ugoe-studio` (Tauri) の macOS 向けの `cargo check` は、依存の objc2-exception-helper が macOS の SDK で Objective-C をコンパイルするので
  Linux ではできない (スタジオ側は `pixel_layout` で NV12 を選ぶだけで、macOS 固有のコードは無い)。

### 確かめていないこと (利用者の確認に残す)

- Swift の拡張はコンパイルもしていない (macOS が無い)。docs/studio.md の「macOS (Camera Extension)」の未確認の 8 項目
  (ビルド・許可の流れ・各アプリで映る・色・受け取るアプリの有無・カメラの許可 (TCC) が要るか・tauri の署名が中の拡張を壊さないか・公証・入れ替え)。
- docs/release.md の署名・公証の手順 (Apple Developer のアカウントが要る)。

### 範囲外の変更

- ルートの `Cargo.toml` ([workspace.dependencies] に objc2・objc2-foundation・objc2-core-foundation・objc2-core-media・objc2-core-video・
  objc2-core-media-io・objc2-system-extensions・dispatch2) と `Cargo.lock`。crates: に足した。理由: System Extension の有効化
  (`OSSystemExtensionRequest` と delegate) は Objective-C の API で、CoreMediaIO・CoreMedia・CoreVideo の呼び出しも、実機で試せないので
  手書きの宣言より生成された型付きの束縛のほうが誤りが少ないため。どれも macOS のときだけ ugoe-studio-vcam が使う。版は Cargo.lock の
  既存の objc2 系 (objc2 0.6.4・0.3.2 の各クレート・dispatch2 0.3.1) とそろえ、default-features は切って要る機能だけを有効にした。
  ライセンスは Zlib OR Apache-2.0 OR MIT (`just licenses` が通る)。
- **利用者の判断 (2026-10-02):** 論点 1 は A (Swift のソースを apps/studio/vcam/macos/ に置き、node のスクリプトが swiftc で作って束を組む。
  Xcode のプロジェクトは置かない)。論点 2・3 は A (objc2 の束縛を macOS のときだけ ugoe-studio-vcam に依存させる。dispatch2 も同じ作者・
  ライセンスとして含める)。ルートの Cargo.toml と Cargo.lock の変更を承認。

### 後続の候補

- macOS の実機で確かめた結果に合わせて拡張とスタジオを直す。配布物に入れる (Release のマイルストーン。CI で macOS の束を作る・署名・公証)。
- 仮想カメラの使用の許可 (TCC) が要るなら `NSCameraUsageDescription` の文言を仮想カメラにも合うものにする。
- AGENTS.md のリポジトリ構成の `apps/studio/` の行 (「仮想カメラ…は M8 の後続のタスク」) を直す (範囲外なので触っていない)。

