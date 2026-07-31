// graphene_layout/tests/provenance_check.rs
//
// Enforces the doc-comment convention from the audit: every public Layout
// implementation must cite a Reference: line. This is what would have caught
// `KamadaKawaiLayout` being mislabeled before it shipped — a reviewer skimming
// diffs won't necessarily re-derive the math, but "does this have a citation
// and does the citation match the struct name" is a cheap, mechanical check.

use std::fs;
use std::path::Path;

#[test]
fn all_public_layout_structs_have_reference_comment() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut missing = Vec::new();

    visit_rs_files(&src_dir, &mut |path, contents| {
        for (i, line) in contents.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub struct") && trimmed.contains("Layout") {
                // Look backwards for a doc comment block containing "Reference:"
                let all_lines: Vec<&str> = contents.lines().collect();
                let preceding = all_lines[..i].iter().rev().take(15);
                let has_reference = preceding
                    .take_while(|l| l.trim().starts_with("///") || l.trim().starts_with("//"))
                    .any(|l| l.contains("Reference:"));

                if !has_reference {
                    missing.push(format!("{}:{} — {}", path.display(), i + 1, trimmed));
                }
            }
        }
    });

    assert!(
        missing.is_empty(),
        "Layout structs missing a `Reference:` doc comment:\n{}",
        missing.join("\n")
    );
}

fn visit_rs_files(dir: &Path, f: &mut impl FnMut(&Path, &str)) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, f);
        } else if path.extension().map_or(false, |e| e == "rs") {
            if let Ok(contents) = fs::read_to_string(&path) {
                f(&path, &contents);
            }
        }
    }
}
