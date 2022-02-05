# CMakeList.txt : Training の CMake プロジェクト。ソースを含めて、次を定義します:
# プロジェクト専用ロジックはこちらです。
#
cmake_minimum_required (VERSION 3.8)

include(${PROJECT_SOURCE_DIR}/cmake/DownloadProject/DownloadProject.cmake)
download_project(PROJ googletest
                GIT_REPOSITORY https://github.com/google/googletest.git
                GIT_TAG main
                UPDATE_DISCONNECTED 1
)
add_subdirectory(${googletest_SOURCE_DIR} ${googletest_BINARY_DIR})
message(STATUS "SOURCE_DIR = ${googletest_SOURCE_DIR}")
message(STATUS "BINARY_DIR = ${googletest_BINARY_DIR}")

add_executable(UnitTest UnitTest.cpp)
target_compile_features(UnitTest PUBLIC cxx_std_20)
target_link_libraries(UnitTest gtest_main)

include(GoogleTest)
gtest_add_tests(TARGET UnitTest)
