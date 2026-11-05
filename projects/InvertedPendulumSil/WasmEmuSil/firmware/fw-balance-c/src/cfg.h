#ifndef FW_CFG_H
#define FW_CFG_H
#include <stddef.h>
/* フラットな JSON から値を引く極小パーサ (Rust 版 cfg.rs と同じ規則) */
double      cfg_number(const char *json, size_t len, const char *key, double def);
/* 文字列値の先頭ポインタと長さを返す。無ければ 0 */
const char *cfg_string(const char *json, size_t len, const char *key, size_t *out_len);
int         cfg_string_eq(const char *json, size_t len, const char *key, const char *expect);
#endif
