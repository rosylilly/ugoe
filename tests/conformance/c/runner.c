/*
 * 適合テストの C ABI (ugoe-ffi) の実行器 (tests/conformance/README.md)。
 *
 *   runner <plan.txt> <出力ディレクトリ>
 *
 * 計画 (ugoe-conformance reference が書く plan.txt) のケースごとに UgoePuppet を作り、モジュールを付け、
 * パラメータを設定して評価し、結果を <出力ディレクトリ>/<ケース>.json に書く (形は src/frame.rs)。
 * C ABI は描画しない (ホストのエンジンが描く) ので、画像は作らない。
 *
 * - update の行が無いケースは、すべての段を止めて ugoe_puppet_update(0) を 1 回呼ぶ (時間を進めない評価)。
 * - update の行があるケースは、目パチと呼吸を effects のとおりにし、ほかの段を動かして、その dt で順に呼ぶ。
 * - follow の行 (受け手として進めるケース。T0133・T0134・T0135) は、update の代わりに
 *   ugoe_puppet_follow(値, リップシンクの音量, 強さ, パーツの不透明度, dt) を順に呼ぶ。
 * - start の行 (時間を進める途中で始めるモーション。T0462) があれば、その番号の update の直前に ugoe_puppet_start_motion で始め、
 *   各 update の後に発火したイベントと出どころ (ugoe_puppet_motion_event・ugoe_puppet_motion_event_source) を結果の motion_events に書く。
 * - conduct の行 (実行器の中で指揮役を進めるケース。T0454) があれば、同じモデルで指揮役の UgoePuppet をもう 1 つ作り、
 *   lead の行ごとに指揮役を ugoe_puppet_update で進め、指揮役の口 (ugoe_puppet_values_before_physics・
 *   ugoe_puppet_lip_sync_applied・ugoe_puppet_part_opacities_applied) で読んだ値で受け手を ugoe_puppet_follow で進める。
 */
#include <errno.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "ugoe.h"

#define MAX_LINE 65536
#define MAX_ITEMS 256
#define MAX_STEPS 4096

/* 呼び出しが失敗したら、エラーメッセージを出して false を返す。 */
#define CHECK(call)                                                                   \
    do {                                                                              \
        UgoeResult check_result_ = (call);                                            \
        if (check_result_ != UGOE_OK) {                                               \
            const char *message_ = ugoe_last_error();                                 \
            fprintf(stderr, "%s: %s failed (%d): %s\n", c->name, #call,               \
                    (int)check_result_, message_ ? message_ : "(no message)");        \
            ok = false;                                                               \
            goto done;                                                                \
        }                                                                             \
    } while (0)

typedef struct {
    char name[256];
    bool bundle;
    char model[MAX_LINE];
    char *modules[MAX_ITEMS];
    size_t module_count;
    char *param_ids[MAX_ITEMS];
    float param_values[MAX_ITEMS];
    size_t param_count;
    bool effects;
    float steps[MAX_STEPS];
    size_t step_count;
    /* follow の行の値 (steps と同じ番号)。follow_count が 0 なら update で進める。 */
    float *follow[MAX_STEPS];
    size_t follow_len[MAX_STEPS];
    float follow_lip_sync[MAX_STEPS][2];
    UgoePartOpacityInput *follow_parts[MAX_STEPS];
    size_t follow_part_count[MAX_STEPS];
    size_t follow_count;
    /* conduct の行があったか、そのモーションの名前 (空なら再生しない)・効果の設定の JSON (NULL なら目パチと呼吸を止める)、
       lead の行の音量 (steps と同じ番号。負なら設定しない)。 */
    bool conduct;
    char conduct_motion[256];
    char *conduct_effects;
    float lead_lip_sync[MAX_STEPS];
    /* start の行 (step の順): update の番号・優先度・始まるはずか・モーションの名前。 */
    size_t start_steps[MAX_ITEMS];
    uint32_t start_priorities[MAX_ITEMS];
    bool start_accepted[MAX_ITEMS];
    char *start_names[MAX_ITEMS];
    size_t start_count;
} Case;

/* 発火したモーションのイベント 1 つ (文字列は複製したもの)。 */
typedef struct {
    size_t step;
    char *value;
    float time;
    char *motion;
    uint64_t number;
} Event;

typedef struct {
    Event *items;
    size_t len;
    size_t cap;
} Events;

static void clear_events(Events *events) {
    for (size_t i = 0; i < events->len; i++) {
        free(events->items[i].value);
        free(events->items[i].motion);
    }
    free(events->items);
    memset(events, 0, sizeof *events);
}

static void clear_case(Case *c) {
    for (size_t i = 0; i < c->module_count; i++) free(c->modules[i]);
    for (size_t i = 0; i < c->param_count; i++) free(c->param_ids[i]);
    for (size_t i = 0; i < c->follow_count; i++) {
        free(c->follow[i]);
        free(c->follow_parts[i]);
    }
    free(c->conduct_effects);
    for (size_t i = 0; i < c->start_count; i++) free(c->start_names[i]);
    memset(c, 0, sizeof *c);
}

static char *duplicate(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

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

static float parse_float(const char *s) {
    char *end = NULL;
    errno = 0;
    float v = strtof(s, &end);
    if (errno != 0 || end == s || (*end != '\0' && *end != '\n')) {
        fprintf(stderr, "invalid number `%s`\n", s);
        exit(2);
    }
    return v;
}

/* JSON の文字列として書く (ID は `"` と `\` と制御文字だけを逃がせばよい)。 */
static void write_string(FILE *f, const char *s) {
    fputc('"', f);
    for (; *s; s++) {
        unsigned char ch = (unsigned char)*s;
        if (ch == '"' || ch == '\\') {
            fputc('\\', f);
            fputc(ch, f);
        } else if (ch < 0x20) {
            fprintf(f, "\\u%04x", ch);
        } else {
            fputc(ch, f);
        }
    }
    fputc('"', f);
}

/* float を同じ値に戻る桁数で書く。 */
static void write_float(FILE *f, float v) {
    fprintf(f, "%.9g", (double)v);
}

static void write_rgb(FILE *f, const float rgb[3]) {
    fputs("{\"r\":", f);
    write_float(f, rgb[0]);
    fputs(",\"g\":", f);
    write_float(f, rgb[1]);
    fputs(",\"b\":", f);
    write_float(f, rgb[2]);
    fputc('}', f);
}

/* モデルから UgoePuppet を作り、モジュール (作ってあれば使い回す) を付け、パラメータを設定する。 */
static bool create_puppet(const Case *c, const unsigned char *bytes, size_t len, UgoeModule **modules,
                          UgoePuppet **out) {
    bool ok = true;
    UgoePuppet *puppet = NULL;
    if (c->bundle) {
        CHECK(ugoe_puppet_create_from_bundle(bytes, len, &puppet));
    } else {
        CHECK(ugoe_puppet_create_from_json(bytes, len, &puppet));
    }
    for (size_t i = 0; i < c->module_count; i++) {
        if (!modules[i]) {
            size_t module_len = 0;
            unsigned char *module_bytes = read_file(c->modules[i], &module_len);
            if (!module_bytes) {
                ok = false;
                goto done;
            }
            UgoeResult r = ugoe_module_create(module_bytes, module_len, &modules[i]);
            free(module_bytes);
            CHECK(r);
        }
        CHECK(ugoe_puppet_attach_module(puppet, modules[i]));
    }
    for (size_t i = 0; i < c->param_count; i++) {
        uint32_t index = 0;
        CHECK(ugoe_puppet_parameter_index(puppet, c->param_ids[i], &index));
        CHECK(ugoe_puppet_set_parameter(puppet, index, c->param_values[i]));
    }
done:
    if (!ok) {
        ugoe_puppet_destroy(puppet);
        puppet = NULL;
    }
    *out = puppet;
    return ok;
}

/* バンドルのモーションの番号 (無ければエラーを出して false)。 */
static bool motion_index(const Case *c, UgoePuppet *puppet, const char *name, uint32_t *out) {
    bool ok = true;
    uint32_t count = 0;
    CHECK(ugoe_puppet_motion_count(puppet, &count));
    for (uint32_t i = 0; i < count; i++) {
        const char *motion = NULL;
        CHECK(ugoe_puppet_motion_name(puppet, i, &motion));
        if (strcmp(motion, name) == 0) {
            *out = i;
            return true;
        }
    }
    fprintf(stderr, "%s: no motion `%s`\n", c->name, name);
    ok = false;
done:
    return ok;
}

/* 直前の update で発火したイベントと出どころを events に足す (T0462)。 */
static bool record_events(const Case *c, UgoePuppet *puppet, size_t step, Events *events) {
    bool ok = true;
    uint32_t count = 0;
    CHECK(ugoe_puppet_motion_event_count(puppet, &count));
    for (uint32_t i = 0; i < count; i++) {
        UgoeMotionEvent e;
        UgoeMotionEventSource source;
        CHECK(ugoe_puppet_motion_event(puppet, i, &e));
        CHECK(ugoe_puppet_motion_event_source(puppet, i, &source));
        if (events->len == events->cap) {
            size_t cap = events->cap ? events->cap * 2 : 16;
            Event *items = realloc(events->items, cap * sizeof *items);
            if (!items) {
                ok = false;
                goto done;
            }
            events->items = items;
            events->cap = cap;
        }
        Event *event = &events->items[events->len++];
        event->step = step;
        event->value = duplicate(e.value);
        event->time = e.time;
        event->motion = duplicate(source.motion);
        event->number = source.number;
    }
done:
    return ok;
}

/* 指揮役を作る (T0454): 効果の設定を書き、物理の段を止め、目パチと呼吸は効果の設定があるときだけ動かし、モーションを再生する。 */
static bool create_conductor(const Case *c, const unsigned char *bytes, size_t len, UgoeModule **modules,
                             UgoePuppet **out) {
    bool ok = true;
    UgoePuppet *conductor = NULL;
    if (!create_puppet(c, bytes, len, modules, &conductor)) return false;
    if (c->conduct_effects) {
        CHECK(ugoe_puppet_set_effects_json(conductor, (const uint8_t *)c->conduct_effects,
                                           strlen(c->conduct_effects)));
    }
    CHECK(ugoe_puppet_set_stage_enabled(conductor, UGOE_STAGE_PHYSICS, false));
    CHECK(ugoe_puppet_set_stage_enabled(conductor, UGOE_STAGE_EYE_BLINK, c->conduct_effects != NULL));
    CHECK(ugoe_puppet_set_stage_enabled(conductor, UGOE_STAGE_BREATH, c->conduct_effects != NULL));
    if (c->conduct_motion[0] != '\0') {
        uint32_t index = 0;
        if (!motion_index(c, conductor, c->conduct_motion, &index)) {
            ok = false;
            goto done;
        }
        bool started = false;
        CHECK(ugoe_puppet_start_motion(conductor, index, UGOE_PRIORITY_NORMAL, &started));
        if (!started) {
            fprintf(stderr, "%s: motion `%s` did not start\n", c->name, c->conduct_motion);
            ok = false;
            goto done;
        }
    }
done:
    if (!ok) {
        ugoe_puppet_destroy(conductor);
        conductor = NULL;
    }
    *out = conductor;
    return ok;
}

/* 指揮役を dt 秒進め、指揮役の口で読んだ値で受け手を進める (T0454)。 */
static bool lead(const Case *c, UgoePuppet *conductor, UgoePuppet *puppet, float level, float dt) {
    bool ok = true;
    float *values = NULL;
    UgoePartOpacityInput *parts = NULL;
    if (level >= 0.0f) CHECK(ugoe_puppet_set_lip_sync_level(conductor, level));
    CHECK(ugoe_puppet_update(conductor, dt));
    size_t count = 0;
    UgoeResult r = ugoe_puppet_values_before_physics(conductor, NULL, 0, &count);
    if (r != UGOE_ERROR_BUFFER_TOO_SMALL) CHECK(r);
    values = malloc((count > 0 ? count : 1) * sizeof *values);
    if (!values) {
        ok = false;
        goto done;
    }
    CHECK(ugoe_puppet_values_before_physics(conductor, values, count, &count));
    float strength = 0.0f;
    CHECK(ugoe_puppet_lip_sync_applied(conductor, &level, &strength));
    size_t part_count = 0;
    r = ugoe_puppet_part_opacities_applied(conductor, NULL, 0, &part_count);
    if (r != UGOE_ERROR_BUFFER_TOO_SMALL) CHECK(r);
    if (part_count > 0) {
        parts = malloc(part_count * sizeof *parts);
        if (!parts) {
            ok = false;
            goto done;
        }
        CHECK(ugoe_puppet_part_opacities_applied(conductor, parts, part_count, &part_count));
    }
    CHECK(ugoe_puppet_follow(puppet, values, count, level, strength, parts, part_count, dt));
done:
    free(values);
    free(parts);
    return ok;
}

static bool run_case(const Case *c, const char *out_dir) {
    bool ok = true;
    UgoePuppet *puppet = NULL;
    UgoePuppet *conductor = NULL;
    UgoeModule *modules[MAX_ITEMS] = {0};
    Events events = {0};
    FILE *f = NULL;
    size_t len = 0;
    unsigned char *bytes = read_file(c->model, &len);
    if (!bytes) return false;

    if (!create_puppet(c, bytes, len, modules, &puppet)) {
        ok = false;
        goto done;
    }
    if (c->conduct && !create_conductor(c, bytes, len, modules, &conductor)) {
        ok = false;
        goto done;
    }
    if (c->step_count == 0) {
        const uint32_t stages[] = {UGOE_STAGE_PHYSICS, UGOE_STAGE_EYE_BLINK, UGOE_STAGE_BREATH,
                                   UGOE_STAGE_LIP_SYNC, UGOE_STAGE_POSE};
        for (size_t i = 0; i < sizeof stages / sizeof stages[0]; i++) {
            CHECK(ugoe_puppet_set_stage_enabled(puppet, stages[i], false));
        }
        CHECK(ugoe_puppet_update(puppet, 0.0f));
    } else {
        CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_EYE_BLINK, c->effects));
        CHECK(ugoe_puppet_set_stage_enabled(puppet, UGOE_STAGE_BREATH, c->effects));
        for (size_t i = 0; i < c->step_count; i++) {
            if (conductor) {
                if (!lead(c, conductor, puppet, c->lead_lip_sync[i], c->steps[i])) {
                    ok = false;
                    goto done;
                }
            } else if (c->follow_count > 0) {
                CHECK(ugoe_puppet_follow(puppet, c->follow[i], c->follow_len[i],
                                         c->follow_lip_sync[i][0], c->follow_lip_sync[i][1],
                                         c->follow_parts[i], c->follow_part_count[i],
                                         c->steps[i]));
            } else {
                for (size_t k = 0; k < c->start_count; k++) {
                    if (c->start_steps[k] != i) continue;
                    uint32_t index = 0;
                    bool started = false;
                    if (!motion_index(c, puppet, c->start_names[k], &index)) {
                        ok = false;
                        goto done;
                    }
                    CHECK(ugoe_puppet_start_motion(puppet, index, c->start_priorities[k], &started));
                    if (started != c->start_accepted[k]) {
                        fprintf(stderr, "%s: starting `%s` at step %zu gave %d\n", c->name,
                                c->start_names[k], i, (int)started);
                        ok = false;
                        goto done;
                    }
                }
                CHECK(ugoe_puppet_update(puppet, c->steps[i]));
                if (c->start_count > 0 && !record_events(c, puppet, i, &events)) {
                    ok = false;
                    goto done;
                }
            }
        }
    }

    char path[MAX_LINE + 300];
    snprintf(path, sizeof path, "%s/%s.json", out_dir, c->name);
    f = fopen(path, "w");
    if (!f) {
        perror(path);
        ok = false;
        goto done;
    }
    uint32_t count = 0;
    CHECK(ugoe_puppet_drawable_count(puppet, &count));
    fputs("{\"meshes\":{", f);
    for (uint32_t i = 0; i < count; i++) {
        UgoeDrawableInfo info;
        UgoeDrawableState state;
        const float *xy = NULL;
        size_t n = 0;
        CHECK(ugoe_puppet_drawable_info(puppet, i, &info));
        CHECK(ugoe_puppet_drawable_state(puppet, i, &state));
        CHECK(ugoe_puppet_positions(puppet, i, &xy, &n));
        if (i > 0) fputc(',', f);
        write_string(f, info.id);
        fputs(":{\"positions\":[", f);
        for (size_t v = 0; v + 1 < n; v += 2) {
            if (v > 0) fputc(',', f);
            fputc('[', f);
            write_float(f, xy[v]);
            fputc(',', f);
            write_float(f, xy[v + 1]);
            fputc(']', f);
        }
        fputs("],\"opacity\":", f);
        write_float(f, state.opacity);
        fputs(",\"draw_order\":", f);
        write_float(f, state.draw_order);
        fputs(",\"multiply_color\":", f);
        write_rgb(f, state.multiply_color);
        fputs(",\"screen_color\":", f);
        write_rgb(f, state.screen_color);
        fputs(",\"mask_strength\":", f);
        write_float(f, state.mask_strength);
        fprintf(f, ",\"visible\":%s}", state.visible ? "true" : "false");
    }
    fputs("},\"render_order\":[", f);
    const uint32_t *order = NULL;
    size_t order_len = 0;
    CHECK(ugoe_puppet_render_order(puppet, &order, &order_len));
    for (size_t i = 0; i < order_len; i++) {
        UgoeDrawableInfo info;
        CHECK(ugoe_puppet_drawable_info(puppet, order[i], &info));
        if (i > 0) fputc(',', f);
        write_string(f, info.id);
    }
    fputc(']', f);
    if (c->start_count > 0) {
        fputs(",\"motion_events\":[", f);
        for (size_t i = 0; i < events.len; i++) {
            const Event *e = &events.items[i];
            if (i > 0) fputc(',', f);
            fprintf(f, "{\"step\":%zu,\"value\":", e->step);
            write_string(f, e->value);
            fputs(",\"time\":", f);
            write_float(f, e->time);
            fputs(",\"motion\":", f);
            write_string(f, e->motion);
            fprintf(f, ",\"number\":%llu}", (unsigned long long)e->number);
        }
        fputc(']', f);
    }
    fputs("}\n", f);

done:
    if (f) fclose(f);
    clear_events(&events);
    for (size_t i = 0; i < c->module_count; i++) ugoe_module_destroy(modules[i]);
    ugoe_puppet_destroy(conductor);
    ugoe_puppet_destroy(puppet);
    free(bytes);
    return ok;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s <plan.txt> <output directory>\n", argv[0]);
        return 2;
    }
    if (ugoe_abi_version() != UGOE_ABI_VERSION) {
        fprintf(stderr, "ABI version %u does not match the header (%u)\n", ugoe_abi_version(),
                (unsigned)UGOE_ABI_VERSION);
        return 2;
    }
    FILE *plan = fopen(argv[1], "r");
    if (!plan) {
        perror(argv[1]);
        return 2;
    }
    static Case c;
    static char line[MAX_LINE];
    int failed = 0;
    int total = 0;
    while (fgets(line, sizeof line, plan)) {
        line[strcspn(line, "\n")] = '\0';
        char *arg = strchr(line, ' ');
        if (arg) *arg++ = '\0';
        if (strcmp(line, "case") == 0 && arg) {
            clear_case(&c);
            snprintf(c.name, sizeof c.name, "%s", arg);
        } else if (strcmp(line, "model") == 0 && arg) {
            char *path = strchr(arg, ' ');
            if (!path) goto invalid;
            *path++ = '\0';
            c.bundle = strcmp(arg, "bundle") == 0;
            snprintf(c.model, sizeof c.model, "%s", path);
        } else if (strcmp(line, "module") == 0 && arg) {
            if (c.module_count == MAX_ITEMS) goto invalid;
            c.modules[c.module_count++] = duplicate(arg);
        } else if (strcmp(line, "param") == 0 && arg) {
            if (c.param_count == MAX_ITEMS) goto invalid;
            char *value = strrchr(arg, ' ');
            if (!value) goto invalid;
            *value++ = '\0';
            c.param_ids[c.param_count] = duplicate(arg);
            c.param_values[c.param_count++] = parse_float(value);
        } else if (strcmp(line, "effects") == 0 && arg) {
            c.effects = strcmp(arg, "1") == 0;
        } else if (strcmp(line, "update") == 0 && arg) {
            if (c.step_count == MAX_STEPS || c.follow_count > 0 || c.conduct) goto invalid;
            c.steps[c.step_count++] = parse_float(arg);
        } else if (strcmp(line, "follow") == 0 && arg) {
            /* follow <dt> <リップシンクの音量 (無ければ -1)> <強さ> <パーツの数 k> (<番号> <値> <重み>) × k <値> ... */
            if (c.step_count == MAX_STEPS || c.follow_count != c.step_count || c.conduct) goto invalid;
            size_t n = 0;
            for (const char *p = arg; *p; p++) n += *p == ' ';
            float *values = malloc((n > 0 ? n : 1) * sizeof *values);
            if (!values) goto invalid;
            char *token = strtok(arg, " ");
            float *lip_sync = c.follow_lip_sync[c.follow_count];
            c.steps[c.step_count++] = parse_float(token);
            for (int k = 0; k < 2; k++) {
                token = strtok(NULL, " ");
                if (!token) {
                    free(values);
                    goto invalid;
                }
                lip_sync[k] = parse_float(token);
            }
            token = strtok(NULL, " ");
            if (!token) {
                free(values);
                goto invalid;
            }
            size_t part_count = strtoul(token, NULL, 10);
            UgoePartOpacityInput *parts = NULL;
            if (part_count > 0) {
                parts = malloc(part_count * sizeof *parts);
                if (!parts) {
                    free(values);
                    goto invalid;
                }
            }
            for (size_t k = 0; k < part_count * 3; k++) {
                token = strtok(NULL, " ");
                if (!token) {
                    free(values);
                    free(parts);
                    goto invalid;
                }
                UgoePartOpacityInput *part = &parts[k / 3];
                if (k % 3 == 0) part->part = (uint32_t)strtoul(token, NULL, 10);
                else if (k % 3 == 1) part->value = parse_float(token);
                else part->weight = parse_float(token);
            }
            c.follow_parts[c.follow_count] = parts;
            c.follow_part_count[c.follow_count] = part_count;
            size_t len = 0;
            while ((token = strtok(NULL, " ")) != NULL && len < n) values[len++] = parse_float(token);
            c.follow[c.follow_count] = values;
            c.follow_len[c.follow_count++] = len;
        } else if (strcmp(line, "conduct") == 0) {
            if (c.step_count > 0 || c.follow_count > 0) goto invalid;
            c.conduct = true;
            snprintf(c.conduct_motion, sizeof c.conduct_motion, "%s", arg ? arg : "");
        } else if (strcmp(line, "conduct_effects") == 0 && arg) {
            if (!c.conduct || c.conduct_effects) goto invalid;
            c.conduct_effects = duplicate(arg);
        } else if (strcmp(line, "lead") == 0 && arg) {
            /* lead <dt> <リップシンクの音量 (設定しなければ -1)> */
            if (!c.conduct || c.step_count == MAX_STEPS) goto invalid;
            char *level = strchr(arg, ' ');
            if (!level) goto invalid;
            *level++ = '\0';
            c.lead_lip_sync[c.step_count] = parse_float(level);
            c.steps[c.step_count++] = parse_float(arg);
        } else if (strcmp(line, "start") == 0 && arg) {
            /* start <step> <idle|normal|force> <0|1> <モーションの名前> */
            if (c.start_count == MAX_ITEMS || c.follow_count > 0 || c.conduct) goto invalid;
            char *priority = strchr(arg, ' ');
            if (!priority) goto invalid;
            *priority++ = '\0';
            char *accepted = strchr(priority, ' ');
            if (!accepted) goto invalid;
            *accepted++ = '\0';
            char *name = strchr(accepted, ' ');
            if (!name) goto invalid;
            *name++ = '\0';
            uint32_t value;
            if (strcmp(priority, "idle") == 0) value = UGOE_PRIORITY_IDLE;
            else if (strcmp(priority, "normal") == 0) value = UGOE_PRIORITY_NORMAL;
            else if (strcmp(priority, "force") == 0) value = UGOE_PRIORITY_FORCE;
            else goto invalid;
            c.start_steps[c.start_count] = strtoul(arg, NULL, 10);
            c.start_priorities[c.start_count] = value;
            c.start_accepted[c.start_count] = strcmp(accepted, "1") == 0;
            c.start_names[c.start_count++] = duplicate(name);
        } else if (strcmp(line, "end") == 0) {
            total++;
            if (!run_case(&c, argv[2])) failed++;
        } else if (line[0] != '\0') {
            goto invalid;
        }
        continue;
    invalid:
        fprintf(stderr, "%s: invalid line `%s`\n", argv[1], line);
        fclose(plan);
        return 2;
    }
    clear_case(&c);
    fclose(plan);
    printf("c: evaluated %d cases (%d failed)\n", total - failed, failed);
    return failed == 0 ? 0 : 1;
}
