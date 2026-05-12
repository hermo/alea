#include "https.h"

#include <bearssl.h>

#include "drand_cas.h"

#include <sys/socket.h>
#include <netdb.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int sock_read(void *ctx, unsigned char *buf, size_t len)
{
    int fd = *(int *)ctx;
    ssize_t r = read(fd, buf, len);
    if (r == 0)
        return -1;
    return (int)r;
}

static int sock_write(void *ctx, const unsigned char *buf, size_t len)
{
    int fd = *(int *)ctx;
    ssize_t r = write(fd, buf, len);
    if (r == 0)
        return -1;
    return (int)r;
}

static int tcp_connect(const char *host, char *errbuf, size_t errlen)
{
    struct addrinfo hints, *res, *p;
    int fd = -1;

    memset(&hints, 0, sizeof hints);
    hints.ai_family   = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;

    int err = getaddrinfo(host, "443", &hints, &res);
    if (err != 0) {
        snprintf(errbuf, errlen, "getaddrinfo(%s): %s", host, gai_strerror(err));
        return -1;
    }
    for (p = res; p; p = p->ai_next) {
        fd = socket(p->ai_family, p->ai_socktype, p->ai_protocol);
        if (fd < 0)
            continue;
        if (connect(fd, p->ai_addr, p->ai_addrlen) == 0)
            break;
        close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    if (fd < 0)
        snprintf(errbuf, errlen, "could not connect to %s:443", host);
    return fd;
}

int https_get(const char *host, const char *path,
              char **body_out, size_t *body_len_out,
              char *errbuf, size_t errlen)
{
    int fd = tcp_connect(host, errbuf, errlen);
    if (fd < 0)
        return -1;

    br_ssl_client_context sc;
    br_x509_minimal_context xc;
    static unsigned char iobuf[BR_SSL_BUFSIZE_BIDI];

    br_ssl_client_init_full(&sc, &xc, TAs, TAs_NUM);
    br_ssl_engine_set_buffer(&sc.eng, iobuf, sizeof iobuf, 1);
    if (!br_ssl_client_reset(&sc, host, 0)) {
        close(fd);
        snprintf(errbuf, errlen, "TLS reset failed");
        return -1;
    }

    br_sslio_context ioc;
    br_sslio_init(&ioc, &sc.eng, sock_read, &fd, sock_write, &fd);

    char req[1024];
    int reqlen = snprintf(req, sizeof req,
        "GET %s HTTP/1.0\r\nHost: %s\r\nConnection: close\r\n\r\n",
        path, host);
    if (br_sslio_write_all(&ioc, req, (size_t)reqlen) < 0 ||
        br_sslio_flush(&ioc) < 0)
    {
        close(fd);
        snprintf(errbuf, errlen, "TLS write failed (err=%d)",
                 br_ssl_engine_last_error(&sc.eng));
        return -1;
    }

    size_t cap = 65536, len = 0;
    char *buf = malloc(cap + 1);
    if (!buf) {
        close(fd);
        snprintf(errbuf, errlen, "out of memory");
        return -1;
    }

    for (;;) {
        if (len >= cap) {
            cap *= 2;
            char *tmp = realloc(buf, cap + 1);
            if (!tmp) {
                free(buf);
                close(fd);
                snprintf(errbuf, errlen, "out of memory");
                return -1;
            }
            buf = tmp;
        }
        int r = br_sslio_read(&ioc, buf + len, cap - len);
        if (r < 0)
            break;
        len += (size_t)r;
    }
    br_sslio_close(&ioc);
    close(fd);

    int tls_err = br_ssl_engine_last_error(&sc.eng);
    if (tls_err != BR_ERR_OK) {
        free(buf);
        snprintf(errbuf, errlen, "TLS error %d", tls_err);
        return -1;
    }

    buf[len] = '\0';

    int status = 0;
    if (sscanf(buf, "HTTP/%*s %d", &status) != 1) {
        free(buf);
        snprintf(errbuf, errlen, "invalid HTTP response");
        return -1;
    }

    char *body = strstr(buf, "\r\n\r\n");
    if (!body) {
        free(buf);
        snprintf(errbuf, errlen, "malformed HTTP response");
        return -1;
    }
    body += 4;

    size_t blen = len - (size_t)(body - buf);
    char *result = malloc(blen + 1);
    if (!result) {
        free(buf);
        snprintf(errbuf, errlen, "out of memory");
        return -1;
    }
    memcpy(result, body, blen);
    result[blen] = '\0';
    free(buf);

    *body_out    = result;
    *body_len_out = blen;
    return status;
}
