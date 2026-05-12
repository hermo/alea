#include "select.h"

#include <stdlib.h>
#include <string.h>

int alea_select(const char *randomness, size_t count)
{
    if (strlen(randomness) < 8 || count == 0)
        return -1;

    char chunk[9];
    memcpy(chunk, randomness, 8);
    chunk[8] = '\0';

    char *end;
    unsigned long val = strtoul(chunk, &end, 16);
    if (*end != '\0')
        return -1;

    return (int)(val % count);
}

int alea_select_multiple(const char *randomness, size_t option_count, size_t pick_count, size_t *indices)
{
    if (pick_count > 8 || pick_count > option_count)
        return -1;
    if (strlen(randomness) < pick_count * 8)
        return -1;

    /* Build pool of remaining indices */
    size_t *pool = malloc(option_count * sizeof(*pool));
    if (!pool)
        return -1;
    for (size_t i = 0; i < option_count; i++)
        pool[i] = i;

    size_t pool_size = option_count;
    for (size_t k = 0; k < pick_count; k++) {
        int idx = alea_select(randomness + k * 8, pool_size);
        if (idx < 0) {
            free(pool);
            return -1;
        }
        indices[k] = pool[idx];
        /* Remove selected from pool (shift left to preserve order, matching Rust's Vec::remove) */
        memmove(&pool[idx], &pool[idx + 1], (pool_size - idx - 1) * sizeof(*pool));
        pool_size--;
    }

    free(pool);
    return 0;
}
