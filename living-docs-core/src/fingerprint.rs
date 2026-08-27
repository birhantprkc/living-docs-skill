//! The records-tree fingerprint the projection-staleness contract compares
//! against `sync_meta`.

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// SHA-256 over the sorted `(relative_path, content_sha256)` pairs of every
/// record `.md` file under `root`, excluding the reserved `index.md`/
/// `log.md` presentation files the read-model does not ingest — the same
/// file set `db_store::sync` walks. Deterministic: the same tree always
/// yields the same hex digest, any content or path change yields a
/// different one, and the result does not depend on directory walk order
/// (the pairs are sorted before hashing).
pub fn tree_fingerprint(root: &Path) -> io::Result<String> {
    let mut files = Vec::new();
    collect_record_files(root, &mut files);

    let mut pairs = Vec::with_capacity(files.len());
    for path in &files {
        let relative = relative_path(root, path);
        let content = fs::read(path)?;
        pairs.push(format!("{relative}:{}", hex_sha256(&content)));
    }
    pairs.sort();

    Ok(hex_sha256(pairs.join("\n").as_bytes()))
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn collect_record_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_record_files(&path, out);
        } else if is_record_file(&path) {
            out.push(path);
        }
    }
}

fn is_record_file(path: &Path) -> bool {
    let is_markdown = path.extension().and_then(|ext| ext.to_str()) == Some("md");
    is_markdown && !is_reserved(path)
}

/// Mirrors `db_store::record::is_reserved`: the two presentation-only
/// filenames excluded from the projection.
fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("index.md") | Some("log.md")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempTestDir {
        path: PathBuf,
    }

    impl TempTestDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("fingerprint-test-{label}-{nanos}"));
            fs::create_dir_all(&path).expect("create temp test dir");
            Self { path }
        }
    }

    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn seed(root: &Path) {
        fs::create_dir_all(root.join("adr")).expect("create adr dir");
        fs::write(root.join("adr").join("0001-a.md"), "a content").expect("write a");
        fs::write(root.join("adr").join("0002-b.md"), "b content").expect("write b");
        fs::write(root.join("index.md"), "reserved").expect("write reserved index");
    }

    #[test]
    fn tree_fingerprint_is_deterministic_on_the_same_tree() {
        let temp = TempTestDir::new("deterministic");
        seed(&temp.path);

        let first = tree_fingerprint(&temp.path).expect("fingerprint succeeds");
        let second = tree_fingerprint(&temp.path).expect("fingerprint succeeds");

        assert_eq!(first, second);
    }

    #[test]
    fn tree_fingerprint_changes_when_one_file_byte_changes() {
        let temp = TempTestDir::new("content-change");
        seed(&temp.path);
        let before = tree_fingerprint(&temp.path).expect("fingerprint succeeds");

        fs::write(temp.path.join("adr").join("0001-a.md"), "a content!").expect("edit a");
        let after = tree_fingerprint(&temp.path).expect("fingerprint succeeds");

        assert_ne!(before, after);
    }

    #[test]
    fn tree_fingerprint_is_independent_of_directory_walk_order() {
        let temp_a = TempTestDir::new("order-a");
        fs::create_dir_all(temp_a.path.join("adr")).expect("create adr dir");
        fs::write(temp_a.path.join("adr").join("0001-a.md"), "a content").expect("write a");
        fs::write(temp_a.path.join("adr").join("0002-b.md"), "b content").expect("write b");

        let temp_b = TempTestDir::new("order-b");
        fs::create_dir_all(temp_b.path.join("adr")).expect("create adr dir");
        fs::write(temp_b.path.join("adr").join("0002-b.md"), "b content").expect("write b");
        fs::write(temp_b.path.join("adr").join("0001-a.md"), "a content").expect("write a");

        assert_eq!(
            tree_fingerprint(&temp_a.path).expect("fingerprint succeeds"),
            tree_fingerprint(&temp_b.path).expect("fingerprint succeeds")
        );
    }

    #[test]
    fn tree_fingerprint_ignores_reserved_index_and_log_files() {
        let temp = TempTestDir::new("reserved");
        seed(&temp.path);
        let before = tree_fingerprint(&temp.path).expect("fingerprint succeeds");

        fs::write(temp.path.join("index.md"), "changed reserved content").expect("edit index");
        let after = tree_fingerprint(&temp.path).expect("fingerprint succeeds");

        assert_eq!(before, after);
    }

    #[test]
    fn tree_fingerprint_changes_when_a_path_is_renamed_with_identical_content() {
        let temp = TempTestDir::new("rename");
        seed(&temp.path);
        let before = tree_fingerprint(&temp.path).expect("fingerprint succeeds");

        fs::rename(
            temp.path.join("adr").join("0002-b.md"),
            temp.path.join("adr").join("0003-b.md"),
        )
        .expect("rename record");
        let after = tree_fingerprint(&temp.path).expect("fingerprint succeeds");

        assert_ne!(before, after);
    }

    #[test]
    fn tree_fingerprint_on_missing_root_hashes_an_empty_set() {
        let temp = TempTestDir::new("missing-root");
        let missing_root = temp.path.join("does-not-exist");
        let empty_root = temp.path.join("empty");
        fs::create_dir_all(&empty_root).expect("create empty dir");

        let missing_fingerprint =
            tree_fingerprint(&missing_root).expect("a missing root is a lenient empty tree");
        let empty_fingerprint = tree_fingerprint(&empty_root).expect("fingerprint succeeds");

        assert_eq!(missing_fingerprint, empty_fingerprint);
    }
}
