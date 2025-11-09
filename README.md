# safeboy

This crate provides a safe Rust wrapper for [SameBoy], an accurate Game Boy
emulator written in C.

[SameBoy]: https://github.com/LIJI32/SameBoy

## What does it do?

- Provides a thread-safe abstraction of SameBoy's interface
  - Most thread safety is statically provided by Rust's type system and borrow
    checker.
- Provides an ergonomic callback API using Rust traits and dynamic dispatch
- Provides a number of comfort types to help you use the crate, like Rust
  wrappers over the C types

## Requirements

In order to use this crate, you need the following:

- `alloc` must be available (`std` is not required)
- `usize` must be at least 32 bits in width

Additionally, SameBoy must be able to compile for your target. Note that as it
is building just the emulator core, itself, most of its requirements do not
apply (e.g. SDL). You really just need a C compiler like GCC or Clang.

For Windows, you also do not need Visual Studio installed. You can use the
`i686-pc-windows-gnu` and `x86_64-pc-windows-gnu` targets to build this crate,
and cross-compiling should generally work.

### Boot ROMs

Since only the emulator core is being compiled, none of the SameBoy boot ROMs
will be provided. If you want those, you will need to compile those separately
using `rgbds`.
