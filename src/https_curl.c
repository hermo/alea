#include "https.h"

#include <curl/curl.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

struct buf {
    char  *data;
    size_t len;
};

static size_t write_cb(char *ptr, size_t size, size_t nmemb, void *userdata)
{
    struct buf *b = userdata;
    size_t n = size * nmemb;
    char *tmp = realloc(b->data, b->len + n + 1);
    if (!tmp)
        return 0;
    b->data = tmp;
    memcpy(b->data + b->len, ptr, n);
    b->len += n;
    b->data[b->len] = '\0';
    return n;
}

int https_get(const char *host, const char *path,
              char **body_out, size_t *body_len_out,
              char *errbuf, size_t errlen)
{
    CURL *curl = curl_easy_init();
    if (!curl) {
        snprintf(errbuf, errlen, "curl_easy_init failed");
        return -1;
    }

    char url[512];
    snprintf(url, sizeof url, "https://%s%s", host, path);

    struct buf body = {0};
    char curl_err[CURL_ERROR_SIZE] = {0};

    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &body);
    curl_easy_setopt(curl, CURLOPT_ERRORBUFFER, curl_err);

    CURLcode res = curl_easy_perform(curl);
    if (res != CURLE_OK) {
        snprintf(errbuf, errlen, "curl: %s",
                 curl_err[0] ? curl_err : curl_easy_strerror(res));
        free(body.data);
        curl_easy_cleanup(curl);
        return -1;
    }

    long status = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    curl_easy_cleanup(curl);

    if (!body.data)
        body.data = calloc(1, 1);

    *body_out     = body.data;
    *body_len_out = body.len;
    return (int)status;
}
