#include "drand.h"
#include "https.h"
#include "json.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

uint64_t drand_now_secs(void)
{
    return (uint64_t)time(NULL);
}

int drand_fetch(uint64_t round, struct drand_response *out, char *errbuf, size_t errlen)
{
    char path[256];
    if (round > 0)
        snprintf(path, sizeof(path), "/public/%llu", (unsigned long long)round);
    else
        snprintf(path, sizeof(path), "/public/latest");

    char *body = NULL;
    size_t body_len = 0;
    int status = https_get(DRAND_HOST, path, &body, &body_len, errbuf, errlen);
    if (status < 0)
        return -1;

    if (status == 425) {
        free(body);
        if (round > 0) {
            uint64_t available_at = DRAND_GENESIS_TIME + round * DRAND_PERIOD;
            uint64_t now = drand_now_secs();
            uint64_t secs = available_at > now ? available_at - now : 0;
            if (secs >= 3600)
                snprintf(errbuf, errlen, "round %llu is in the future (available in ~%llu h %llu min)",
                         (unsigned long long)round, (unsigned long long)(secs / 3600),
                         (unsigned long long)((secs % 3600) / 60));
            else if (secs >= 60)
                snprintf(errbuf, errlen, "round %llu is in the future (available in ~%llu min)",
                         (unsigned long long)round, (unsigned long long)(secs / 60));
            else
                snprintf(errbuf, errlen, "round %llu is in the future (available in ~%llu s)",
                         (unsigned long long)round, (unsigned long long)secs);
        } else {
            snprintf(errbuf, errlen, "drand request failed: HTTP 425");
        }
        return -1;
    }

    if (status == 500 && round > 0) {
        uint64_t available_at = DRAND_GENESIS_TIME + round * DRAND_PERIOD;
        uint64_t now = drand_now_secs();
        uint64_t diff = available_at > now ? available_at - now : now - available_at;
        if (diff <= DRAND_PERIOD) {
            free(body);
            snprintf(errbuf, errlen,
                     "round %llu is not yet available -- it is being generated, try again in a moment",
                     (unsigned long long)round);
            return -1;
        }
    }

    if (status != 200) {
        free(body);
        snprintf(errbuf, errlen, "drand request failed: HTTP %d", status);
        return -1;
    }

    struct json_value_s *root = json_parse(body, body_len);
    free(body);
    if (!root) {
        snprintf(errbuf, errlen, "failed to parse drand response");
        return -1;
    }

    struct json_object_s *obj = json_value_as_object(root);
    if (!obj) {
        snprintf(errbuf, errlen, "invalid drand response format");
        free(root);
        return -1;
    }

    const char *rand_str = NULL;
    uint64_t round_val = 0;
    int found = 0;

    for (struct json_object_element_s *e = obj->start; e; e = e->next) {
        if (strcmp(e->name->string, "round") == 0) {
            struct json_number_s *num = json_value_as_number(e->value);
            if (num) { round_val = strtoull(num->number, NULL, 10); found |= 1; }
        } else if (strcmp(e->name->string, "randomness") == 0) {
            struct json_string_s *str = json_value_as_string(e->value);
            if (str) { rand_str = str->string; found |= 2; }
        }
    }

    if (found != 3) {
        snprintf(errbuf, errlen, "invalid drand response format");
        free(root);
        return -1;
    }

    out->round = round_val;
    snprintf(out->randomness, sizeof(out->randomness), "%s", rand_str);

    free(root);
    return 0;
}
