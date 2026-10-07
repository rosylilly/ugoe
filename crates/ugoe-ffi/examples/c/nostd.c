/*
 * std の無い形 (feature `std` を外した静的ライブラリ) の C ABI の見本 (crates/ugoe-ffi/README.md#std-の無い環境)。
 * ゲーム機や組み込み機器を模して、C の標準ライブラリのヘッダ (stdio.h など) を使わずに書く (フリースタンディング)。
 * ファイルは読まず、`.ugoe` のバイト列はホストが渡す。
 *
 * - `just nostd-check` は aarch64-unknown-none 向けにコンパイルして静的ライブラリとリンクし、未解決のシンボルが
 *   ホストの C ライブラリのもの (`malloc`・`free`・`abort` など) だけであることを確かめる。
 * - `just ffi-check` はホスト (Linux・macOS) 向けの std の無い形の静的ライブラリとリンクし、nostd_main.c から実行する。
 */
#include "ugoe.h"

/* NUL 終端の文字列が等しいか (string.h を使わない)。 */
static bool same(const char *a, const char *b) {
    if (a == NULL || b == NULL) {
        return false;
    }
    while (*a != '\0' && *a == *b) {
        a++;
        b++;
    }
    return *a == *b;
}

/*
 * `bytes` の `.ugoe` を読み、パラメータを動かして 1 フレーム進め、毎フレームの状態を読む。
 * テクスチャのデコードは、std の無い形では PNG と BCn の KTX2 を `"unsupported_texture"` で断る (ホストのデコーダで読む)。
 * 成功なら 0、失敗した段階の番号 (1 から) を返す。
 */
int ugoe_nostd_sample(const uint8_t *bytes, size_t len) {
    if (ugoe_abi_version() != UGOE_ABI_VERSION) {
        return 1;
    }
    UgoeError *err = ugoe_error_create();
    if (err == NULL) {
        return 2;
    }
    int status = 0;
    UgoePuppet *puppet = NULL;
    uint32_t parameters = 0, drawables = 0, textures = 0;
    if (ugoe_puppet_create_from_bundle(bytes, len, &puppet, err) != UGOE_OK ||
        ugoe_puppet_parameter_count(puppet, &parameters, err) != UGOE_OK ||
        ugoe_puppet_drawable_count(puppet, &drawables, err) != UGOE_OK ||
        ugoe_puppet_texture_count(puppet, &textures, err) != UGOE_OK || drawables == 0) {
        status = 3;
        goto done;
    }

    for (uint32_t i = 0; i < parameters; i++) {
        if (ugoe_puppet_set_parameter(puppet, i, 0.5f, err) != UGOE_OK) {
            status = 4;
            goto done;
        }
    }
    UgoeFrame frame;
    for (int step = 0; step < 3; step++) {
        if (ugoe_puppet_update(puppet, 1.0f / 60.0f, err) != UGOE_OK ||
            ugoe_puppet_frame(puppet, &frame, err) != UGOE_OK || frame.drawable_count != drawables ||
            frame.positions_len == 0) {
            status = 5;
            goto done;
        }
    }

    for (uint32_t t = 0; t < textures; t++) {
        UgoeTexture texture;
        if (ugoe_puppet_texture(puppet, t, &texture, err) != UGOE_OK) {
            status = 6;
            goto done;
        }
        if (texture.format == UGOE_TEXTURE_NONE) {
            continue;
        }
        UgoeImage *image = NULL;
        UgoeResult r = ugoe_image_decode(texture.format, texture.data, texture.len, NULL, true, &image, err);
        if (r == UGOE_OK) {
            ugoe_image_destroy(image);
        } else if (r != UGOE_ERROR_FORMAT || !same(ugoe_error_kind(err), "unsupported_texture")) {
            status = 7;
            goto done;
        }
    }

    /* 失敗の中身は呼び出し側の UgoeError から読む (スレッドローカルを使わない)。 */
    UgoePuppet *broken = NULL;
    if (ugoe_puppet_create_from_bundle(bytes, len / 2, &broken, err) != UGOE_ERROR_FORMAT ||
        broken != NULL || ugoe_error_message(err) == NULL ||
        !same(ugoe_error_kind(err), "invalid_file")) {
        status = 8;
        goto done;
    }

done:
    ugoe_puppet_destroy(puppet);
    ugoe_error_destroy(err);
    return status;
}
