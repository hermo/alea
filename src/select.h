#ifndef SELECT_H
#define SELECT_H

#include <stddef.h>
#include <stdint.h>

/* Pick `pick_count` unique winners. `indices` must have room for pick_count entries.
   Returns 0 on success, -1 on error. */
int alea_select_multiple(const char *randomness, size_t option_count, size_t pick_count, size_t *indices);

#endif
