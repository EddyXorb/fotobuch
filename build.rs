
fn main() {
    #[cfg(windows)]
    {
        let libclang_path = env::var("LIBCLANG_PATH")
            .ok()
            .or_else(|| find_libclang().map(|p| p.to_string_lossy().to_string()));

        if let Some(path) = libclang_path {
            // Setze für aktuelles Projekt
            println!("cargo:rustc-env=LIBCLANG_PATH={}", path);

            // Aktualisiere .cargo/config.toml für zukünftige Builds
            ensure_cargo_config(&path);
        }
    }
}

#[cfg(windows)]
fn find_libclang() -> Option<PathBuf> {
    // Typische Installationspfade für LLVM/Clang auf Windows
    let candidates = [
        "C:\\Program Files\\LLVM\\bin",
        "C:\\Program Files (x86)\\LLVM\\bin",
        "C:\\Program Files\\Tools\\LLVM\\bin",
        "C:\\llvm\\bin",
    ];

    for candidate in &candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            let dll_path = path.join("libclang.dll");
            if dll_path.exists() {
                return Some(path);
            }
        }
    }

    // Versuche clang.exe im PATH zu finden
    if let Ok(output) = std::process::Command::new("where")
        .arg("clang.exe")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let path = PathBuf::from(&path_str);
            if let Some(parent) = path.parent() {
                if parent.join("libclang.dll").exists() {
                    return Some(parent.to_path_buf());
                }
            }
        }
    }

    None
}

#[cfg(windows)]
fn ensure_cargo_config(libclang_path: &str) {
    let cargo_dir = Path::new(".cargo");
    let config_path = cargo_dir.join("config.toml");

    // Erstelle .cargo Verzeichnis wenn nicht vorhanden
    let _ = fs::create_dir(cargo_dir);

    // Schreibe oder aktualisiere config.toml
    let config_content = format!(
        "[build]\nrustflags = []\n\n[env]\nLIBCLANG_PATH = \"{}\"\n",
        libclang_path.replace('\\', "\\\\")
    );

    let _ = fs::write(&config_path, config_content);
}
