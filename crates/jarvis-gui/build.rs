fn main() {
    // Tauri build
    tauri_build::build();

    // Copy native DLLs to exe output dir (needed at runtime on Windows)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let lib_dir = std::path::Path::new(&manifest_dir).join("../../lib/windows/amd64");

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

    println!("cargo:rerun-if-changed=../../lib/windows/amd64");
}
