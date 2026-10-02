/*
 * ugoe の C ABI の小さな見本。`.ugoe` バンドルを読み、パラメータを動かして更新し、頂点を取り出す。
 * 読み込みの警告・当たり判定・トラッキング・パラメータの一括設定・表情の解除・ライティング・着せ替えも使う。
 * ビルドと実行は `just ffi-check` (crates/ugoe-ffi/README.md)。
 *
 *   sample [--lit] <model.ugoe> [module.ugoemod ...]
 *
 * --lit を付けると、ライティングを受けるモデルで法線マップがあることを確かめる。
 * モジュールを渡すと、順に付けて外す。期待どおりに動かなければ 0 以外で終わる。
 */
#include <math.h>
#include <stdbool.h>
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

/* 読み込みの警告を出す。 */
static void print_warnings(UgoePuppet *puppet) {
    uint32_t count = 0;
    CHECK(ugoe_puppet_load_warning_count(puppet, &count));
    for (uint32_t i = 0; i < count; i++) {
        UgoeWarning warning;
        CHECK(ugoe_puppet_load_warning(puppet, i, &warning));
        printf("warning [%s]: %s\n", warning.code, warning.message);
    }
}

/* 当たり判定の一覧と、キャンバスの中ほどで当たるもの。 */
static void hit_test(UgoePuppet *puppet, const UgoeCanvas *canvas) {
    uint32_t count = 0;
    CHECK(ugoe_puppet_hit_area_count(puppet, &count));
    float x = canvas->width / 2, y = canvas->height / 2;
    size_t hits = 0;
    /* まず数を聞いてから、足りるバッファで受け取る。 */
    UgoeResult r = ugoe_puppet_hit_areas_at(puppet, x, y, NULL, 0, &hits);
    EXPECT(r == UGOE_OK || r == UGOE_ERROR_BUFFER_TOO_SMALL);
    uint32_t *list = malloc((hits > 0 ? hits : 1) * sizeof(uint32_t));
    CHECK(ugoe_puppet_hit_areas_at(puppet, x, y, list, hits, &hits));
    printf("%u hit areas, %zu at (%.0f, %.0f)", count, hits, x, y);
    for (size_t i = 0; i < hits; i++) {
        const char *id = NULL;
        CHECK(ugoe_puppet_hit_area_id(puppet, list[i], &id));
        printf(" %s", id);
    }
    printf("\n");
    free(list);

    /* 標準の名前 (アバタープロファイル) での判定。対応が無ければ false。 */
    bool head = true;
    CHECK(ugoe_puppet_hit_test_standard(puppet, "Head", x, y, &head));
    uint32_t index = 0;
    if (ugoe_puppet_standard_hit_area(puppet, "Head", &index) == UGOE_OK) {
        const char *id = NULL;
        CHECK(ugoe_puppet_hit_area_id(puppet, index, &id));
        bool hit = false;
        CHECK(ugoe_puppet_hit_test(puppet, id, x, y, &hit));
        EXPECT(hit == head);
        printf("standard Head -> %s (%s)\n", id, head ? "hit" : "miss");
    }
}

/* フェイストラッキングの入力を 1 フレーム渡す。 */
static void track(UgoePuppet *puppet) {
    UgoeTrackingInput input;
    memset(&input, 0, sizeof input);
    input.head_yaw = 15.0f;
    input.body_yaw = NAN; /* 体は省略 (頭から求める)。 */
    input.body_pitch = NAN;
    input.body_roll = NAN;
    input.shoulder_left = NAN;
    input.shoulder_right = NAN;
    uint32_t blink = 0;
    CHECK(ugoe_blend_shape_index("eyeBlinkLeft", &blink));
    input.blend_shapes[blink] = 1.0f;
    EXPECT(ugoe_blend_shape_index("_neutral", &blink) == UGOE_ERROR_NOT_FOUND);

    /* トラッキング中はモデルの目パチと呼吸を止める (呼吸は AngleX などに重なるため)。 */
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_EYE_BLINK, false));
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_BREATH, false));
    CHECK(ugoe_puppet_set_tracking(puppet, &input));
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));

    size_t targets = 0;
    UgoeResult r = ugoe_puppet_tracking_targets(puppet, NULL, 0, &targets);
    EXPECT(r == UGOE_OK || r == UGOE_ERROR_BUFFER_TOO_SMALL);
    printf("tracking writes %zu parameters\n", targets);

    uint32_t angle_x = 0;
    if (ugoe_puppet_parameter_index(puppet, "AngleX", &angle_x) == UGOE_OK) {
        float value = 0;
        CHECK(ugoe_puppet_get_parameter(puppet, angle_x, &value));
        EXPECT(fabsf(value - 15.0f) < 1e-3f);
    }
    CHECK(ugoe_puppet_set_tracking(puppet, NULL));
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_EYE_BLINK, true));
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_BREATH, true));
}

/* ライティング: ライティングを受ける要素と法線マップを数え、光源をシェーダに渡す形にする。 */
static void light(UgoePuppet *puppet, bool expect_lit) {
    bool uses = false;
    CHECK(ugoe_puppet_uses_lighting(puppet, &uses));
    uint32_t drawables = 0, textures = 0;
    CHECK(ugoe_puppet_drawable_count(puppet, &drawables));
    CHECK(ugoe_puppet_texture_count(puppet, &textures));
    uint32_t lit = 0, normal_maps = 0;
    for (uint32_t d = 0; d < drawables; d++) {
        bool is_lit = false;
        CHECK(ugoe_puppet_drawable_lit(puppet, d, &is_lit));
        lit += is_lit;
    }
    for (uint32_t t = 0; t < textures; t++) {
        UgoeTexture normal_map;
        CHECK(ugoe_puppet_normal_map(puppet, t, &normal_map));
        if (normal_map.format != UGOE_TEXTURE_NONE) {
            EXPECT(normal_map.data != NULL && normal_map.len > 0);
            normal_maps++;
        }
    }
    printf("lighting %s: %u lit drawables, %u normal maps\n", uses ? "on" : "off", lit, normal_maps);
    EXPECT(uses == (lit > 0));
    if (expect_lit) {
        EXPECT(uses && normal_maps > 0);
    }

    /* 左上の手前からの平行光源と、暖色の点光源。 */
    const float ambient[3] = {0.3f, 0.3f, 0.35f};
    UgoeLight lights[2] = {
        {UGOE_LIGHT_DIRECTIONAL, {1.0f, 1.0f, -1.0f}, {1.0f, 1.0f, 1.0f}, 0.0f},
        {UGOE_LIGHT_POINT, {512.0f, 300.0f, 200.0f}, {1.5f, 1.2f, 1.0f}, INFINITY},
    };
    UgoeLightingUniform uniform;
    CHECK(ugoe_lighting_uniform(ambient, lights, 2, &uniform));
    EXPECT(uniform.count == 2);
    EXPECT(uniform.lights[0].vector[3] == 0.0f && uniform.lights[1].vector[3] == 1.0f);
    lights[0].vector[0] = lights[0].vector[1] = lights[0].vector[2] = 0.0f;
    EXPECT(ugoe_lighting_uniform(ambient, lights, 2, &uniform) == UGOE_ERROR_INVALID_ARGUMENT);
}

static UgoeModule *load_module(const char *path) {
    size_t len = 0;
    unsigned char *bytes = read_file(path, &len);
    if (!bytes) {
        exit(1);
    }
    UgoeModule *module = NULL;
    CHECK(ugoe_module_create(bytes, len, &module));
    free(bytes);
    return module;
}

/* 着せ替え: モジュールを順に付け、テクスチャを設定し直す番号を確かめてから外す。 */
static void dress_up(UgoePuppet *puppet, int count, char **paths) {
    uint32_t slots = 0;
    CHECK(ugoe_puppet_slot_count(puppet, &slots));
    printf("%u slots:", slots);
    for (uint32_t i = 0; i < slots; i++) {
        const char *id = NULL;
        CHECK(ugoe_puppet_slot_id(puppet, i, &id));
        printf(" %s", id);
    }
    printf("\n");

    UgoeModule **modules = malloc((count > 0 ? (size_t)count : 1) * sizeof(UgoeModule *));
    for (int i = 0; i < count; i++) {
        modules[i] = load_module(paths[i]);
        const char *name = NULL, *slot = NULL;
        CHECK(ugoe_module_name(modules[i], &name));
        CHECK(ugoe_module_slot(modules[i], &slot));

        uint32_t before = 0, after = 0, textures = 0;
        CHECK(ugoe_puppet_drawable_count(puppet, &before));
        CHECK(ugoe_puppet_attach_module(puppet, modules[i]));
        CHECK(ugoe_puppet_drawable_count(puppet, &after));
        CHECK(ugoe_puppet_texture_count(puppet, &textures));
        const uint32_t *changed = NULL;
        size_t changed_len = 0;
        CHECK(ugoe_puppet_changed_textures(puppet, &changed, &changed_len));
        printf("attached %s to %s: %u -> %u drawables, %zu textures to reload\n", name, slot, before,
               after, changed_len);
        EXPECT(changed_len > 0);
        for (size_t k = 0; k < changed_len; k++) {
            UgoeTexture texture;
            EXPECT(changed[k] < textures);
            CHECK(ugoe_puppet_texture(puppet, changed[k], &texture));
            EXPECT(texture.format != UGOE_TEXTURE_NONE);
        }
        CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));
    }

    uint32_t attached = 0;
    CHECK(ugoe_puppet_attached_module_count(puppet, &attached));
    printf("%u modules attached\n", attached);
    EXPECT(count == 0 || attached > 0);

    /* 付いていないモジュールは外せない。種類は wasm の DressUpError と同じ。 */
    EXPECT(ugoe_puppet_detach_module(puppet, "NoSuchModule") == UGOE_ERROR_DRESS_UP);
    EXPECT(strcmp(ugoe_last_error_kind(), "not_attached") == 0);
    EXPECT(strcmp(ugoe_last_error_field("module"), "NoSuchModule") == 0);

    if (attached > 0) {
        const char *name = NULL;
        CHECK(ugoe_puppet_attached_module_name(puppet, attached - 1, &name));
        char copy[256];
        snprintf(copy, sizeof copy, "%s", name); /* 名前は外すと無効になるので写す。 */
        CHECK(ugoe_puppet_detach_module(puppet, copy));
        printf("detached %s\n", copy);
    }
    /* モジュールを破棄しても、付けたものは残る。本体だけに戻す。 */
    for (int i = 0; i < count; i++) {
        ugoe_module_destroy(modules[i]);
    }
    free(modules);
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));
    CHECK(ugoe_puppet_set_modules(puppet, NULL, 0));
    CHECK(ugoe_puppet_attached_module_count(puppet, &attached));
    EXPECT(attached == 0);
}

/* モーションを順に再生してイベントを数え、物理の設定を読み出して差し替え直す (T0412)。 */
static void animate(UgoePuppet *puppet, uint32_t motions) {
    uint32_t fired = 0;
    for (uint32_t m = 0; m < motions; m++) {
        CHECK(ugoe_puppet_start_motion(puppet, m, UGOE_PRIORITY_FORCE, NULL));
        for (int frame = 0; frame < 120; frame++) {
            CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));
            uint32_t events = 0;
            CHECK(ugoe_puppet_motion_event_count(puppet, &events));
            for (uint32_t e = 0; e < events; e++) {
                UgoeMotionEvent event;
                CHECK(ugoe_puppet_motion_event(puppet, e, &event));
                EXPECT(event.value != NULL);
                fired++;
            }
        }
    }
    printf("%u motion events\n", fired);

    size_t len = 0;
    int result = ugoe_puppet_physics_json(puppet, NULL, 0, &len);
    if (len == 0) {
        CHECK(result);
        printf("no physics\n");
        return;
    }
    EXPECT(result == UGOE_ERROR_BUFFER_TOO_SMALL);
    uint8_t *json = malloc(len);
    CHECK(ugoe_puppet_physics_json(puppet, json, len, &len));
    CHECK(ugoe_puppet_set_physics_json(puppet, json, len));
    EXPECT(ugoe_puppet_set_physics_json(puppet, json, len / 2) == UGOE_ERROR_FORMAT);
    /* モジュールを付けていなければ、本体の設定は今の設定と同じ (T0416)。 */
    size_t base_len = 0;
    EXPECT(ugoe_puppet_base_physics_json(puppet, NULL, 0, &base_len) == UGOE_ERROR_BUFFER_TOO_SMALL);
    EXPECT(base_len == len);
    CHECK(ugoe_puppet_set_base_physics_json(puppet, json, len));
    free(json);
    CHECK(ugoe_puppet_reset_physics(puppet));
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));
    printf("physics settings: %zu bytes of JSON\n", len);
}

/* パーツの不透明度の上書きと評価後の値を読み、各パーツをポーズで表示してみて、効果の設定を差し替える (T0414・T0416)。 */
static void host_actions(UgoePuppet *puppet) {
    uint32_t parts = 0, entries = 0, overridden = 0;
    CHECK(ugoe_puppet_part_count(puppet, &parts));
    for (uint32_t i = 0; i < parts; i++) {
        const char *id = NULL;
        CHECK(ugoe_puppet_part_id(puppet, i, &id));
        uint32_t index = UINT32_MAX;
        CHECK(ugoe_puppet_part_index(puppet, id, &index));
        EXPECT(index == i);
        UgoePartOpacity opacity;
        CHECK(ugoe_puppet_part_opacity(puppet, i, &opacity));
        EXPECT(opacity.weight >= 0.0f && opacity.weight <= 1.0f);
        if (opacity.weight > 0.0f) {
            overridden++;
        }
        /* 評価後の不透明度 (祖先を掛けた値。T0416)。 */
        float evaluated = -1.0f;
        CHECK(ugoe_puppet_part_evaluated_opacity(puppet, i, &evaluated));
        EXPECT(!isnan(evaluated));
        /* どの要素の part でもなければ false (エラーにはならない)。 */
        bool shown = false;
        CHECK(ugoe_puppet_show_pose(puppet, id, &shown));
        if (shown) {
            entries++;
        }
    }
    EXPECT(ugoe_puppet_show_pose(puppet, NULL, NULL) == UGOE_ERROR_NULL_POINTER);
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));
    printf("%u parts, %u overridden, %u pose entries\n", parts, overridden, entries);

    /* モデルの effects と同じ JSON。省略したフィールドは既定値。 */
    static const char effects[] =
        "{\"eye_blink\": {\"parameters\": [\"EyeLOpen\", \"EyeROpen\"], \"interval\": 2, \"seed\": 42}}";
    CHECK(ugoe_puppet_set_effects_json(puppet, (const uint8_t *)effects, strlen(effects)));
    EXPECT(ugoe_puppet_set_effects_json(puppet, (const uint8_t *)"{", 1) == UGOE_ERROR_FORMAT);
    for (int frame = 0; frame < 60; frame++) {
        CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f));
    }
}

int main(int argc, char **argv) {
    bool expect_lit = false;
    int arg = 1;
    if (arg < argc && strcmp(argv[arg], "--lit") == 0) {
        expect_lit = true;
        arg++;
    }
    if (arg >= argc) {
        fprintf(stderr, "usage: %s [--lit] <model.ugoe> [module.ugoemod ...]\n", argv[0]);
        return 2;
    }
    const char *model_path = argv[arg++];

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
    unsigned char *bytes = read_file(model_path, &len);
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

    print_warnings(puppet);
    hit_test(puppet, &canvas);

    /* ID でまとめて設定する。知らない ID は飛ばす。 */
    const char *ids[2] = {"AngleX", "NoSuchParameter"};
    const float values[2] = {10.0f, 1.0f};
    size_t applied = 0;
    CHECK(ugoe_puppet_set_parameters_by_id(puppet, ids, values, 2, &applied));
    printf("set %zu of 2 parameters by id\n", applied);
    EXPECT(applied <= 1);

    track(puppet);

    uint32_t expressions = 0;
    CHECK(ugoe_puppet_expression_count(puppet, &expressions));
    if (expressions > 0) {
        CHECK(ugoe_puppet_set_expression(puppet, 0));
        CHECK(ugoe_puppet_update(puppet, 0.5f));
        CHECK(ugoe_puppet_stop_expressions(puppet));
        CHECK(ugoe_puppet_update(puppet, 0.5f));
    }

    animate(puppet, motions);
    host_actions(puppet);
    light(puppet, expect_lit);
    dress_up(puppet, argc - arg, argv + arg);

    ugoe_puppet_destroy(puppet);
    printf("ok\n");
    return 0;
}
