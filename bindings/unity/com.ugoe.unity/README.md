# ugoe for Unity

[ugoe](https://github.com/rosylilly/ugoe) のモデル (`.ugoe`) を Unity 2022.3 以降で表示し、動かすパッケージ。
評価はネイティブライブラリ (`ugoe_ffi`、C ABI) で行い、描画は Unity のメッシュとマテリアルで行う。

- GameObject に **Ugoe Puppet** のコンポーネントを付け、`.ugoe` のファイル (**Ugoe Model** として取り込まれる) を選ぶ。
- `.ugoemod` (着せ替えのモジュール) は **Ugoe Module** として取り込まれる。
- Package Manager から見本 **Basic** を取り込むと、動く見本のシーンがある。
- 色空間は Gamma でも Linear でもよい (Linear では中間のテクスチャに合成してから写し、参照実装と同じ色にする)。

ネイティブライブラリは `Runtime/Plugins/` に置く (リリースの tarball `com.ugoe.unity-<版>.tgz` には入っている。
Git のリポジトリには入っていないので、CI で作ったパッケージを使うか、自分でビルドする)。使い方とビルドの方法: https://github.com/rosylilly/ugoe/tree/main/bindings/unity

ライセンス: MIT (`LICENSE.md`)。
