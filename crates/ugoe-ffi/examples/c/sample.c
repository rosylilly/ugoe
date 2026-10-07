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

/*
 * 失敗の中身を受け取る UgoeError。失敗しうる関数は最後の引数にこれを取る (NULL なら書かない)。
 * この見本は 1 つのスレッドで動くので 1 つを使い回す。複数のスレッドから呼ぶホストは、呼び出しごとかスレッドごとに 1 つ持つ。
 */
static UgoeError *err;

/* 呼び出しが失敗したら、エラーメッセージを出して終わる。 */
#define CHECK(call)                                                                \
    do {                                                                           \
        UgoeResult check_result_ = (call);                                         \
        if (check_result_ != UGOE_OK) {                                            \
            const char *message_ = ugoe_error_message(err);                        \
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
    CHECK(ugoe_puppet_load_warning_count(puppet, &count, err));
    for (uint32_t i = 0; i < count; i++) {
        UgoeWarning warning;
        CHECK(ugoe_puppet_load_warning(puppet, i, &warning, err));
        printf("warning [%s]: %s\n", warning.code, warning.message);
    }
}

/* 当たり判定の一覧と、キャンバスの中ほどで当たるもの。 */
static void hit_test(UgoePuppet *puppet, const UgoeCanvas *canvas) {
    uint32_t count = 0;
    CHECK(ugoe_puppet_hit_area_count(puppet, &count, err));
    float x = canvas->width / 2, y = canvas->height / 2;
    size_t hits = 0;
    /* まず数を聞いてから、足りるバッファで受け取る。 */
    UgoeResult r = ugoe_puppet_hit_areas_at(puppet, x, y, NULL, 0, &hits, err);
    EXPECT(r == UGOE_OK || r == UGOE_ERROR_BUFFER_TOO_SMALL);
    uint32_t *list = malloc((hits > 0 ? hits : 1) * sizeof(uint32_t));
    CHECK(ugoe_puppet_hit_areas_at(puppet, x, y, list, hits, &hits, err));
    printf("%u hit areas, %zu at (%.0f, %.0f)", count, hits, x, y);
    for (size_t i = 0; i < hits; i++) {
        const char *id = NULL;
        CHECK(ugoe_puppet_hit_area_id(puppet, list[i], &id, err));
        printf(" %s", id);
    }
    printf("\n");
    free(list);

    /* 標準の名前 (アバタープロファイル) での判定。対応が無ければ false。 */
    bool head = true;
    CHECK(ugoe_puppet_hit_test_standard(puppet, "Head", x, y, &head, err));
    uint32_t index = 0;
    if (ugoe_puppet_standard_hit_area(puppet, "Head", &index, err) == UGOE_OK) {
        const char *id = NULL;
        CHECK(ugoe_puppet_hit_area_id(puppet, index, &id, err));
        bool hit = false;
        CHECK(ugoe_puppet_hit_test(puppet, id, x, y, &hit, err));
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
    CHECK(ugoe_blend_shape_index("eyeBlinkLeft", &blink, err));
    input.blend_shapes[blink] = 1.0f;
    EXPECT(ugoe_blend_shape_index("_neutral", &blink, err) == UGOE_ERROR_NOT_FOUND);

    /* トラッキング中はモデルの目パチと呼吸を止める (呼吸は AngleX などに重なるため)。 */
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_EYE_BLINK, false, err));
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_BREATH, false, err));
    CHECK(ugoe_puppet_set_tracking(puppet, &input, err));
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));

    size_t targets = 0;
    UgoeResult r = ugoe_puppet_tracking_targets(puppet, NULL, 0, &targets, err);
    EXPECT(r == UGOE_OK || r == UGOE_ERROR_BUFFER_TOO_SMALL);
    printf("tracking writes %zu parameters\n", targets);

    uint32_t angle_x = 0;
    if (ugoe_puppet_parameter_index(puppet, "AngleX", &angle_x, err) == UGOE_OK) {
        float value = 0;
        CHECK(ugoe_puppet_get_parameter(puppet, angle_x, &value, err));
        EXPECT(fabsf(value - 15.0f) < 1e-3f);
    }
    CHECK(ugoe_puppet_set_tracking(puppet, NULL, err));
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_EYE_BLINK, true, err));
    CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_BREATH, true, err));
}

/* ライティング: ライティングを受ける要素と法線マップを数え、光源をシェーダに渡す形にする。 */
static void light(UgoePuppet *puppet, bool expect_lit) {
    bool uses = false;
    CHECK(ugoe_puppet_uses_lighting(puppet, &uses, err));
    uint32_t drawables = 0, textures = 0;
    CHECK(ugoe_puppet_drawable_count(puppet, &drawables, err));
    CHECK(ugoe_puppet_texture_count(puppet, &textures, err));
    uint32_t lit = 0, normal_maps = 0;
    for (uint32_t d = 0; d < drawables; d++) {
        bool is_lit = false;
        CHECK(ugoe_puppet_drawable_lit(puppet, d, &is_lit, err));
        lit += is_lit;
    }
    for (uint32_t t = 0; t < textures; t++) {
        UgoeTexture normal_map;
        CHECK(ugoe_puppet_normal_map(puppet, t, &normal_map, err));
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
    CHECK(ugoe_lighting_uniform(ambient, lights, 2, &uniform, err));
    EXPECT(uniform.count == 2);
    EXPECT(uniform.lights[0].vector[3] == 0.0f && uniform.lights[1].vector[3] == 1.0f);
    lights[0].vector[0] = lights[0].vector[1] = lights[0].vector[2] = 0.0f;
    EXPECT(ugoe_lighting_uniform(ambient, lights, 2, &uniform, err) == UGOE_ERROR_INVALID_ARGUMENT);
}

static UgoeModule *load_module(const char *path) {
    size_t len = 0;
    unsigned char *bytes = read_file(path, &len);
    if (!bytes) {
        exit(1);
    }
    UgoeModule *module = NULL;
    CHECK(ugoe_module_create(bytes, len, &module, err));
    free(bytes);
    return module;
}

/* 着せ替え: モジュールを順に付け、テクスチャを設定し直す番号を確かめてから外す。 */
static void dress_up(UgoePuppet *puppet, int count, char **paths) {
    uint32_t slots = 0;
    CHECK(ugoe_puppet_slot_count(puppet, &slots, err));
    printf("%u slots:", slots);
    for (uint32_t i = 0; i < slots; i++) {
        const char *id = NULL;
        CHECK(ugoe_puppet_slot_id(puppet, i, &id, err));
        printf(" %s", id);
    }
    printf("\n");

    UgoeModule **modules = malloc((count > 0 ? (size_t)count : 1) * sizeof(UgoeModule *));
    for (int i = 0; i < count; i++) {
        modules[i] = load_module(paths[i]);
        const char *name = NULL, *slot = NULL;
        CHECK(ugoe_module_name(modules[i], &name, err));
        CHECK(ugoe_module_slot(modules[i], &slot, err));

        uint32_t before = 0, after = 0, textures = 0;
        CHECK(ugoe_puppet_drawable_count(puppet, &before, err));
        CHECK(ugoe_puppet_attach_module(puppet, modules[i], err));
        CHECK(ugoe_puppet_drawable_count(puppet, &after, err));
        CHECK(ugoe_puppet_texture_count(puppet, &textures, err));
        const uint32_t *changed = NULL;
        size_t changed_len = 0;
        CHECK(ugoe_puppet_changed_textures(puppet, &changed, &changed_len, err));
        printf("attached %s to %s: %u -> %u drawables, %zu textures to reload\n", name, slot, before,
               after, changed_len);
        EXPECT(changed_len > 0);
        for (size_t k = 0; k < changed_len; k++) {
            UgoeTexture texture;
            EXPECT(changed[k] < textures);
            CHECK(ugoe_puppet_texture(puppet, changed[k], &texture, err));
            EXPECT(texture.format != UGOE_TEXTURE_NONE);
        }
        CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));
    }

    uint32_t attached = 0;
    CHECK(ugoe_puppet_attached_module_count(puppet, &attached, err));
    printf("%u modules attached\n", attached);
    EXPECT(count == 0 || attached > 0);

    /* 付いていないモジュールは外せない。種類は wasm の DressUpError と同じ。 */
    EXPECT(ugoe_puppet_detach_module(puppet, "NoSuchModule", err) == UGOE_ERROR_DRESS_UP);
    EXPECT(strcmp(ugoe_error_kind(err), "not_attached") == 0);
    EXPECT(strcmp(ugoe_error_field(err, "module"), "NoSuchModule") == 0);
    /* キーを知らなくても、補足の値は番号で列挙できる。 */
    EXPECT(ugoe_error_field_count(err) == 1);
    EXPECT(strcmp(ugoe_error_field_key_at(err, 0), "module") == 0);
    EXPECT(strcmp(ugoe_error_field_value_at(err, 0), "NoSuchModule") == 0);
    EXPECT(ugoe_error_field_key_at(err, 1) == NULL);

    if (attached > 0) {
        const char *name = NULL;
        CHECK(ugoe_puppet_attached_module_name(puppet, attached - 1, &name, err));
        char copy[256];
        snprintf(copy, sizeof copy, "%s", name); /* 名前は外すと無効になるので写す。 */
        CHECK(ugoe_puppet_detach_module(puppet, copy, err));
        printf("detached %s\n", copy);
    }
    /* モジュールを破棄しても、付けたものは残る。本体だけに戻す。 */
    for (int i = 0; i < count; i++) {
        ugoe_module_destroy(modules[i]);
    }
    free(modules);
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));
    CHECK(ugoe_puppet_set_modules(puppet, NULL, 0, err));
    CHECK(ugoe_puppet_attached_module_count(puppet, &attached, err));
    EXPECT(attached == 0);
}

/* モーションを順に再生してイベントを数え、物理の設定を読み出して差し替え直す (T0412)。 */
static void animate(UgoePuppet *puppet, uint32_t motions) {
    uint32_t fired = 0;
    for (uint32_t m = 0; m < motions; m++) {
        CHECK(ugoe_puppet_start_motion(puppet, m, UGOE_PRIORITY_FORCE, NULL, err));
        for (int frame = 0; frame < 120; frame++) {
            CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));
            uint32_t events = 0;
            CHECK(ugoe_puppet_motion_event_count(puppet, &events, err));
            for (uint32_t e = 0; e < events; e++) {
                UgoeMotionEvent event;
                CHECK(ugoe_puppet_motion_event(puppet, e, &event, err));
                EXPECT(event.value != NULL);
                fired++;
            }
        }
    }
    printf("%u motion events\n", fired);

    size_t len = 0;
    int result = ugoe_puppet_physics_json(puppet, NULL, 0, &len, err);
    if (len == 0) {
        CHECK(result);
        printf("no physics\n");
        return;
    }
    EXPECT(result == UGOE_ERROR_BUFFER_TOO_SMALL);
    uint8_t *json = malloc(len);
    CHECK(ugoe_puppet_physics_json(puppet, json, len, &len, err));
    CHECK(ugoe_puppet_set_physics_json(puppet, json, len, err));
    EXPECT(ugoe_puppet_set_physics_json(puppet, json, len / 2, err) == UGOE_ERROR_FORMAT);
    /* モジュールを付けていなければ、本体の設定は今の設定と同じ (T0416)。 */
    size_t base_len = 0;
    EXPECT(ugoe_puppet_base_physics_json(puppet, NULL, 0, &base_len, err) == UGOE_ERROR_BUFFER_TOO_SMALL);
    EXPECT(base_len == len);
    CHECK(ugoe_puppet_set_base_physics_json(puppet, json, len, err));
    free(json);
    CHECK(ugoe_puppet_reset_physics(puppet, err));
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));
    printf("physics settings: %zu bytes of JSON\n", len);
}

/* パーツの不透明度の上書きと評価後の値を読み、各パーツをポーズで表示してみて、効果の設定を差し替える (T0414・T0416)。 */
static void host_actions(UgoePuppet *puppet) {
    uint32_t parts = 0, entries = 0, overridden = 0;
    CHECK(ugoe_puppet_part_count(puppet, &parts, err));
    for (uint32_t i = 0; i < parts; i++) {
        const char *id = NULL;
        CHECK(ugoe_puppet_part_id(puppet, i, &id, err));
        uint32_t index = UINT32_MAX;
        CHECK(ugoe_puppet_part_index(puppet, id, &index, err));
        EXPECT(index == i);
        UgoePartOpacity opacity;
        CHECK(ugoe_puppet_part_opacity(puppet, i, &opacity, err));
        EXPECT(opacity.weight >= 0.0f && opacity.weight <= 1.0f);
        if (opacity.weight > 0.0f) {
            overridden++;
        }
        /* 評価後の不透明度 (祖先を掛けた値。T0416)。 */
        float evaluated = -1.0f;
        CHECK(ugoe_puppet_part_evaluated_opacity(puppet, i, &evaluated, err));
        EXPECT(!isnan(evaluated));
        /* どの要素の part でもなければ false (エラーにはならない)。 */
        bool shown = false;
        CHECK(ugoe_puppet_show_pose(puppet, id, &shown, err));
        if (shown) {
            entries++;
        }
    }
    EXPECT(ugoe_puppet_show_pose(puppet, NULL, NULL, err) == UGOE_ERROR_NULL_POINTER);
    CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));
    printf("%u parts, %u overridden, %u pose entries\n", parts, overridden, entries);

    /* 表示中のポーズの要素 (グループごとのパーツの番号。T0478)。上で最後に選んだ要素が表示中になる。
       ポーズの定義 (グループごとの要素のパーツの番号。T0483) で、表示中の要素がグループの要素であることも確かめる。 */
    uint32_t groups = 0;
    CHECK(ugoe_puppet_pose_group_count(puppet, &groups, err));
    for (uint32_t g = 0; g < groups; g++) {
        uint32_t count = 0;
        CHECK(ugoe_puppet_pose_entry_count(puppet, g, &count, err));
        uint32_t part = UINT32_MAX;
        UgoeResult r = ugoe_puppet_pose_visible_part(puppet, g, &part, err);
        if (r == UGOE_ERROR_NOT_FOUND) {
            EXPECT(count == 0);
            continue; /* 要素の無いグループ */
        }
        CHECK(r);
        bool listed = false;
        for (uint32_t e = 0; e < count; e++) {
            uint32_t entry = UINT32_MAX;
            CHECK(ugoe_puppet_pose_entry_part(puppet, g, e, &entry, err));
            listed = listed || entry == part;
        }
        EXPECT(listed);
        uint32_t none = 0;
        EXPECT(ugoe_puppet_pose_entry_part(puppet, g, count, &none, err) == UGOE_ERROR_OUT_OF_RANGE);
        const char *id = NULL;
        CHECK(ugoe_puppet_part_id(puppet, part, &id, err));
        bool shown = false;
        CHECK(ugoe_puppet_show_pose(puppet, id, &shown, err));
        EXPECT(shown);
        printf("pose group %u shows %s\n", g, id);
    }
    uint32_t unused = 0;
    EXPECT(ugoe_puppet_pose_visible_part(puppet, groups, &unused, err) == UGOE_ERROR_OUT_OF_RANGE);
    EXPECT(ugoe_puppet_pose_entry_count(puppet, groups, &unused, err) == UGOE_ERROR_OUT_OF_RANGE);

    /* モデルの effects と同じ JSON。省略したフィールドは既定値。 */
    static const char effects[] =
        "{\"eye_blink\": {\"parameters\": [\"EyeLOpen\", \"EyeROpen\"], \"interval\": 2, \"seed\": 42}}";
    CHECK(ugoe_puppet_set_effects_json(puppet, (const uint8_t *)effects, strlen(effects), err));
    EXPECT(ugoe_puppet_set_effects_json(puppet, (const uint8_t *)"{", 1, err) == UGOE_ERROR_FORMAT);
    for (int frame = 0; frame < 60; frame++) {
        CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));
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
    err = ugoe_error_create();

    /* 壊れた入力はエラーコードとメッセージで返る。 */
    UgoePuppet *puppet = NULL;
    const unsigned char garbage[] = "not a bundle";
    EXPECT(ugoe_puppet_create_from_bundle(garbage, sizeof garbage, &puppet, err) == UGOE_ERROR_FORMAT);
    EXPECT(puppet == NULL);
    EXPECT(ugoe_error_message(err) != NULL);
    printf("rejected garbage: %s\n", ugoe_error_message(err));

    size_t len = 0;
    unsigned char *bytes = read_file(model_path, &len);
    if (!bytes) {
        return 1;
    }
    CHECK(ugoe_puppet_create_from_bundle(bytes, len, &puppet, err));
    free(bytes); /* 読み込みの後はバイト列を解放してよい。 */

    UgoeCanvas canvas;
    CHECK(ugoe_puppet_canvas(puppet, &canvas, err));
    uint32_t parameters = 0, drawables = 0, textures = 0, motions = 0;
    CHECK(ugoe_puppet_parameter_count(puppet, &parameters, err));
    CHECK(ugoe_puppet_drawable_count(puppet, &drawables, err));
    CHECK(ugoe_puppet_texture_count(puppet, &textures, err));
    CHECK(ugoe_puppet_motion_count(puppet, &motions, err));
    printf("canvas %.0fx%.0f, %u parameters, %u drawables, %u textures, %u motions\n",
           canvas.width, canvas.height, parameters, drawables, textures, motions);
    EXPECT(drawables > 0);

    /* テクスチャのデコード (T0466): エンジンが読めない形式 (KTX2 の BCn など) は、参照と同じデコーダで RGBA8 にできる。 */
    size_t decoded_pixels = 0;
    for (uint32_t t = 0; t < textures; t++) {
        UgoeTexture texture;
        CHECK(ugoe_puppet_texture(puppet, t, &texture, err));
        if (texture.format == UGOE_TEXTURE_NONE) {
            continue;
        }
        UgoeImage *image = NULL;
        CHECK(ugoe_image_decode(texture.format, texture.data, texture.len, NULL, true, &image, err));
        uint32_t width = 0, height = 0;
        const uint8_t *pixels = NULL;
        size_t pixels_len = 0;
        CHECK(ugoe_image_size(image, &width, &height, err));
        CHECK(ugoe_image_pixels(image, &pixels, &pixels_len, err)); /* 乗算済みの RGBA8、上の行が先 */
        EXPECT(pixels_len == (size_t)width * height * 4);
        decoded_pixels += (size_t)width * height;
        ugoe_image_destroy(image);
    }
    printf("decoded %u textures: %zu pixels\n", textures, decoded_pixels);
    UgoeImageLimits image_limits = ugoe_image_limits_default();
    image_limits.max_side = 1;
    UgoeTexture first;
    UgoeImage *too_large = NULL;
    CHECK(ugoe_puppet_texture(puppet, 0, &first, err));
    EXPECT(ugoe_image_decode(first.format, first.data, first.len, &image_limits, true, &too_large, err) ==
           UGOE_ERROR_LIMIT_EXCEEDED);
    EXPECT(too_large == NULL && strcmp(ugoe_error_kind(err), "image_limit_exceeded") == 0);

    /* すべてのパラメータを最大値にして 1 秒ぶん進める。 */
    for (uint32_t i = 0; i < parameters; i++) {
        UgoeParameterInfo info;
        CHECK(ugoe_puppet_parameter_info(puppet, i, &info, err));
        CHECK(ugoe_puppet_set_parameter(puppet, i, info.max, err));
    }
    for (int frame = 0; frame < 60; frame++) {
        CHECK(ugoe_puppet_update(puppet, 1.0f / 60.0f, err));
    }

    /* このフレームの状態をまとめて借り、奥から順に読む。コピーした状態・頂点が借りたものと一致することも確かめる。 */
    UgoeFrame frame;
    CHECK(ugoe_puppet_frame(puppet, &frame, err));
    EXPECT(frame.drawable_count == drawables);
    size_t visible = 0, vertices = 0;
    for (uint32_t k = 0; k < frame.drawable_count; k++) {
        uint32_t d = frame.render_order[k];
        UgoeDrawableInfo info;
        CHECK(ugoe_puppet_drawable_info(puppet, d, &info, err));
        EXPECT(info.texture < textures);
        const UgoeDrawableState *state = &frame.states[d];
        const UgoePositionRange *range = &frame.position_ranges[d];
        EXPECT(range->count == 2 * (size_t)info.vertex_count);
        EXPECT(range->first + range->count <= frame.positions_len);
        for (size_t v = 0; v < range->count; v++) {
            EXPECT(isfinite(frame.positions[range->first + v]));
        }
        if (state->visible) {
            visible++;
        }
        vertices += info.vertex_count;
    }
    printf("%zu visible drawables, %zu vertices\n", visible, vertices);
    EXPECT(visible > 0);

    /* コピーの形: まず要る数を聞き、ホストのバッファに写す。 */
    UgoeFrameBuffers buffers = {0};
    UgoeFrameCopy copy;
    EXPECT(ugoe_puppet_copy_frame(puppet, &buffers, false, &copy, err) == UGOE_ERROR_BUFFER_TOO_SMALL);
    EXPECT(copy.drawable_count == drawables && copy.positions_len == frame.positions_len);
    buffers.drawable_capacity = copy.drawable_count;
    buffers.states = malloc((copy.drawable_count + 1) * sizeof(UgoeDrawableState));
    buffers.render_order = malloc((copy.drawable_count + 1) * sizeof(uint32_t));
    buffers.position_ranges = malloc((copy.drawable_count + 1) * sizeof(UgoePositionRange));
    buffers.positions_capacity = copy.positions_len;
    buffers.positions = malloc((copy.positions_len + 1) * sizeof(float));
    CHECK(ugoe_puppet_copy_frame(puppet, &buffers, false, &copy, err));
    EXPECT(copy.render_order_changed == frame.render_order_changed);
    EXPECT(memcmp(buffers.render_order, frame.render_order, drawables * sizeof(uint32_t)) == 0);
    EXPECT(memcmp(buffers.position_ranges, frame.position_ranges, drawables * sizeof(UgoePositionRange)) == 0);
    EXPECT(frame.positions_len == 0 || memcmp(buffers.positions, frame.positions, frame.positions_len * sizeof(float)) == 0);
    for (uint32_t d = 0; d < drawables; d++) {
        EXPECT(buffers.states[d].opacity == frame.states[d].opacity);
        EXPECT(buffers.states[d].changes == frame.states[d].changes);
    }
    free(buffers.states);
    free(buffers.render_order);
    free(buffers.position_ranges);
    free(buffers.positions);

    /* 範囲外の番号はエラーになる。 */
    UgoeDrawableInfo out_of_range;
    EXPECT(ugoe_puppet_drawable_info(puppet, drawables, &out_of_range, err) == UGOE_ERROR_OUT_OF_RANGE);

    print_warnings(puppet);
    hit_test(puppet, &canvas);

    /* ID でまとめて設定する。知らない ID は飛ばす。 */
    const char *ids[2] = {"AngleX", "NoSuchParameter"};
    const float values[2] = {10.0f, 1.0f};
    size_t applied = 0;
    CHECK(ugoe_puppet_set_parameters_by_id(puppet, ids, values, 2, &applied, err));
    printf("set %zu of 2 parameters by id\n", applied);
    EXPECT(applied <= 1);

    track(puppet);

    uint32_t expressions = 0;
    CHECK(ugoe_puppet_expression_count(puppet, &expressions, err));
    if (expressions > 0) {
        CHECK(ugoe_puppet_set_expression(puppet, 0, err));
        CHECK(ugoe_puppet_update(puppet, 0.5f, err));
        CHECK(ugoe_puppet_stop_expressions(puppet, err));
        CHECK(ugoe_puppet_update(puppet, 0.5f, err));
    }

    animate(puppet, motions);
    host_actions(puppet);
    light(puppet, expect_lit);
    dress_up(puppet, argc - arg, argv + arg);

    ugoe_puppet_destroy(puppet);
    ugoe_error_destroy(err);
    printf("ok\n");
    return 0;
}
