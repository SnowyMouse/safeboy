use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let sameboy: PathBuf = cmake::build(Path::new("SameBoy"));

    let core = PathBuf::from("SameBoy/Core")
        .canonicalize()
        .expect("should have a Core dir in SameBoy");

    println!("cargo:rustc-link-search={}", sameboy.display());
    println!("cargo:rustc-link-lib=sameboy");

    // If you encounter issues with stuff like time.h not being found, you might need
    // to use BINDGEN_EXTRA_CLANG_ARGS='--sysroot <path/to/sysroot>'
    //
    // For example, if crosscompiling with mingw on a Mac via homebrew, you'd do something like
    // BINDGEN_EXTRA_CLANG_ARGS='--sysroot "/opt/homebrew/opt/mingw-w64/toolchain-x86_64"' cargo build --target x86_64-pc-windows-gnu
    //
    // Annoying, unfortunately.
    //
    // See https://github.com/rust-lang/rust-bindgen/issues/1229
    let bindings = bindgen::Builder::default()
        .header(core.join("gb.h").to_str().unwrap())
        .allowlist_type("GB_.*")
        .allowlist_function("GB_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).canonicalize().expect("should have an out dir");
    let bindings_path = out.join("bindings.rs");
    bindings.write_to_file(bindings_path).expect("failed to write bindings");
}
