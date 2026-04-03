use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let commands_dir = Path::new(&manifest_dir).join("commands");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest = Path::new(&out_dir).join("toml_includes.rs");

    let mut entries = Vec::new();
    collect_toml_files(&commands_dir, &commands_dir, &mut entries);
    entries.sort();

    let mut code = String::from("{\n    let mut all = Vec::new();\n");
    for rel_path in &entries {
        let full = commands_dir.join(rel_path);
        let path_str = full.to_str().expect("non-UTF-8 path").replace('\\', "/");
        code.push_str(&format!(
            "    all.extend(load_toml(include_str!(\"{path_str}\")));\n",
        ));
    }
    code.push_str("    build_registry(all)\n}");

    fs::write(&dest, code).expect("failed to write toml_includes.rs");

    println!("cargo:rerun-if-changed=commands");
    for entry in &entries {
        println!(
            "cargo:rerun-if-changed=commands/{}",
            entry.display()
        );
    }
}

fn collect_toml_files(
    base: &Path,
    dir: &Path,
    out: &mut Vec<std::path::PathBuf>,
) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_toml_files(base, &path, out);
        } else if path.extension().is_some_and(|e| e == "toml") {
            let name = path
                .file_name()
                .expect("file has no name")
                .to_str()
                .expect("non-UTF-8 filename");
            if name != "SAMPLE.toml"
                && let Ok(rel) = path.strip_prefix(base)
            {
                out.push(rel.to_path_buf());
            }
        }
    }
}
