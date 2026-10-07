/*
 * nostd.c の見本をホスト (Linux・macOS) で動かす入口。ファイルを読んで `ugoe_nostd_sample` に渡す。
 * std の無い形の静的ライブラリはアロケータとパニックの処理をホストの C ライブラリ (`malloc`・`free`・`abort` など) に任せるので、
 * ここではホストの C ライブラリがその役をする。`just ffi-check` がビルドして実行する (crates/ugoe-ffi/README.md#std-の無い環境)。
 *
 *   nostd-sample <model.ugoe>
 */
#include <stdio.h>
#include <stdlib.h>

#include "ugoe.h"

int ugoe_nostd_sample(const uint8_t *bytes, size_t len);

/*
 * ホスト向けに配られている Rust の core と compiler_builtins はパニックの巻き戻しありでビルドされていて、
 * `rust_eh_personality` を参照する (std の無い形は巻き戻さないので呼ばれない)。ホストで動かすこの見本だけで埋める。
 * 巻き戻しの無いターゲット (aarch64-unknown-none など) や、core を `panic=abort` でビルドし直す組み込み先では要らない
 * (crates/ugoe-ffi/README.md#std-の無い環境)。
 */
void rust_eh_personality(void);
void rust_eh_personality(void) {
    abort();
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <model.ugoe>\n", argv[0]);
        return 2;
    }
    FILE *f = fopen(argv[1], "rb");
    if (f == NULL) {
        perror(argv[1]);
        return 1;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    uint8_t *bytes = malloc(size > 0 ? (size_t)size : 1);
    if (bytes == NULL || size < 0 || fread(bytes, 1, (size_t)size, f) != (size_t)size) {
        fprintf(stderr, "failed to read %s\n", argv[1]);
        return 1;
    }
    fclose(f);
    int status = ugoe_nostd_sample(bytes, (size_t)size);
    free(bytes);
    if (status != 0) {
        fprintf(stderr, "%s: the std-less sample failed at step %d\n", argv[1], status);
        return 1;
    }
    printf("%s: ok (std-less static library)\n", argv[1]);
    return 0;
}
