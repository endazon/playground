/* freestanding wasm ビルド用の最小 stdlib.h (wit-bindgen 生成コードが参照する分だけ) */
#ifndef IP_SHIM_STDLIB_H
#define IP_SHIM_STDLIB_H
#include <stddef.h>
void *malloc(size_t size);
void *realloc(void *ptr, size_t size);
void  free(void *ptr);
void  abort(void) __attribute__((noreturn));
#endif
