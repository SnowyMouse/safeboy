#!/bin/bash

bindgen \
  --allowlist-type "GB_.*" \
  --allowlist-function "GB_.*" \
  --use-core \
  --rust-edition "2024" \
  --rust-target "1.92.0" \
  --wrap-unsafe-ops \
  ./SameBoy/Core/gb.h > src/bindings_pregenerated.rs
