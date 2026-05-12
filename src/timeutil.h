#ifndef TIME_H
#define TIME_H

#include <stddef.h>
#include <stdint.h>

/* Parse ISO 8601 timestamp to unix epoch seconds. Returns 0 on success, -1 on error. */
int parse_iso8601(const char *s, uint64_t *out);

/* Format epoch seconds as ISO 8601 UTC string. buf must be at least 21 bytes. */
void epoch_to_iso(uint64_t epoch, char *buf, size_t len);

#endif
