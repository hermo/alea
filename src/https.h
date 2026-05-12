#ifndef ALEA_HTTPS_H
#define ALEA_HTTPS_H

#include <stddef.h>

/*
 * Perform an HTTPS GET. Returns the HTTP status code, or -1 on error.
 * On success, *body is a malloc'd, NUL-terminated string that the caller
 * must free; *body_len is its length excluding the NUL.
 */
int https_get(const char *host, const char *path,
              char **body, size_t *body_len,
              char *errbuf, size_t errlen);

#endif
