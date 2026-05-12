#ifndef FORMAT_H
#define FORMAT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

enum output_mode {
    OUTPUT_HUMAN,
    OUTPUT_JSON,
    OUTPUT_TSV,
    OUTPUT_SH,
    OUTPUT_FISH,
    OUTPUT_PS,
    OUTPUT_ALL
};

struct selection_result {
    uint64_t round;
    const char *randomness;
    size_t *indices;
    size_t count;
    char **options;
    size_t option_count;
    const char *input_hash;  /* NULL if not from file */
    const char *file;        /* NULL if positional args */
    const char *delimiter;   /* NULL if default */
};

/* Render the full output for a completed selection. */
void format_render(const struct selection_result *r, enum output_mode mode, bool quiet);

/* Render output for a scheduled (--at) selection. */
void format_render_scheduled(const struct selection_result *r, bool quiet, bool at_past);

/* Returns non-zero if mode is a shell oneliner mode (sh/fish/ps/all). */
int format_is_shell(enum output_mode mode);

#endif
