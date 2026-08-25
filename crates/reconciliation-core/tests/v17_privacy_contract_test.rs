use std::fs;
use std::path::PathBuf;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("repository root")
        .to_path_buf()
}

#[test]
fn offline_dependency_and_confidential_file_guards_remain_present() {
    let root = repository_root();
    let package = fs::read_to_string(root.join("package.json")).expect("package.json");
    let tauri = fs::read_to_string(root.join("src-tauri/Cargo.toml")).expect("Tauri manifest");
    let core = fs::read_to_string(root.join("crates/reconciliation-core/Cargo.toml"))
        .expect("core manifest");
    let manifests = format!("{package}\n{tauri}\n{core}").to_ascii_lowercase();
    for forbidden in [
        "sentry",
        "segment",
        "google-analytics",
        "reqwest",
        "ureq",
        "openai",
        "anthropic",
    ] {
        assert!(
            !manifests.contains(forbidden),
            "offline product must not depend on {forbidden}"
        );
    }

    let gitignore = fs::read_to_string(root.join(".gitignore")).expect(".gitignore");
    for protected in [
        "node_modules",
        "target",
        ".local-testdata",
        "*.xlsx",
        "*.xls",
    ] {
        assert!(
            gitignore.contains(protected),
            "missing confidential/generated ignore guard: {protected}"
        );
    }
}
