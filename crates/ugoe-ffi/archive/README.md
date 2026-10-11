# ugoe の C ABI @VERSION@

ugoe のランタイムを C・C++ やほかの言語から使うためのライブラリとヘッダです。ライセンスは MIT (`LICENSE`)。
関数の使い方・失敗の受け取り方・ABI の版の規則は、リポジトリの `crates/ugoe-ffi/README.md`
(<https://github.com/rosylilly/ugoe/blob/main/crates/ugoe-ffi/README.md>) にあります。

## 中身

```
include/ugoe.h         ヘッダ (C11。C++ からも使える)
lib/                   ライブラリ (下の表)
examples/sample.c      見本 (.ugoe を読み、パラメータを動かして更新し、描画の状態を取り出す)
LICENSE
README.md
THIRD-PARTY-NOTICES.txt  ライブラリに入っている Rust のクレートのライセンスの表記
```

| アーカイブ | 共有ライブラリ | 静的ライブラリ |
| --- | --- | --- |
| `linux-x86_64`・`linux-aarch64` (glibc) | `libugoe_ffi.so.<ABI の版>` (soname も同じ) と、そこへのシンボリックリンク `libugoe_ffi.so` | `libugoe_ffi.a` |
| `windows-x86_64` (MSVC) | `ugoe_ffi.dll` と、リンクに使うインポートライブラリ `ugoe_ffi.dll.lib` | `ugoe_ffi.lib` |
| `macos-universal` (arm64 と x86_64) | `libugoe_ffi.<ABI の版>.dylib` (インストール名は `@rpath/libugoe_ffi.<ABI の版>.dylib`) と、そこへのシンボリックリンク `libugoe_ffi.dylib` | `libugoe_ffi.a` |

どちらか一方をリンクします。`<ABI の版>` はヘッダの `UGOE_ABI_VERSION` です。

Linux と macOS の共有ライブラリは、ABI の版を名前に入れています。`-lugoe_ffi` でリンクすると、実行ファイルには版の付いた名前
(`libugoe_ffi.so.<ABI の版>`・`@rpath/libugoe_ffi.<ABI の版>.dylib`) が残り、実行するときはその名前のファイルを読みます。
ABI を壊した新しい版のライブラリ (名前の版が違う) を同じ場所に入れても、古い版でビルドしたプログラムがそれを読むことはありません。
実行するときに置くのは版の付いた名前のファイルです (シンボリックリンクはリンクするときだけ要ります)。
読み込んだライブラリがヘッダと同じ ABI かは、`ugoe_abi_version()` と `UGOE_ABI_VERSION` を比べても確かめられます (Windows の DLL は
名前に版を持たないので、こちらで確かめます)。

## リンクの例

見本を `ugoe-ffi-@VERSION@-<プラットフォーム>/` の中でビルドする例です。動かすときは `.ugoe` のファイルを渡します
(`./sample model.ugoe`)。

Linux:

```sh
# 共有ライブラリ (実行するときに lib/ から読むよう rpath を付ける)
cc -std=c11 -I include examples/sample.c -L lib -lugoe_ffi -lm -Wl,-rpath,'$ORIGIN/lib' -o sample
# 静的ライブラリ (C ライブラリの数学・スレッド・動的読み込みも要る)
cc -std=c11 -I include examples/sample.c lib/libugoe_ffi.a -lm -lpthread -ldl -o sample
```

macOS:

```sh
# 共有ライブラリ
cc -std=c11 -I include examples/sample.c -L lib -lugoe_ffi -Wl,-rpath,@executable_path/lib -o sample
# 静的ライブラリ
cc -std=c11 -I include examples/sample.c lib/libugoe_ffi.a -framework CoreFoundation -o sample
```

Windows (clang。MSVC の `cl` でも同じライブラリを渡す):

```bat
rem 共有ライブラリ (実行するときは ugoe_ffi.dll を実行ファイルの隣か PATH に置く)
clang -std=c11 -D_CRT_SECURE_NO_WARNINGS -I include examples\sample.c lib\ugoe_ffi.dll.lib -o sample.exe
rem 静的ライブラリ (C ライブラリは DLL の形 (/MD) にそろえる)
clang -std=c11 -D_CRT_SECURE_NO_WARNINGS -fms-runtime-lib=dll -Wl,-nodefaultlib:libcmt -I include examples\sample.c lib\ugoe_ffi.lib ^
  -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -o sample.exe
```

## 入っていないもの

- `std` の無い環境 (ゲーム機・組み込み機器) 向けの静的ライブラリ。ターゲットごとに作るものなので、ソースから作ります
  (`crates/ugoe-ffi/README.md` の「std の無い環境」)。
- Android・iOS のライブラリ。Unity のパッケージ (`bindings/unity`) に入っています。
