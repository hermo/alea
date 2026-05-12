#include "timeutil.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int is_leap(int64_t y)
{
    return y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
}

static const int mdays[2][12] = {
    {31,28,31,30,31,30,31,31,30,31,30,31},
    {31,29,31,30,31,30,31,31,30,31,30,31}
};

void epoch_to_iso(uint64_t epoch, char *buf, size_t len)
{
    int64_t t = (int64_t)epoch;
    int64_t days = t / 86400;
    int64_t rem = t % 86400;
    int h = (int)(rem / 3600);
    int m = (int)((rem % 3600) / 60);
    int s = (int)(rem % 60);

    int y = 1970;
    while (1) {
        int yd = is_leap(y) ? 366 : 365;
        if (days < yd) break;
        days -= yd;
        y++;
    }

    int leap = is_leap(y);
    int mo = 0;
    while (mo < 12 && days >= mdays[leap][mo]) {
        days -= mdays[leap][mo];
        mo++;
    }

    snprintf(buf, len, "%04d-%02d-%02dT%02d:%02d:%02dZ", y, mo + 1, (int)days + 1, h, m, s);
}

int parse_iso8601(const char *s, uint64_t *out)
{
    size_t slen = strlen(s);
    const char *datetime_str;
    size_t datetime_len;
    int64_t offset_secs = 0;

    if (slen > 0 && s[slen - 1] == 'Z') {
        datetime_len = slen - 1;
        datetime_str = s;
        offset_secs = 0;
    } else if (slen >= 6 && (s[slen - 6] == '+' || s[slen - 6] == '-')) {
        datetime_len = slen - 6;
        datetime_str = s;
        const char *tz = s + slen - 6;
        int64_t sign = (tz[0] == '-') ? -1 : 1;
        char hbuf[3] = {tz[1], tz[2], '\0'};
        char mbuf[3] = {tz[4], tz[5], '\0'};
        int64_t th = strtol(hbuf, NULL, 10);
        int64_t tm = strtol(mbuf, NULL, 10);
        offset_secs = sign * (th * 3600 + tm * 60);
    } else {
        return -1;
    }

    /* Parse YYYY-MM-DDTHH:MM:SS */
    if (datetime_len != 19) return -1;

    char dtbuf[20];
    memcpy(dtbuf, datetime_str, 19);
    dtbuf[19] = '\0';

    if (dtbuf[4] != '-' || dtbuf[7] != '-' || dtbuf[10] != 'T' ||
        dtbuf[13] != ':' || dtbuf[16] != ':')
        return -1;

    dtbuf[4] = dtbuf[7] = dtbuf[10] = dtbuf[13] = dtbuf[16] = '\0';

    int64_t year = strtol(dtbuf, NULL, 10);
    uint64_t month = strtoul(dtbuf + 5, NULL, 10);
    uint64_t day = strtoul(dtbuf + 8, NULL, 10);
    uint64_t hour = strtoul(dtbuf + 11, NULL, 10);
    uint64_t min = strtoul(dtbuf + 14, NULL, 10);
    uint64_t sec = strtoul(dtbuf + 17, NULL, 10);

    if (month < 1 || month > 12 || day < 1 || day > 31 ||
        hour > 23 || min > 59 || sec > 59)
        return -1;

    int64_t days = 0;
    for (int64_t y = 1970; y < year; y++)
        days += is_leap(y) ? 366 : 365;

    int leap = is_leap(year);
    for (uint64_t m = 0; m < month - 1; m++)
        days += mdays[leap][m];
    days += (int64_t)day - 1;

    int64_t epoch = days * 86400 + (int64_t)hour * 3600 + (int64_t)min * 60 + (int64_t)sec - offset_secs;
    *out = (uint64_t)epoch;
    return 0;
}
