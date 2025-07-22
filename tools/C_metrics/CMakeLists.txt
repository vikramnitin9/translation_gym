cmake_minimum_required(VERSION 3.10)
project(C_metrics)

find_package(LLVM REQUIRED CONFIG)
find_package(Clang REQUIRED CONFIG)
list(APPEND CMAKE_MODULE_PATH "${LLVM_CMAKE_DIR}")
include(LLVMConfig)

add_definitions(${LLVM_DEFINITIONS})
include_directories(${LLVM_INCLUDE_DIRS})
link_directories(${LLVM_LIBRARY_DIRS})
include_directories(${CLANG_INCLUDE_DIRS})

set(LLVM_ENABLE_PIC ON)
set(BUILD_SHARED_LIBS ON)
set(LLVM_LINK_COMPONENTS Support)

add_executable(C_metrics
  main_metrics.cpp
  MetricsVisitor.cpp
  MetricsVisitorConsumer.cpp
)

target_link_libraries(C_metrics
  PRIVATE
    clangAST
    clangBasic
    clangFrontend
    clangTooling
    ${LLVM_LIBRARIES}
)
