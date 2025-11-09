use std::path::{Path, PathBuf};
use regex::Regex;

const DEBUGGER_FILES: &[&str] = &[
    "debugger.c",
    "symbol_hash.c",
    "sm83_disassembler.c"
];

const CHEAT_SEARCH_FILES: &[&str] = &[
    "cheat_search.c"
];

fn main() {
    let disable_debugger = !cfg!(feature = "debugger");
    let disable_cheat_search = disable_debugger;

    let core_path = Path::new("SameBoy/Core");
    if !core_path.is_dir() {
        panic!("Missing Core directory in SameBoy; perhaps you forgot to initialize the submodule?")
    }

    let version_file_data = std::fs::read_to_string("SameBoy/version.mk")
        .expect("failed to read version.mk; it may be missing or SameBoy changed its build system");

    let version_regex = Regex::new(r#"VERSION := ([0-9]+\.[0-9]+\.[0-9]+)\b"#).expect("regex borked - this is a bug");
    let Some(n) = version_regex.captures(&version_file_data) else {
        panic!("version file does not match version regex {version_regex}")
    };
    let version_str = &n[1];
    let version_formatted = format!("\"{version_str}\"");

    let mut build_system = cc::Build::new();

    if std::env::var("CARGO_CFG_TARGET_OS") == Ok("windows".to_string()) {
        let windows_path = Path::new("windows-hacks");
        build_system.include(windows_path);

        for i in windows_path.read_dir().expect("failed to read windows-hacks dir") {
            let file = i.expect("error when iterating windows-hacks");
            let path = file.path();
            if path.extension() != Some("c".as_ref()) {
                continue;
            }
            build_system.file(path);
        }
    }

    for i in core_path.read_dir().expect("failed to read SameBoy/Core dir") {
        let file = i.expect("error when iterating SameBoy/Core");
        let path = file.path();
        if path.extension() != Some("c".as_ref()) {
            continue;
        }
        let file_name = path.file_name().expect("has an extension, so it should have a filename").to_str().expect("SameBoy's filenames should all be UTF-8");

        if disable_debugger && DEBUGGER_FILES.contains(&file_name) {
            continue;
        }
        if disable_cheat_search && CHEAT_SEARCH_FILES.contains(&file_name) {
            continue;
        }

        build_system.file(path);
    }

    println!("cargo:rustc-env=GB_VERSION={version_str}");

    if disable_debugger {
        build_system.define("GB_DISABLE_DEBUGGER", None);
    }

    if disable_cheat_search {
        build_system.define("GB_DISABLE_CHEAT_SEARCH", None);
    }

    build_system.define("GB_INTERNAL", None);
    build_system.define("GB_VERSION", version_formatted.as_str());
    build_system.warnings(false); // suppress warnings; we can't do anything about them
    build_system.compile("sameboy");

    // If you encounter issues with stuff like time.h not being found, you might need
    // to use BINDGEN_EXTRA_CLANG_ARGS='--sysroot <path/to/sysroot>'
    //
    // For example, if crosscompiling with mingw on a Mac via homebrew, you'd do something like
    // BINDGEN_EXTRA_CLANG_ARGS='--sysroot "/opt/homebrew/opt/mingw-w64/toolchain-x86_64"' cargo build --target x86_64-pc-windows-gnu
    //
    // Annoying, unfortunately.
    //
    // See https://github.com/rust-lang/rust-bindgen/issues/1229
    let mut builder = bindgen::Builder::default();

    if disable_debugger {
        builder = builder.header_contents("disable_debugger_config.h", "#define GB_DISABLE_DEBUGGER");
    }
    if disable_cheat_search {
        builder = builder.header_contents("disable_cheat_search_config.h", "#define GB_DISABLE_CHEAT_SEARCH");
    }

    let bindings = builder
        .header(core_path.join("gb.h").to_str().unwrap())
        .allowlist_type("GB_.*")
        .allowlist_function("GB_.*")
        .use_core()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate bindings");

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).canonicalize().expect("should have an out dir");
    let bindings_path = out.join("bindings.rs");
    bindings.write_to_file(bindings_path).expect("failed to write bindings");
}
