#include "drand.h"
#include "json.h"

#include <curl/curl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

struct buffer {
    char *data;
    size_t len;
    size_t cap;
};

static size_t write_cb(void *ptr, size_t size, size_t nmemb, void *userdata)
{
    struct buffer *buf = userdata;
    size_t total = size * nmemb;
    if (buf->len + total >= buf->cap) {
        size_t newcap = (buf->cap + total) * 2;
        char *tmp = realloc(buf->data, newcap);
        if (!tmp)
            return 0;
        buf->data = tmp;
        buf->cap = newcap;
    }
    memcpy(buf->data + buf->len, ptr, total);
    buf->len += total;
    buf->data[buf->len] = '\0';
    return total;
}

uint64_t drand_now_secs(void)
{
    return (uint64_t)time(NULL);
}

int drand_fetch(uint64_t round, struct drand_response *out, char *errbuf, size_t errlen)
{
    char url[256];
    if (round > 0)
        snprintf(url, sizeof(url), "%s/%llu", DRAND_BASE_URL, (unsigned long long)round);
    else
        snprintf(url, sizeof(url), "%s/latest", DRAND_BASE_URL);

    struct buffer buf = { .data = malloc(4096), .len = 0, .cap = 4096 };
    if (!buf.data) {
        snprintf(errbuf, errlen, "out of memory");
        return -1;
    }
    buf.data[0] = '\0';

    CURL *curl = curl_easy_init();
    if (!curl) {
        free(buf.data);
        snprintf(errbuf, errlen, "curl init failed");
        return -1;
    }

    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buf);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 10L);

    CURLcode res = curl_easy_perform(curl);
    if (res != CURLE_OK) {
        snprintf(errbuf, errlen, "drand request failed: %s", curl_easy_strerror(res));
        curl_easy_cleanup(curl);
        free(buf.data);
        return -1;
    }

    long http_code = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &http_code);
    curl_easy_cleanup(curl);

    if (http_code == 425) {
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
        free(buf.data);
        return -1;
    }

    if (http_code == 500 && round > 0) {
        uint64_t available_at = DRAND_GENESIS_TIME + round * DRAND_PERIOD;
        uint64_t now = drand_now_secs();
        uint64_t diff = available_at > now ? available_at - now : now - available_at;
        if (diff <= DRAND_PERIOD) {
            snprintf(errbuf, errlen,
                     "round %llu is not yet available -- it is being generated, try again in a moment",
                     (unsigned long long)round);
            free(buf.data);
            return -1;
        }
    }

    if (http_code != 200) {
        snprintf(errbuf, errlen, "drand request failed: HTTP %ld", http_code);
        free(buf.data);
        return -1;
    }

    struct json_value_s *root = json_parse(buf.data, buf.len);
    free(buf.data);
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
