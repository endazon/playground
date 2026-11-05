/* ネイティブ形の glue: ip_hal_* → vtable、ip_fw_* → fw_app_* */
#include "ip_hal.h"

static const ip_hal_vtable *vt = 0;

uint64_t ip_hal_micros(void) { return vt && vt->micros ? vt->micros(vt->ctx) : 0; }
int32_t  ip_hal_enc_read(uint32_t ch) { return vt && vt->enc_read ? vt->enc_read(vt->ctx, ch) : 0; }
void     ip_hal_pwm_write(uint32_t ch, float duty) { if (vt && vt->pwm_write) vt->pwm_write(vt->ctx, ch, duty); }
void     ip_hal_dac_write(uint32_t ch, float value) { if (vt && vt->dac_write) vt->dac_write(vt->ctx, ch, value); }
bool     ip_hal_gpio_read(uint32_t pin) { return vt && vt->gpio_read ? vt->gpio_read(vt->ctx, pin) : false; }
void     ip_hal_gpio_write(uint32_t pin, bool level) { if (vt && vt->gpio_write) vt->gpio_write(vt->ctx, pin, level); }
uint32_t ip_hal_uart_write(uint32_t ch, const uint8_t *data, uint32_t len) { return vt && vt->uart_write ? vt->uart_write(vt->ctx, ch, data, len) : 0; }
uint32_t ip_hal_uart_read(uint32_t ch, uint8_t *buf, uint32_t max) { return vt && vt->uart_read ? vt->uart_read(vt->ctx, ch, buf, max) : 0; }
void     ip_hal_log(ip_log_level level, const char *msg) { if (vt && vt->log) vt->log(vt->ctx, level, msg); }
void     ip_hal_trace(uint32_t id, double value) { if (vt && vt->trace) vt->trace(vt->ctx, id, value); }

IP_FW_EXPORT uint32_t ip_fw_abi_version(void) { return IP_FW_ABI_VERSION; }
IP_FW_EXPORT int32_t ip_fw_init(const ip_hal_vtable *hal, const char *cfg_json, size_t cfg_len) {
    if (!hal) return -1;
    vt = hal;
    return fw_app_init(cfg_json ? cfg_json : "", cfg_json ? cfg_len : 0);
}
IP_FW_EXPORT void ip_fw_tick(void) { fw_app_tick(); }
IP_FW_EXPORT void ip_fw_irq(uint32_t id) { fw_app_irq(id); }
IP_FW_EXPORT void ip_fw_idle(void) { fw_app_idle(); }
