#![cfg(feature = "geometry")]

use std::fs;
use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "HashMap",
    "HashSet",
    "rand::",
    "rand_pcg",
    "rand_core",
    "thread::",
    "rayon",
    ".sin(",
    ".cos(",
    ".tan(",
    ".sin_cos(",
    ".asin(",
    ".acos(",
    ".atan(",
    ".atan2(",
    ".sinh(",
    ".cosh(",
    ".tanh(",
    ".exp(",
    ".exp2(",
    ".exp_m1(",
    ".ln(",
    ".ln_1p(",
    ".log",
    ".powf(",
    ".powi(",
    ".hypot(",
    ".cbrt(",
    "mul_add",
    "Instant",
    "SystemTime",
    "unsafe",
];

fn visit(dir: &Path, files: &mut usize) {
    for entry in fs::read_dir(dir).expect("read the geometry directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            visit(&path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            *files += 1;
            let text = fs::read_to_string(&path).expect("read a source file");
            for (number, line) in text.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for word in FORBIDDEN {
                    assert!(
                        !line.contains(word),
                        "{}:{} uses `{}`",
                        path.display(),
                        number + 1,
                        word
                    );
                }
            }
        }
    }
}

#[test]
fn geometry_module_uses_only_portable_arithmetic() {
    let mut files = 0;
    visit(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/geometry")),
        &mut files,
    );
    assert!(files > 0, "no source file found under src/geometry");
}
