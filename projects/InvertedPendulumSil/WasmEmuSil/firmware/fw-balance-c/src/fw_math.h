/* 数学関数の宣言。native は <math.h>、wasm は ip-cmath (Rust libm) が同名の C ABI を提供する */
#ifndef FW_MATH_H
#define FW_MATH_H
#if defined(IP_HAL_TARGET_WASM)
double sin(double); double cos(double); double tanh(double); double exp(double);
double sqrt(double); double round(double); double fabs(double); double fmod(double, double);
#else
#include <math.h>
#endif
#endif
