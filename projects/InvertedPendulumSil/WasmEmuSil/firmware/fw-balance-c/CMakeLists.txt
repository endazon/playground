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

# 出力先を <buildRoot>/bin に固定する。ジェネレータで場所が変わると launch.vs.json の
# --firmware のパスが追従できない: Visual Studio の「フォルダーを開く」は既定で Ninja
# (単一構成 → <buildRoot>/) を使うが、-G "Visual Studio NN" は多構成 (<buildRoot>/Debug/)。
# 構成ごとの変数も明示しないと、多構成ジェネレータは末尾に構成名を足してしまう。
set_target_properties(fw_balance PROPERTIES
  RUNTIME_OUTPUT_DIRECTORY         "${CMAKE_BINARY_DIR}/bin"   # Windows の .dll
  RUNTIME_OUTPUT_DIRECTORY_DEBUG   "${CMAKE_BINARY_DIR}/bin"
  RUNTIME_OUTPUT_DIRECTORY_RELEASE "${CMAKE_BINARY_DIR}/bin"
  LIBRARY_OUTPUT_DIRECTORY         "${CMAKE_BINARY_DIR}/bin"   # Linux / macOS の .so / .dylib
  LIBRARY_OUTPUT_DIRECTORY_DEBUG   "${CMAKE_BINARY_DIR}/bin"
  LIBRARY_OUTPUT_DIRECTORY_RELEASE "${CMAKE_BINARY_DIR}/bin"
  ARCHIVE_OUTPUT_DIRECTORY         "${CMAKE_BINARY_DIR}/bin"   # Windows の .lib
  ARCHIVE_OUTPUT_DIRECTORY_DEBUG   "${CMAKE_BINARY_DIR}/bin"
  ARCHIVE_OUTPUT_DIRECTORY_RELEASE "${CMAKE_BINARY_DIR}/bin")
if(NOT WIN32)
  target_link_libraries(fw_balance PRIVATE m)
endif()
# デバッグ情報を常に出す (ブレークポイントのため)。
# MSVC には /utf-8 が要る: ソースは UTF-8 だが、既定では実行環境のコードページ (日本語なら CP932)
# として読もうとして C4819 を山ほど出す
target_compile_options(fw_balance PRIVATE $<$<C_COMPILER_ID:MSVC>:/Zi /W4 /utf-8> $<$<NOT:$<C_COMPILER_ID:MSVC>>:-g -Wall -Wextra>)
