fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // Embed Windows application manifest (enables Common Controls v6 → TaskDialogIndirect)
    // Only meaningful on Windows MSVC targets
    #[cfg(target_os = "windows")]
    {
        let manifest_path = std::path::Path::new(&manifest_dir).join("app.manifest");
        if manifest_path.exists() {
            // MSVC linker: embed manifest as resource ID 1
            println!(
                "cargo:rustc-link-arg=/MANIFEST:EMBED"
            );
            println!(
                "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
                manifest_path.display()
            );
            println!("cargo:rerun-if-changed=app.manifest");
        } else {
            eprintln!("cargo:warning=app.manifest not found at {}", manifest_path.display());
        }
    }

    let lib_dir = std::path::Path::new(&manifest_dir).join("../../lib/windows/amd64");

    // Tell linker where to find libvosk.lib
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Copy all DLLs to the exe output directory so Windows can find them at runtime.
    // OUT_DIR is target/{profile}/build/crate-hash/out — exe is 3 levels up.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let exe_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .expect("could not resolve exe output directory");

    if lib_dir.exists() {
        for entry in std::fs::read_dir(&lib_dir).expect("cannot read lib dir") {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("dll") {
                let dest = exe_dir.join(path.file_name().unwrap());
                std::fs::copy(&path, &dest).unwrap_or_else(|e| {
                    eprintln!("cargo:warning=Failed to copy {:?}: {}", path, e);
                    0
                });
            }
        }
    }

    // Re-run if the lib directory contents change
    println!("cargo:rerun-if-changed=../../lib/windows/amd64");
}
