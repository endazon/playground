#include "cfg.h"
#include <stdint.h>

static int is_space(char c) { return c == ' ' || c == '\t' || c == '\n' || c == '\r'; }

/* "key" の直後の ':' と空白を飛ばした位置を返す。無ければ 0 */
static const char *find_value(const char *json, size_t len, const char *key) {
    size_t klen = 0; while (key[klen]) klen++;
    size_t i = 0;
    while (i < len) {
        while (i < len && json[i] != '"') i++;
        if (i >= len) return 0;
        size_t start = ++i;
        while (i < len && json[i] != '"') i++;
        if (i >= len) return 0;
        size_t flen = i - start;
        i++;
        int match = (flen == klen);
        for (size_t k = 0; match && k < klen; k++) if (json[start + k] != key[k]) match = 0;
        if (match) {
            while (i < len && is_space(json[i])) i++;
            if (i >= len || json[i] != ':') return 0;
            i++;
            while (i < len && is_space(json[i])) i++;
            return json + i;
        }
    }
    return 0;
}

/* 10 進数を正しく丸めて double にする (仮数 15 桁以下・指数 |e|<=22 の Clinger 高速経路) */
static double pow10_exact(int e) {
    static const double p[] = {1e0,1e1,1e2,1e3,1e4,1e5,1e6,1e7,1e8,1e9,1e10,1e11,1e12,1e13,1e14,1e15,1e16,1e17,1e18,1e19,1e20,1e21,1e22};
    return p[e];
}

double cfg_number(const char *json, size_t len, const char *key, double def) {
    const char *v = find_value(json, len, key);
    if (!v) return def;
    const char *end = json + len;
    int neg = 0;
    if (v < end && (*v == '-' || *v == '+')) { neg = (*v == '-'); v++; }
    uint64_t mant = 0; int digits = 0, frac_digits = 0, seen = 0;
    while (v < end && *v >= '0' && *v <= '9') { if (digits < 19) { mant = mant * 10 + (uint64_t)(*v - '0'); digits++; } else { frac_digits--; } v++; seen = 1; }
    if (v < end && *v == '.') {
        v++;
        while (v < end && *v >= '0' && *v <= '9') { if (digits < 19) { mant = mant * 10 + (uint64_t)(*v - '0'); digits++; frac_digits++; } v++; seen = 1; }
    }
    if (!seen) return def;
    int exp10 = 0;
    if (v < end && (*v == 'e' || *v == 'E')) {
        v++; int eneg = 0;
        if (v < end && (*v == '-' || *v == '+')) { eneg = (*v == '-'); v++; }
        while (v < end && *v >= '0' && *v <= '9') { exp10 = exp10 * 10 + (*v - '0'); v++; }
        if (eneg) exp10 = -exp10;
    }
    int scale = exp10 - frac_digits;
    double r = (double)mant;
    if (scale >= 0) {
        while (scale > 22) { r *= 1e22; scale -= 22; }
        r *= pow10_exact(scale);
    } else {
        while (scale < -22) { r /= 1e22; scale += 22; }
        r /= pow10_exact(-scale);
    }
    return neg ? -r : r;
}

const char *cfg_string(const char *json, size_t len, const char *key, size_t *out_len) {
    const char *v = find_value(json, len, key);
    if (!v || v >= json + len || *v != '"') return 0;
    v++;
    const char *p = v;
    while (p < json + len && *p != '"') p++;
    if (p >= json + len) return 0;
    *out_len = (size_t)(p - v);
    return v;
}

int cfg_string_eq(const char *json, size_t len, const char *key, const char *expect) {
    size_t n = 0;
    const char *s = cfg_string(json, len, key, &n);
    if (!s) return 0;
    size_t e = 0; while (expect[e]) e++;
    if (e != n) return 0;
    for (size_t i = 0; i < n; i++) if (s[i] != expect[i]) return 0;
    return 1;
}
