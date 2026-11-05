/*
 * ip_hal.h — ファームウェアから見た HAL
 *
 * 同じファームウェアソースを 2 形にビルドするための境界。
 *   IP_HAL_TARGET_WASM   : wasm32-wasip2。関数は wit-bindgen が生成した ip:emu/hal の import へ委譲
 *   IP_HAL_TARGET_NATIVE : .dll / .so / .dylib。ip_fw_init で受け取った関数テーブルへ委譲
 * ファームウェアはこのヘッダだけを include し、どちらの形でも同じコードで動く。
 * 対応する WIT は abi/wit/firmware.wit。関数名・引数・意味は 1 対 1。
 */
#ifndef IP_HAL_H
#define IP_HAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define IP_FW_ABI_VERSION 1u

typedef enum ip_log_level {
    IP_LOG_TRACE = 0, IP_LOG_DEBUG, IP_LOG_INFO, IP_LOG_WARN, IP_LOG_ERROR
} ip_log_level;

/* ---- ファームウェアが実装するエントリポイント (両形で同名) ---- */
#if defined(IP_HAL_TARGET_NATIVE)
#  if defined(_WIN32)
#    define IP_FW_EXPORT __declspec(dllexport)
#  else
#    define IP_FW_EXPORT __attribute__((visibility("default")))
#  endif
#else
#  define IP_FW_EXPORT /* wasm 形は wit-bindgen の生成コードが export する */
#endif

/* ---- HAL 関数 (ファームウェアが呼ぶ側) ---- */
uint64_t ip_hal_micros(void);                          /* 仮想時刻 [µs]。壁時計ではない */
int32_t  ip_hal_enc_read(uint32_t ch);                 /* 累積カウント (ラップあり) */
void     ip_hal_pwm_write(uint32_t ch, float duty);    /* -1.0..1.0、範囲外は飽和 */
void     ip_hal_dac_write(uint32_t ch, float value);
bool     ip_hal_gpio_read(uint32_t pin);
void     ip_hal_gpio_write(uint32_t pin, bool level);
uint32_t ip_hal_uart_write(uint32_t ch, const uint8_t *data, uint32_t len); /* 受理したバイト数 */
uint32_t ip_hal_uart_read(uint32_t ch, uint8_t *buf, uint32_t max);         /* 読めたバイト数 */
void     ip_hal_log(ip_log_level level, const char *msg);
void     ip_hal_trace(uint32_t id, double value);

/* ---- ネイティブ形: エンジンが渡す関数テーブル ---- */
#if defined(IP_HAL_TARGET_NATIVE)
typedef struct ip_hal_vtable {
    uint32_t size;                                     /* sizeof(ip_hal_vtable)。前方互換の判定に使う */
    uint64_t (*micros)(void *ctx);
    int32_t  (*enc_read)(void *ctx, uint32_t ch);
    void     (*pwm_write)(void *ctx, uint32_t ch, float duty);
    void     (*dac_write)(void *ctx, uint32_t ch, float value);
    bool     (*gpio_read)(void *ctx, uint32_t pin);
    void     (*gpio_write)(void *ctx, uint32_t pin, bool level);
    uint32_t (*uart_write)(void *ctx, uint32_t ch, const uint8_t *data, uint32_t len);
    uint32_t (*uart_read)(void *ctx, uint32_t ch, uint8_t *buf, uint32_t max);
    void     (*log)(void *ctx, ip_log_level level, const char *msg);
    void     (*trace)(void *ctx, uint32_t id, double value);
    void *ctx;                                         /* エンジン側のコンテキスト。ファームウェアは触らない */
} ip_hal_vtable;

/*
 * ネイティブ形のエントリポイント。エンジンは libloading でこれらを引く。
 *   ip_fw_abi_version : IP_FW_ABI_VERSION を返す
 *   ip_fw_init        : HAL テーブルを受け取り、cfg_json (manifest の firmware.params) で初期化。0 以外は失敗
 *   ip_fw_tick / irq / idle : wasm 形の fw.tick / fw.irq / fw.idle と同義
 * これらは glue (ip_hal_native.c) が実装し、ファームウェア本体の fw_app_* へ委譲する。
 */
IP_FW_EXPORT uint32_t ip_fw_abi_version(void);
IP_FW_EXPORT int32_t  ip_fw_init(const ip_hal_vtable *hal, const char *cfg_json, size_t cfg_len);
IP_FW_EXPORT void     ip_fw_tick(void);
IP_FW_EXPORT void     ip_fw_irq(uint32_t id);
IP_FW_EXPORT void     ip_fw_idle(void);
#endif /* IP_HAL_TARGET_NATIVE */

/* ---- ファームウェア本体が実装する関数 (両形で共通。glue がここへ委譲する) ---- */
int32_t fw_app_init(const char *cfg_json, size_t cfg_len);   /* 0 で成功 */
void    fw_app_tick(void);
void    fw_app_irq(uint32_t id);
void    fw_app_idle(void);

/*
 * wasm 形では wit-bindgen c が生成する exports_ip_emu_fw_* を fw_app_* に橋渡しする glue (ip_hal_wasm.c)、
 * ネイティブ形では ip_fw_* を fw_app_* に橋渡しする glue (ip_hal_native.c) をビルドに含める。
 * ファームウェア本体はどちらの形でも fw_app_* を定義するだけでよい。
 */

#ifdef __cplusplus
}
#endif
#endif /* IP_HAL_H */
