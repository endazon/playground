/* wasm 形の glue: ip_hal_* → wit-bindgen の import、exports_ip_emu_fw_* → fw_app_* */
#include "ip_hal.h"
#include "firmware.h"
#include <string.h>

uint64_t ip_hal_micros(void) { return ip_emu_hal_micros(); }
int32_t  ip_hal_enc_read(uint32_t ch) { return ip_emu_hal_enc_read(ch); }
void     ip_hal_pwm_write(uint32_t ch, float duty) { ip_emu_hal_pwm_write(ch, duty); }
void     ip_hal_dac_write(uint32_t ch, float value) { ip_emu_hal_dac_write(ch, value); }
bool     ip_hal_gpio_read(uint32_t pin) { return ip_emu_hal_gpio_read(pin); }
void     ip_hal_gpio_write(uint32_t pin, bool level) { ip_emu_hal_gpio_write(pin, level); }

uint32_t ip_hal_uart_write(uint32_t ch, const uint8_t *data, uint32_t len) {
    firmware_list_u8_t l = { (uint8_t *)data, len };
    return ip_emu_hal_uart_write(ch, &l);
}
uint32_t ip_hal_uart_read(uint32_t ch, uint8_t *buf, uint32_t max) {
    firmware_list_u8_t ret;
    ip_emu_hal_uart_read(ch, max, &ret);
    uint32_t n = ret.len < max ? (uint32_t)ret.len : max;
    memcpy(buf, ret.ptr, n);
    firmware_list_u8_free(&ret);
    return n;
}
void ip_hal_log(ip_log_level level, const char *msg) {
    firmware_string_t s = { (uint8_t *)msg, strlen(msg) };
    ip_emu_hal_log((ip_emu_hal_log_level_t)level, &s);
}
void ip_hal_trace(uint32_t id, double value) { ip_emu_hal_trace(id, value); }

uint32_t exports_ip_emu_fw_abi_version(void) { return IP_FW_ABI_VERSION; }
bool exports_ip_emu_fw_init(firmware_string_t *cfg_json, firmware_string_t *err) {
    int32_t rc = fw_app_init((const char *)cfg_json->ptr, cfg_json->len);
    if (rc != 0) { firmware_string_dup(err, "fw_app_init failed"); return false; }
    return true;
}
void exports_ip_emu_fw_tick(void) { fw_app_tick(); }
void exports_ip_emu_fw_irq(uint32_t id) { fw_app_irq(id); }
void exports_ip_emu_fw_idle(void) { fw_app_idle(); }
