#ifndef DRAND_H
#define DRAND_H

#include <stddef.h>
#include <stdint.h>

#define DRAND_GENESIS_TIME 1595431050ULL
#define DRAND_PERIOD       30ULL
#define DRAND_HOST         "api.drand.sh"
#define DRAND_BASE_URL     "https://api.drand.sh/public"

struct drand_response {
    uint64_t round;
    char randomness[128]; /* 64 hex chars + null, with margin */
};

/* Fetch a drand round. If round == 0, fetches latest. Returns 0 on success, -1 on error. */
int drand_fetch(uint64_t round, struct drand_response *out, char *errbuf, size_t errlen);

uint64_t drand_now_secs(void);

#endif
