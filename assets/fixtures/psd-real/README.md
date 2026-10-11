# 実際のペイントツールの PSD

Photoshop・CLIP STUDIO PAINT・Krita・SAI などが書き出した PSD を置く場所です。
ここに置いた `*.psd` は `crates/ugoe-psd/tests/real_world.rs` が自動でテストします (ファイルが無ければ何もしません)。

- 8/16 ビット RGB の PSD: 取り込みが成功すること (警告はあってよい)。ImageMagick (`identify`) があれば、
  画素を持つレイヤーの数が ImageMagick の見方と一致すること
- それ以外 (PSB・32 ビット・グレースケール・CMYK など): `PsdError::Unsupported` になること

実行: `cargo test -p ugoe-psd --test real_world -- --nocapture` (警告も表示される)

## 追加の手順

1. **再配布できるファイルだけを置く。** 自分で描いたもの、または CC0 / CC BY / MIT などの再配布を許すライセンスの
   ものに限る。ライセンスが分からないもの・再配布を禁じるもの・他人の作品の無断転載はコミットしない。
   生成 AI で作った絵は、生成に使った道具 (サービス・モデル・分けたツール) を表の「出所」に書く。
2. 1 ファイル 1MB 以下 (`docs/testing.md`)。大きければキャンバスを縮めて保存し直す。縮めると試したい性質
   (実際の大きさ・レイヤーの数) が失われる場合は、理由を表の「メモ」に書いて超えてよい。
3. ファイル名は `<ツール>-<バージョン>-<内容>.psd` (例: `krita-5.2-groups-masks.psd`)。
4. 下の表に 1 行足す: 書き出したツールとバージョン、含まれる機能 (グループ・マスク・クリッピング・
   16 ビットなど)、作者、ライセンス、出所 (URL か「自作」)。
5. テストが失敗したら、その内容を読み込みの不具合としてタスクにする (テストを弱めない)。
   ImageMagick 側の読み方の違いでレイヤーの数が合わない場合は、理由をこの表の「メモ」に書いて相談する。

下の 2 つの PSD は生成 AI で作った絵で、利用者の決定 (2026-10-11) で、このまま MIT としてリポジトリとともに公開する。
出所 (生成に使った道具) は、どちらも同じ: 絵は ChatGPT (OpenAI) の画像の生成で作り、ComfyUI の See-Through でパーツのレイヤーに
分けて PSD に書き出した。

| ファイル | ツール / バージョン | 含まれる機能 | 作者 / ライセンス / 出所 | メモ |
| --- | --- | --- | --- | --- |
| `comfyui-see-through-full-body-parts.psd` | ChatGPT で生成した画像を ComfyUI の See-Through でパーツに分けて書き出したもの / 版は不明 (ファイルにツールの記録がない) | 1024x1024 の 8 ビット RGB、フラットな 23 レイヤー (グループ・マスク・クリッピングなし)、英語のレイヤー名 (`back hair`・`face`・`irides-l`・`front hair` など)、全身の立ち絵 | AI 生成 (ChatGPT の画像の生成 → ComfyUI の See-Through。リポジトリの管理者が用意) / MIT (利用者の決定 2026-10-11) / 管理者の提供 | 1.8MB で上限を超えるが、実際の大きさと部位の数を試すため縮めずに置く (管理者の判断)。ugoe の描画は ImageMagick の合成画像と PSNR 約 42 dB で一致 |
| `comfyui-see-through-bustup-parts.psd` | ChatGPT で生成した画像を ComfyUI の See-Through でパーツに分けて書き出したもの / 版は不明 (ファイルにツールの記録がない) | 1024x1024 の 8 ビット RGB、24 レイヤー。グループ `body` と `face` (中に `ear-r`・`ear-l`・`mouth`・`eye-r`・`eye-l`)、レイヤー効果 (`earwear-r`・`eyeclose-r`。取り込みでは無視して警告)。胸から上の絵で、目は開き (`eyewhite-*`・`irides-*`・`eyelash-*`) と閉じ (`eyeclose-*`)、口は `mouth-open` と `mouth-close` の別のレイヤー (合成画像は両方が見えた状態) | AI 生成 (ChatGPT の画像の生成 → ComfyUI の See-Through。利用者が用意) / MIT (利用者の決定 2026-10-11) / 利用者の提供 | 見本のモデル (`assets/samples/avatar.ugoe`) の元。3.4MB で上限を超えるが、目と口の開き・閉じのレイヤーを実際の大きさで使うため縮めずに置く (利用者の判断。全身の PSD と同じ)。ugoe の描画は ImageMagick の合成画像と PSNR 約 69 dB で一致 |
