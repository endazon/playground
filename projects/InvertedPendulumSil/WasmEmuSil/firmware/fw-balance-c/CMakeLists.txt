# C 版ファームウェアのネイティブ形 (デバッグ用共有ライブラリ) を CMake でビルドする。
# Visual Studio は「フォルダーを開く」でこのディレクトリを開けば、launch.vs.json の構成で F5 できる。
# wasm 形は build.sh (clang + wasm-tools) で作る。
cmake_minimum_required(VERSION 3.20)
project(fw_balance_c C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

add_library(fw_balance SHARED
  src/fw_balance.c
  src/cfg.c
  src/ip_hal_native.c)
target_include_directories(fw_balance PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../../abi ${CMAKE_CURRENT_SOURCE_DIR}/src)
target_compile_definitions(fw_balance PRIVATE IP_HAL_TARGET_NATIVE)
set_target_properties(fw_balance PROPERTIES OUTPUT_NAME "fw-balance-c" PREFIX "")
if(NOT WIN32)
  target_link_libraries(fw_balance PRIVATE m)
endif()
# デバッグ情報を常に出す (ブレークポイントのため)
target_compile_options(fw_balance PRIVATE $<$<C_COMPILER_ID:MSVC>:/Zi /W4> $<$<NOT:$<C_COMPILER_ID:MSVC>>:-g -Wall -Wextra>)
