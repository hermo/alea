#include "format.h"
#include "drand.h"
#include "timeutil.h"

#include <stdio.h>
#include <string.h>

int format_is_shell(enum output_mode mode)
{
    return mode == OUTPUT_SH || mode == OUTPUT_FISH || mode == OUTPUT_PS || mode == OUTPUT_ALL;
}

void format_shell_quote(const char *s, char *buf, size_t len)
{
    int needs_quote = 0;
    for (const char *p = s; *p; p++) {
        if (!((*p >= 'a' && *p <= 'z') || (*p >= 'A' && *p <= 'Z') ||
              (*p >= '0' && *p <= '9') || *p == '-' || *p == '_' || *p == '.' || *p == '/')) {
            needs_quote = 1;
            break;
        }
    }
    if (!needs_quote) {
        snprintf(buf, len, "%s", s);
        return;
    }
    size_t pos = 0;
    if (pos < len - 1) buf[pos++] = '\'';
    for (const char *p = s; *p && pos < len - 5; p++) {
        if (*p == '\'') {
            buf[pos++] = '\'';
            buf[pos++] = '\\';
            buf[pos++] = '\'';
            buf[pos++] = '\'';
        } else {
            buf[pos++] = *p;
        }
    }
    if (pos < len - 1) buf[pos++] = '\'';
    buf[pos] = '\0';
}

static void ps_quote(const char *s, char *buf, size_t len)
{
    size_t pos = 0;
    if (pos < len - 1) buf[pos++] = '\'';
    for (const char *p = s; *p && pos < len - 3; p++) {
        if (*p == '\'') {
            buf[pos++] = '\'';
            buf[pos++] = '\'';
        } else {
            buf[pos++] = *p;
        }
    }
    if (pos < len - 1) buf[pos++] = '\'';
    buf[pos] = '\0';
}

void format_print_verify_args(const struct selection_result *r)
{
    if (r->count > 1)
        printf("--count %zu ", r->count);
    if (r->file) {
        const char *base = strrchr(r->file, '/');
        base = base ? base + 1 : r->file;
        char quoted[256];
        format_shell_quote(base, quoted, sizeof(quoted));
        printf("--file %s", quoted);
        if (r->delimiter) {
            char dq[256];
            format_shell_quote(r->delimiter, dq, sizeof(dq));
            printf(" --delimiter %s", dq);
        }
    } else {
        for (size_t i = 0; i < r->option_count; i++) {
            char quoted[256];
            format_shell_quote(r->options[i], quoted, sizeof(quoted));
            printf("%s%s", quoted, i < r->option_count - 1 ? " " : "");
        }
    }
}

static void print_header(const struct selection_result *r, const char *timestamp)
{
    if (r->count == 1) {
        printf("\xf0\x9f\x8e\xb2 %s\n", r->options[r->indices[0]]);
    } else {
        for (size_t i = 0; i < r->count; i++) {
            if (i == 0)
                printf("\xf0\x9f\x8e\xb2 %zu. %s\n", i + 1, r->options[r->indices[i]]);
            else
                printf("   %zu. %s\n", i + 1, r->options[r->indices[i]]);
        }
    }
    printf("\nround: %llu\n", (unsigned long long)r->round);
    printf("time:  %s\n", timestamp);
    if (r->input_hash)
        printf("input: sha256:%s\n", r->input_hash);
}

/* JSON string escaping */
static void print_json_str(const char *s)
{
    putchar('"');
    for (const char *p = s; *p; p++) {
        switch (*p) {
        case '"':  printf("\\\""); break;
        case '\\': printf("\\\\"); break;
        case '\n': printf("\\n"); break;
        case '\r': printf("\\r"); break;
        case '\t': printf("\\t"); break;
        default:   putchar(*p); break;
        }
    }
    putchar('"');
}

static void render_json(const struct selection_result *r, const char *timestamp)
{
    printf("{\"round\":%llu,\"randomness\":\"%s\"",
           (unsigned long long)r->round, r->randomness);
    if (r->count == 1) {
        printf(",\"index\":%zu,\"winner\":", r->indices[0]);
        print_json_str(r->options[r->indices[0]]);
    }
    printf(",\"winners\":[");
    for (size_t i = 0; i < r->count; i++) {
        if (i > 0) putchar(',');
        print_json_str(r->options[r->indices[i]]);
    }
    printf("],\"timestamp\":\"%s\",\"options\":[", timestamp);
    for (size_t i = 0; i < r->option_count; i++) {
        if (i > 0) putchar(',');
        print_json_str(r->options[i]);
    }
    putchar(']');
    if (r->input_hash)
        printf(",\"input_hash\":\"%s\"", r->input_hash);
    printf("}\n");
}

static void render_tsv(const struct selection_result *r, const char *timestamp)
{
    printf("round\t%llu\n", (unsigned long long)r->round);
    printf("randomness\t%s\n", r->randomness);
    printf("timestamp\t%s\n", timestamp);
    if (r->count == 1) {
        printf("index\t%zu\n", r->indices[0]);
        printf("winner\t%s\n", r->options[r->indices[0]]);
    } else {
        for (size_t i = 0; i < r->count; i++)
            printf("winner\t%zu\t%s\n", i + 1, r->options[r->indices[i]]);
    }
    if (r->input_hash)
        printf("input_hash\t%s\n", r->input_hash);
    printf("options");
    for (size_t i = 0; i < r->option_count; i++)
        printf("\t%s", r->options[i]);
    printf("\n");
}

static void print_oneliner_comment(const struct selection_result *r)
{
    printf("# alea ");
    format_print_verify_args(r);
    printf(" --round %llu => %s\n", (unsigned long long)r->round, r->options[r->indices[0]]);
}

static void render_sh(const struct selection_result *r, const char *indent)
{
    printf("%s", indent);
    print_oneliner_comment(r);
    printf("%sopts=(", indent);
    for (size_t i = 0; i < r->option_count; i++) {
        char quoted[256];
        format_shell_quote(r->options[i], quoted, sizeof(quoted));
        printf("%s%s", quoted, i < r->option_count - 1 ? " " : "");
    }
    printf("); r=$(curl -s https://api.drand.sh/public/%llu"
           " | grep -o '\"randomness\":\"[^\"]*\"' | cut -d'\"' -f4);"
           " i=$(printf \"%%d\" \"0x${r:0:8}\");"
           " echo \"${opts[@]:$((i %% ${#opts[@]})):1}\"\n",
           (unsigned long long)r->round);
}

static void render_fish(const struct selection_result *r, const char *indent)
{
    printf("%s", indent);
    print_oneliner_comment(r);
    printf("%sset opts ", indent);
    for (size_t i = 0; i < r->option_count; i++) {
        char quoted[256];
        format_shell_quote(r->options[i], quoted, sizeof(quoted));
        printf("%s%s", quoted, i < r->option_count - 1 ? " " : "");
    }
    printf("; set r (curl -s https://api.drand.sh/public/%llu"
           " | grep -o '\"randomness\":\"[^\"]*\"' | cut -d'\"' -f4);"
           " set i (printf \"%%d\" \"0x\"(string sub -l 8 $r));"
           " math (math $i %% (count $opts)) + 1 | read idx; echo $opts[$idx]\n",
           (unsigned long long)r->round);
}

static void render_ps(const struct selection_result *r, const char *indent)
{
    printf("%s", indent);
    print_oneliner_comment(r);
    printf("%s$opts=@(", indent);
    for (size_t i = 0; i < r->option_count; i++) {
        char quoted[256];
        ps_quote(r->options[i], quoted, sizeof(quoted));
        printf("%s%s", quoted, i < r->option_count - 1 ? "," : "");
    }
    printf(");$r=(Invoke-RestMethod https://api.drand.sh/public/%llu).randomness;"
           "$i=[Convert]::ToUInt32($r.Substring(0,8),16);$opts[$i%%$opts.Count]\n",
           (unsigned long long)r->round);
}

void format_render(const struct selection_result *r, enum output_mode mode, int quiet)
{
    char timestamp[32];
    epoch_to_iso(DRAND_GENESIS_TIME + r->round * DRAND_PERIOD, timestamp, sizeof(timestamp));

    switch (mode) {
    case OUTPUT_HUMAN:
        if (quiet) {
            for (size_t i = 0; i < r->count; i++)
                printf("%s\n", r->options[r->indices[i]]);
        } else {
            print_header(r, timestamp);
            if (!(r->file && strcmp(r->file, "-") == 0)) {
                printf("\nverify:\n  alea --round %llu ", (unsigned long long)r->round);
                format_print_verify_args(r);
                printf("\n");
            }
        }
        break;

    case OUTPUT_JSON:
        render_json(r, timestamp);
        break;

    case OUTPUT_TSV:
        render_tsv(r, timestamp);
        break;

    case OUTPUT_SH:
        if (quiet) {
            render_sh(r, "");
        } else {
            print_header(r, timestamp);
            printf("\nverify (bash/zsh):\n");
            render_sh(r, "  ");
        }
        break;

    case OUTPUT_FISH:
        if (quiet) {
            render_fish(r, "");
        } else {
            print_header(r, timestamp);
            printf("\nverify (fish):\n");
            render_fish(r, "  ");
        }
        break;

    case OUTPUT_PS:
        if (quiet) {
            render_ps(r, "");
        } else {
            print_header(r, timestamp);
            printf("\nverify (PowerShell):\n");
            render_ps(r, "  ");
        }
        break;

    case OUTPUT_ALL:
        if (!quiet)
            print_header(r, timestamp);
        printf("%sverify (alea):\n  alea --round %llu ", quiet ? "" : "\n",
               (unsigned long long)r->round);
        format_print_verify_args(r);
        printf("\n\nverify (bash/zsh):\n");
        render_sh(r, "  ");
        printf("\nverify (fish):\n");
        render_fish(r, "  ");
        printf("\nverify (PowerShell):\n");
        render_ps(r, "  ");
        break;
    }
}
