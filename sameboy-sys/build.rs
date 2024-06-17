use std::env;
use std::path::PathBuf;

fn main() {
    let mut version = include_str!("SameBoy/version.mk").split("VERSION := ");
    version.next();
    let version = version.next().expect("should be able to extract version");

    let core = PathBuf::from("SameBoy/Core")
        .canonicalize()
        .expect("should have a Core dir in SameBoy");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).canonicalize().expect("should have an out dir");
    let archive = out.join("libsameboy.a");

    let mut entries = Vec::new();
    for i in std::fs::read_dir(&core).expect("can't iterate core") {
        let entry = i.expect("can't iterate core entry");
        let path = entry.path();
        if path.extension() == Some("c".as_ref()) {
            let mut object_path = path.clone();
            object_path.set_extension("o");

            let object_path = out.join(object_path.file_name().unwrap());
            let command = std::process::Command::new("clang")
                .arg("-c")
                .arg("-o")
                .arg(&object_path)
                .arg("-O3")
                .arg("-DGB_INTERNAL")
                .arg(format!("-DGB_VERSION=\"{version}\""))
                .arg(&path)
                .output()
                .expect("could not spawn `clang`");

            if !command.status.success() {
                eprintln!("Failed to compile SameBoy ({path:?} failed)");
                eprintln!("stdout: {}\n", std::str::from_utf8(&command.stdout).unwrap());
                eprintln!("stderr: {}\n", std::str::from_utf8(&command.stderr).unwrap());
                panic!();
            }

            entries.push(object_path);
        }
    }

    let command = std::process::Command::new("ar")
        .arg("rcs")
        .arg(&archive)
        .args(&entries)
        .output()
        .expect("could not spawn `ar`");

    if !command.status.success() {
        eprintln!("Failed to compile SameBoy (archive failed)");
        eprintln!("stdout: {}\n", std::str::from_utf8(&command.stdout).unwrap());
        eprintln!("stderr: {}\n", std::str::from_utf8(&command.stderr).unwrap());
        panic!();
    }

    println!("cargo:rustc-link-search={}", out.to_str().unwrap());
    println!("cargo:rustc-link-lib=sameboy");

    let bindings = bindgen::Builder::default()
        .header(core.join("gb.h").to_str().unwrap())
        .allowlist_type("GB_.*")
        .allowlist_function("GB_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate bindings");

    let bindings_path = out.join("bindings.rs");
    bindings.write_to_file(bindings_path).expect("failed to write bindings");
}