//! Generates `embedded_files.rs` listing every content file for the
//! `embedded` feature.
use std::path::{Path, PathBuf};

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(root, &p, out);
        } else if matches!(
            p.extension().and_then(|x| x.to_str()),
            Some("jsonc" | "ftl")
        ) {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, p));
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest.join("../../content");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    let mut code = String::from("pub static EMBEDDED_FILES: &[(&str, &str)] = &[\n");
    for (rel, path) in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let abs = path.canonicalize().unwrap_or(path.clone());
        code.push_str(&format!(
            "    ({rel:?}, include_str!({:?})),\n",
            abs.to_string_lossy()
        ));
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()).join("embedded_files.rs");
    let _ = std::fs::write(out, code);
}
