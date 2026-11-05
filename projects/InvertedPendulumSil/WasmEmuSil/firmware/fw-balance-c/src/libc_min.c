/*
 * freestanding wasm 用の極小 libc。
 * ファームウェアは起動時 (init) と稀なログでしかヒープを使わないので、解放しないバンプアロケータで足りる。
 * -fno-builtin でコンパイルすること (memcpy のループが memcpy 呼び出しに置換されて再帰するのを防ぐ)。
 */
#include <stddef.h>
#include <stdint.h>

#define HEAP_SIZE (256 * 1024)
static uint8_t heap[HEAP_SIZE] __attribute__((aligned(16)));
static size_t heap_top = 0;

static size_t align_up(size_t v, size_t a) { return (v + a - 1) & ~(a - 1); }

void *malloc(size_t size) {
    size_t start = align_up(heap_top, 16);
    if (size > HEAP_SIZE - start) return 0;
    heap_top = start + size;
    return heap + start;
}

void *memcpy(void *dst, const void *src, size_t n) {
    uint8_t *d = dst; const uint8_t *s = src;
    for (size_t i = 0; i < n; i++) d[i] = s[i];
    return dst;
}

void *memmove(void *dst, const void *src, size_t n) {
    uint8_t *d = dst; const uint8_t *s = src;
    if (d < s) { for (size_t i = 0; i < n; i++) d[i] = s[i]; }
    else { for (size_t i = n; i > 0; i--) d[i - 1] = s[i - 1]; }
    return dst;
}

void *memset(void *dst, int c, size_t n) {
    uint8_t *d = dst;
    for (size_t i = 0; i < n; i++) d[i] = (uint8_t)c;
    return dst;
}

int memcmp(const void *a, const void *b, size_t n) {
    const uint8_t *x = a, *y = b;
    for (size_t i = 0; i < n; i++) if (x[i] != y[i]) return x[i] - y[i];
    return 0;
}

size_t strlen(const char *s) { size_t n = 0; while (s[n]) n++; return n; }

void *realloc(void *ptr, size_t size) {
    /* 旧サイズが分からないので新領域へ size ぶんコピーする (読み過ぎはヒープ内なので安全) */
    void *p = malloc(size);
    if (p && ptr) memcpy(p, ptr, size);
    return p;
}

void free(void *ptr) { (void)ptr; }

__attribute__((noreturn)) void abort(void) { __builtin_trap(); }
