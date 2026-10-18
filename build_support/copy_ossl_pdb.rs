// Vendored OpenSSL (MSVC) records its debug PDB as the bare name `ossl_static.pdb`, so
// link.exe looks for it in target/<profile>/deps and prints LNK4099 for every object.
// Copy it there. DEP_OPENSSL_ROOT is only set when openssl-sys is a direct dependency.
fn copy_ossl_pdb() {
    use std::path::Path;
    let Some(root) = std::env::var_os("DEP_OPENSSL_ROOT") else { return };
    let src = Path::new(&root).join("lib").join("ossl_static.pdb");
    println!("cargo:rerun-if-changed={}", src.display());
    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out
    let Some(profile_dir) = std::env::var_os("OUT_DIR")
        .and_then(|o| Path::new(&o).ancestors().nth(3).map(Path::to_path_buf))
    else {
        return;
    };
    if src.exists() {
        let _ = std::fs::copy(&src, profile_dir.join("deps").join("ossl_static.pdb"));
    }
}
