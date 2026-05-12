#include <errno.h>
#include <getopt.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#ifdef USE_BEARSSL
#  include <bearssl.h>
#  define SHA256_DIGEST_LENGTH 32
#  define SHA256(d, n, md) do { \
    br_sha256_context _sha_ctx; \
    br_sha256_init(&_sha_ctx); \
    br_sha256_update(&_sha_ctx, (d), (n)); \
    br_sha256_out(&_sha_ctx, (md)); \
} while (0)
#elif defined(__APPLE__)
#  include <CommonCrypto/CommonDigest.h>
#  define SHA256_DIGEST_LENGTH CC_SHA256_DIGEST_LENGTH
#  define SHA256(d, n, md)     CC_SHA256((d), (n), (md))
#else
#  include <openssl/sha.h>
#endif

#include "drand.h"
#include "format.h"
#include "select.h"
#include "timeutil.h"

#define VERSION "0.9.10"
#define MAX_WINNERS 8

static void usage(void)
{
    fprintf(stderr,
        "alea %s - verifiable random selection using drand\n\n"
        "Usage: alea [OPTIONS] <option1> <option2> [option3...]\n"
        "       alea [OPTIONS] --file <path> [--delimiter <delim>]\n\n"
        "Options:\n"
        "  --round <N>           Use a specific drand round (for verification)\n"
        "  --at <TIMESTAMP>      Calculate round for a specific time (ISO 8601)\n"
        "  -f, --file <path>     Read options from a file (use - for stdin)\n"
        "  -d, --delimiter <str> Split file by delimiter (default: newline)\n"
        "  -n, --count <N>       Pick N unique winners (default: 1, max: 8)\n"
        "  -q, --quiet           Print only the result, no headers or labels\n"
        "  --all                 Show all verification methods\n"
        "  --json                Machine-readable JSON output\n"
        "  --tsv                 Tab-separated key/value output\n"
        "  --sh                  Output bash/zsh verification oneliner\n"
        "  --fish                Output fish verification oneliner\n"
        "  --ps                  Output PowerShell verification oneliner\n"
        "  -V, --version         Show version\n"
        "  -h, --help            Show this help\n",
        VERSION);
}

static void hex_sha256(const unsigned char *data, size_t len, char *out)
{
    unsigned char hash[SHA256_DIGEST_LENGTH];
    SHA256(data, len, hash);
    for (int i = 0; i < SHA256_DIGEST_LENGTH; i++)
        snprintf(out + (size_t)i * 2, 3, "%02x", hash[i]);
}

static unsigned char *read_all(const char *path, size_t *out_len)
{
    FILE *f;
    if (strcmp(path, "-") == 0) {
        if (isatty(STDIN_FILENO)) {
            fprintf(stderr, "error: --file - requires piped input (stdin is a terminal)\n");
            return NULL;
        }
        f = stdin;
    } else {
        f = fopen(path, "rb");
        if (!f) {
            fprintf(stderr, "error: cannot read %s: %s\n", path, strerror(errno));
            return NULL;
        }
    }

    size_t cap = 4096, len = 0;
    unsigned char *buf = malloc(cap);
    if (!buf) { if (f != stdin) fclose(f); return NULL; }

    size_t n;
    while ((n = fread(buf + len, 1, cap - len, f)) > 0) {
        len += n;
        if (len == cap) {
            cap *= 2;
            unsigned char *tmp = realloc(buf, cap);
            if (!tmp) { free(buf); if (f != stdin) fclose(f); return NULL; }
            buf = tmp;
        }
    }
    if (f != stdin) fclose(f);
    *out_len = len;
    return buf;
}

static char **split_options(const char *text, size_t text_len, const char *delim, size_t *out_count)
{
    size_t cap = 64, count = 0;
    char **opts = malloc(cap * sizeof(char *));
    if (!opts) return NULL;

    size_t delim_len = strlen(delim);
    const char *p = text;
    const char *end = text + text_len;

    while (p < end) {
        const char *next = NULL;
        if (delim_len == 1) {
            next = memchr(p, delim[0], (size_t)(end - p));
        } else {
            for (const char *s = p; s <= end - delim_len; s++) {
                if (memcmp(s, delim, delim_len) == 0) { next = s; break; }
            }
        }
        if (!next) next = end;

        /* Trim whitespace (but not the delimiter itself) */
        const char *start = p;
        const char *stop = next;
        while (start < stop && (*start == ' ' || *start == '\t' || *start == '\r')) start++;
        while (stop > start && (stop[-1] == ' ' || stop[-1] == '\t' || stop[-1] == '\r')) stop--;

        if (stop > start) {
            if (count == cap) {
                cap *= 2;
                char **tmp = realloc(opts, cap * sizeof(char *));
                if (!tmp) goto oom;
                opts = tmp;
            }
            opts[count] = strndup(start, (size_t)(stop - start));
            if (!opts[count]) goto oom;
            count++;
        }
        p = next + delim_len;
    }

    *out_count = count;
    return opts;

oom:
    for (size_t i = 0; i < count; i++) free(opts[i]);
    free(opts);
    return NULL;
}

/* Owns and frees a dynamically allocated options array */
struct options_buf {
    char **items;
    size_t count;
};

static void options_buf_free(struct options_buf *ob)
{
    if (ob->items) {
        for (size_t i = 0; i < ob->count; i++)
            free(ob->items[i]);
        free(ob->items);
        ob->items = NULL;
    }
}


int main(int argc, char **argv)
{
    uint64_t round = 0;
    bool quiet = false;
    size_t count = 1;
    const char *file = NULL;
    const char *delimiter = NULL;
    const char *at = NULL;
    enum output_mode output = OUTPUT_HUMAN;

    enum { OPT_JSON = 256, OPT_TSV, OPT_SH, OPT_FISH, OPT_PS, OPT_ALL, OPT_AT };

    static struct option long_opts[] = {
        {"round",     required_argument, NULL, 'r'},
        {"at",        required_argument, NULL, OPT_AT},
        {"count",     required_argument, NULL, 'n'},
        {"file",      required_argument, NULL, 'f'},
        {"delimiter", required_argument, NULL, 'd'},
        {"quiet",     no_argument,       NULL, 'q'},
        {"json",      no_argument,       NULL, OPT_JSON},
        {"tsv",       no_argument,       NULL, OPT_TSV},
        {"sh",        no_argument,       NULL, OPT_SH},
        {"fish",      no_argument,       NULL, OPT_FISH},
        {"ps",        no_argument,       NULL, OPT_PS},
        {"all",       no_argument,       NULL, OPT_ALL},
        {"version",   no_argument,       NULL, 'V'},
        {"help",      no_argument,       NULL, 'h'},
        {NULL, 0, NULL, 0}
    };

    int opt;
    while ((opt = getopt_long(argc, argv, "n:f:d:qVh", long_opts, NULL)) != -1) {
        switch (opt) {
        case 'r': {
            char *end;
            round = strtoull(optarg, &end, 10);
            if (*end != '\0' || round == 0) {
                fprintf(stderr, "error: invalid round number\n");
                return 2;
            }
            break;
        }
        case OPT_AT:
            at = optarg;
            break;
        case 'n': {
            char *end;
            unsigned long v = strtoul(optarg, &end, 10);
            if (*end != '\0' || v == 0 || v > MAX_WINNERS) {
                fprintf(stderr, "error: --count must be between 1 and %d\n", MAX_WINNERS);
                return 2;
            }
            count = v;
            break;
        }
        case 'f': file = optarg; break;
        case 'd': delimiter = optarg; break;
        case 'q': quiet = true; break;
        case OPT_JSON: output = OUTPUT_JSON; break;
        case OPT_TSV:  output = OUTPUT_TSV; break;
        case OPT_SH:   output = OUTPUT_SH; break;
        case OPT_FISH: output = OUTPUT_FISH; break;
        case OPT_PS:   output = OUTPUT_PS; break;
        case OPT_ALL:  output = OUTPUT_ALL; break;
        case 'V':
            fprintf(stderr, "alea %s\n", VERSION);
            return 0;
        case 'h':
            usage();
            return 0;
        default:
            usage();
            return 2;
        }
    }

    /* Validate flag combinations */
    if (at && round) {
        fprintf(stderr, "error: --at and --round cannot be used together\n");
        return 2;
    }
    if (at && file && strcmp(file, "-") == 0) {
        fprintf(stderr, "error: --file - cannot be used with --at (stdin is ephemeral)\n");
        return 2;
    }
    if (file && format_is_shell(output)) {
        fprintf(stderr, "error: --sh/--fish/--ps/--all cannot be used with --file\n");
        return 2;
    }
    if (at && format_is_shell(output)) {
        fprintf(stderr, "error: --sh/--fish/--ps/--all cannot be used with --at\n");
        return 2;
    }
    if (count > 1 && format_is_shell(output)) {
        fprintf(stderr, "error: --sh/--fish/--ps/--all cannot be used with --count > 1\n");
        return 2;
    }

    /* Resolve --at to a round number */
    bool at_mode = false, at_past = false;
    if (at) {
        uint64_t epoch;
        if (parse_iso8601(at, &epoch) != 0) {
            fprintf(stderr, "error: invalid timestamp: %s (expected ISO 8601, e.g. 2026-07-22T12:00:00+03:00)\n", at);
            return 2;
        }
        if (epoch <= DRAND_GENESIS_TIME) {
            fprintf(stderr, "error: --at timestamp is before drand genesis\n");
            return 2;
        }
        round = (epoch - DRAND_GENESIS_TIME) / DRAND_PERIOD;
        at_mode = true;
        at_past = epoch <= drand_now_secs();
    }

    /* Collect options */
    struct options_buf ob = {0};
    char **options = NULL;
    size_t option_count = 0;
    char input_hash[65] = {0};

    if (file) {
        size_t file_len;
        unsigned char *contents = read_all(file, &file_len);
        if (!contents) return 2;

        hex_sha256(contents, file_len, input_hash);

        const char *delim = delimiter ? delimiter : "\n";
        ob.items = split_options((const char *)contents, file_len, delim, &ob.count);
        free(contents);
        if (!ob.items) {
            fprintf(stderr, "error: out of memory\n");
            return 1;
        }
        options = ob.items;
        option_count = ob.count;
    } else {
        int nopts = argc - optind;
        if (nopts < 2) {
            fprintf(stderr, "error: at least 2 options required\n\n");
            usage();
            return 2;
        }
        options = &argv[optind];
        option_count = (size_t)nopts;
    }

    if (option_count < 2) {
        fprintf(stderr, "error: at least 2 options required\n\n");
        usage();
        options_buf_free(&ob);
        return 2;
    }
    if (count > option_count) {
        fprintf(stderr, "error: --count %zu exceeds the number of options (%zu)\n", count, option_count);
        options_buf_free(&ob);
        return 2;
    }

    /* Build result struct (shared by --at and normal paths) */
    struct selection_result result = {
        .round = round,
        .randomness = NULL,
        .indices = NULL,
        .count = count,
        .options = options,
        .option_count = option_count,
        .input_hash = input_hash[0] ? input_hash : NULL,
        .file = file,
        .delimiter = delimiter,
    };

    int rc;
    if (at_mode) {
        format_render_scheduled(&result, quiet, at_past);
        rc = 0;
    } else {
        /* Fetch drand */
        struct drand_response resp;
        char errbuf[256];
        if (drand_fetch(round, &resp, errbuf, sizeof(errbuf)) != 0) {
            fprintf(stderr, "error: %s\n", errbuf);
            options_buf_free(&ob);
            return 1;
        }

        /* Select winners */
        size_t indices[MAX_WINNERS];
        if (alea_select_multiple(resp.randomness, option_count, count, indices) != 0) {
            fprintf(stderr, "error: selection failed\n");
            options_buf_free(&ob);
            return 1;
        }

        result.round = resp.round;
        result.randomness = resp.randomness;
        result.indices = indices;
        format_render(&result, output, quiet);
        rc = 0;
    }

    options_buf_free(&ob);
    return rc;
}
