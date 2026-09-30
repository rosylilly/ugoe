/*
 * ugoe の C ABI の小さな見本。`.ugoe` バンドルを読み、パラメータを動かして更新し、頂点を取り出す。
 * ビルドと実行は `just ffi-check` (crates/ugoe-ffi/README.md)。
 *
 *   sample <model.ugoe>
 *
 * 期待どおりに動かなければ 0 以外で終わる。
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "ugoe.h"

/* 呼び出しが失敗したら、エラーメッセージを出して終わる。 */
#define CHECK(call)                                                                \
    do {                                                                           \
        UgoeResult check_result_ = (call);                                         \
        if (check_result_ != UGOE_OK) {                                            \
            const char *message_ = ugoe_last_error();                              \
            fprintf(stderr, "%s:%d: %s failed (%d): %s\n", __FILE__, __LINE__, #call, \
                    (int)check_result_, message_ ? message_ : "(no message)");     \
            exit(1);                                                               \
        }                                                                          \
    } while (0)

#define EXPECT(cond)                                                               \
    do {                                                                           \
        if (!(cond)) {                                                             \
            fprintf(stderr, "%s:%d: expected %s\n", __FILE__, __LINE__, #cond);    \
            exit(1);                                                               \
        }                                                                          \
    } while (0)

static unsigned char *read_file(const char *path, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        perror(path);
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    unsigned char *bytes = malloc(size > 0 ? (size_t)size : 1);
    if (!bytes || fread(bytes, 1, (size_t)size, f) != (size_t)size) {
        fprintf(stderr, "%s: could not read\n", path);
        fclose(f);
        free(bytes);
        return NULL;
    }
    fclose(f);
    *len = (size_t)size;
    return bytes;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <model.ugoe>\n", argv[0]);
        return 2;
    }

    /* ヘッダとライブラリの ABI が一致すること。 */
    EXPECT(ugoe_abi_version() == UGOE_ABI_VERSION);
    printf("ugoe %s (ABI %u)\n", ugoe_version(), ugoe_abi_version());

    /* 壊れた入力はエラーコードとメッセージで返る。 */
    UgoePuppet *puppet = NULL;
    const unsigned char garbage[] = "not a bundle";
    EXPECT(ugoe_puppet_create_from_bundle(garbage, sizeof garbage, &puppet) == UGOE_ERROR_FORMAT);
    EXPECT(puppet == NULL);
    EXPECT(ugoe_last_error() != NULL);
    printf("rejected garbage: %s\n", ugoe_last_error());

    size_t len = 0;
    unsigned char *bytes = read_file(argv[1], &len);
    if (!bytes) {
        return 1;
    }
    CHECK(ugoe_puppet_create_from_bundle(bytes, len, &puppet));
    free(bytes); /* 読み込みの後はバイト列を解放してよい。 */

    UgoeCanvas canvas;
    CHECK(ugoe_puppet_canvas(puppet, &canvas));
    uint32_t parameters = 0, drawables = 0, textures = 0, motions = 0;
    CHECK(ugoe_puppet_parameter_count(puppet, &parameters));
    CHECK(ugoe_puppet_drawable_count(puppet, &drawables));
    CHECK(ugoe_puppet_texture_count(puppet, &textures));
    CHECK(ugoe_puppet_motion_count(puppet, &motions));
    printf("canvas %.0fx%.0f, %u parameters, %u drawables, %u textures, %u motions\n",
           canvas.width, canvas.height, parameters, drawables, textures, motions);
    EXPECT(drawables > 0);

    /* すべてのパラメータを最大値にして 1 秒ぶん進める。 */
    for (uint32_t i = 0; i < parameters; i++) {
        UgoeParameterInfo info;
        CHECK(ugoe_puppet_parameter_info(puppet, i, &info));
        CHECK(ugoe_puppet_set_parameter(puppet, i, info.max));
    }
    for (int frame = 0; frame < 60; frame++) {
        CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));
    }

    /* 奥から順に、借りた頂点とコピーした頂点が一致することを確かめる。 */
    const uint32_t *order = NULL;
    size_t order_len = 0;
    CHECK(ugoe_puppet_render_order(puppet, &order, &order_len));
    EXPECT(order_len == drawables);
    size_t visible = 0, vertices = 0;
    for (size_t k = 0; k < order_len; k++) {
        uint32_t d = order[k];
        UgoeDrawableInfo info;
        UgoeDrawableState state;
        CHECK(ugoe_puppet_drawable_info(puppet, d, &info));
        CHECK(ugoe_puppet_drawable_state(puppet, d, &state));
        EXPECT(info.texture < textures);

        const float *borrowed = NULL;
        size_t floats = 0;
        CHECK(ugoe_puppet_positions(puppet, d, &borrowed, &floats));
        EXPECT(floats == 2 * (size_t)info.vertex_count);
        float *copied = malloc((floats > 0 ? floats : 1) * sizeof(float));
        CHECK(ugoe_puppet_copy_positions(puppet, d, copied, floats));
        EXPECT(floats == 0 || memcmp(borrowed, copied, floats * sizeof(float)) == 0);
        for (size_t v = 0; v < floats; v++) {
            EXPECT(isfinite(copied[v]));
        }
        free(copied);

        if (floats > 0) {
            float too_small[1];
            EXPECT(ugoe_puppet_copy_positions(puppet, d, too_small, 1) == UGOE_ERROR_BUFFER_TOO_SMALL);
        }
        if (state.visible) {
            visible++;
        }
        vertices += info.vertex_count;
    }
    printf("%zu visible drawables, %zu vertices\n", visible, vertices);
    EXPECT(visible > 0);

    /* 範囲外の番号はエラーになる。 */
    UgoeDrawableState state;
    EXPECT(ugoe_puppet_drawable_state(puppet, drawables, &state) == UGOE_ERROR_OUT_OF_RANGE);

    ugoe_puppet_destroy(puppet);
    printf("ok\n");
    return 0;
}
