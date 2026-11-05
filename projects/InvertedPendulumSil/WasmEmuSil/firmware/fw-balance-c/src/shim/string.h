/* freestanding wasm ビルド用の最小 string.h */
#ifndef IP_SHIM_STRING_H
#define IP_SHIM_STRING_H
#include <stddef.h>
void  *memcpy(void *dst, const void *src, size_t n);
void  *memmove(void *dst, const void *src, size_t n);
void  *memset(void *dst, int c, size_t n);
int    memcmp(const void *a, const void *b, size_t n);
size_t strlen(const char *s);
#endif
