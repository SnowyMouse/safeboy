cmake_minimum_required(VERSION 3.9)

project(SameBoy
    LANGUAGES C
)

add_library(sameboy STATIC
    Core/apu.c
    Core/camera.c
    Core/cheats.c
    Core/debugger.c
    Core/display.c
    Core/gb.c
    Core/joypad.c
    Core/mbc.c
    Core/memory.c
    Core/printer.c
    Core/random.c
    Core/rewind.c
    Core/rumble.c
    Core/save_state.c
    Core/sgb.c
    Core/sm83_cpu.c
    Core/sm83_disassembler.c
    Core/symbol_hash.c
    Core/timing.c
    Core/workboy.c
)

file(READ "version.mk" GB_VERSION_FILE)
string(REPLACE "VERSION := " "" GB_VERSION_DATA "${GB_VERSION_FILE}")
target_compile_definitions(sameboy PUBLIC GB_INTERNAL GB_VERSION="${GB_VERSION_DATA}")

install(TARGETS sameboy DESTINATION ".")
