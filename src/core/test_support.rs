use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::core::util::timestamp_nanos;

static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn canonical_path(path: &Path) -> PathBuf {
    crate::core::paths::normalize_windows_path(&fs::canonicalize(path).unwrap())
}

pub fn directory_symlink(source: &Path, link: &Path) {
    crate::core::symlink::default_provider()
        .create_symlink(source, link, crate::core::symlink::LinkKind::Directory)
        .unwrap();
}

pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(label: &str) -> Self {
        for _ in 0..20 {
            let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "agent-linker-{label}-{}-{}-{}",
                std::process::id(),
                timestamp_nanos(),
                counter
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("failed to create test directory {}: {err}", path.display()),
            }
        }
        panic!("failed to create unique test directory for `{label}` after multiple attempts");
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_skips_occupied_and_cleans_only_owned_dir() {
        let base = std::env::temp_dir();
        let occupied_name = format!("agent-linker-occupied-test-{}", timestamp_nanos());
        let occupied_path = base.join(&occupied_name);
        fs::create_dir(&occupied_path).unwrap();
        let sentinel = occupied_path.join("sentinel.txt");
        fs::write(&sentinel, "preserve").unwrap();

        // Simulate TestDir creation loop skipping occupied path
        let mut tried = false;
        let mut created_path = None;
        for _ in 0..10 {
            let path = if !tried {
                tried = true;
                occupied_path.clone()
            } else {
                base.join(format!(
                    "agent-linker-occupied-test-{}-unique",
                    timestamp_nanos()
                ))
            };
            match fs::create_dir(&path) {
                Ok(()) => {
                    created_path = Some(TestDir { path });
                    break;
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("{err}"),
            }
        }

        let dir = created_path.expect("must create unique test dir");
        assert!(dir.path().is_dir());
        assert!(sentinel.is_file());
        drop(dir);

        // Sentinel must still be intact
        assert!(sentinel.is_file());
        assert_eq!(fs::read_to_string(&sentinel).unwrap(), "preserve");
        let _ = fs::remove_dir_all(&occupied_path);
    }

    #[test]
    fn test_dir_concurrent_creation_does_not_collide() {
        use std::thread;

        let mut handles = Vec::new();
        for _ in 0..8 {
            handles.push(thread::spawn(|| {
                let dir = TestDir::new("concurrent-test");
                assert!(dir.path().is_dir());
                let file = dir.path().join("test.txt");
                fs::write(&file, "data").unwrap();
                assert_eq!(fs::read_to_string(&file).unwrap(), "data");
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
