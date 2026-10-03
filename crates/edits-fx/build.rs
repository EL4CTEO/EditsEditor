//! Embeds every built-in effect (`effects/**/*.wgsl`) and preset (`presets/**/*.rhai`).
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, ext, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(p);
        }
    }
}

fn emit(name: &str, files: &[PathBuf], root: &Path) -> String {
    let mut s = format!("pub static {name}: &[(&str, &str)] = &[\n");
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        s.push_str(&format!("    ({rel:?}, include_str!({:?})),\n", f.canonicalize().unwrap()));
    }
    s.push_str("];\n");
    s
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut effects = vec![];
    let mut presets = vec![];
    collect(&root.join("effects"), "wgsl", &mut effects);
    collect(&root.join("presets"), "rhai", &mut presets);
    effects.sort();
    presets.sort();
    let code = emit("BUILTIN_EFFECTS", &effects, &root) + &emit("BUILTIN_PRESETS", &presets, &root);
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("builtin.rs"), code).unwrap();
    println!("cargo:rerun-if-changed=effects");
    println!("cargo:rerun-if-changed=presets");
}
