use std::path::PathBuf;

pub fn get_test_doc_path() -> PathBuf {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    PathBuf::from(root).join("data/small_test_doc.pdf")
}
