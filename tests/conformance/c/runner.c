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
 * - follow の行 (受け手として進めるケース。T0133・T0134) は、update の代わりに
 *   ugoe_puppet_follow(値, リップシンクの音量, 強さ, dt) を順に呼ぶ。
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
    size_t follow_count;
} Case;

static void clear_case(Case *c) {
    for (size_t i = 0; i < c->module_count; i++) free(c->modules[i]);
    for (size_t i = 0; i < c->param_count; i++) free(c->param_ids[i]);
    for (size_t i = 0; i < c->follow_count; i++) free(c->follow[i]);
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

static bool run_case(const Case *c, const char *out_dir) {
    bool ok = true;
    UgoePuppet *puppet = NULL;
    UgoeModule *modules[MAX_ITEMS] = {0};
    FILE *f = NULL;
    size_t len = 0;
    unsigned char *bytes = read_file(c->model, &len);
    if (!bytes) return false;

    if (c->bundle) {
        CHECK(ugoe_puppet_create_from_bundle(bytes, len, &puppet));
    } else {
        CHECK(ugoe_puppet_create_from_json(bytes, len, &puppet));
    }
    for (size_t i = 0; i < c->module_count; i++) {
        size_t module_len = 0;
        unsigned char *module_bytes = read_file(c->modules[i], &module_len);
        if (!module_bytes) {
            ok = false;
            goto done;
        }
        UgoeResult r = ugoe_module_create(module_bytes, module_len, &modules[i]);
        free(module_bytes);
        CHECK(r);
        CHECK(ugoe_puppet_attach_module(puppet, modules[i]));
    }
    for (size_t i = 0; i < c->param_count; i++) {
        uint32_t index = 0;
        CHECK(ugoe_puppet_parameter_index(puppet, c->param_ids[i], &index));
        CHECK(ugoe_puppet_set_parameter(puppet, index, c->param_values[i]));
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
            if (c->follow_count > 0) {
                CHECK(ugoe_puppet_follow(puppet, c->follow[i], c->follow_len[i],
                                         c->follow_lip_sync[i][0], c->follow_lip_sync[i][1],
                                         c->steps[i]));
            } else {
                CHECK(ugoe_puppet_update(puppet, c->steps[i]));
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
    fputs("]}\n", f);

done:
    if (f) fclose(f);
    for (size_t i = 0; i < c->module_count; i++) ugoe_module_destroy(modules[i]);
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
            if (c.step_count == MAX_STEPS || c.follow_count > 0) goto invalid;
            c.steps[c.step_count++] = parse_float(arg);
        } else if (strcmp(line, "follow") == 0 && arg) {
            /* follow <dt> <リップシンクの音量 (無ければ -1)> <強さ> <値> <値> ... */
            if (c.step_count == MAX_STEPS || c.follow_count != c.step_count) goto invalid;
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
            size_t len = 0;
            while ((token = strtok(NULL, " ")) != NULL && len < n) values[len++] = parse_float(token);
            c.follow[c.follow_count] = values;
            c.follow_len[c.follow_count++] = len;
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
