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
        Self::create_in(&std::env::temp_dir(), 20, || {
            let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            format!(
                "agent-linker-{label}-{}-{}-{}",
                std::process::id(),
                timestamp_nanos(),
                counter
            )
        })
    }

    pub fn create_in(base: &Path, max_retries: usize, name_gen: impl FnMut() -> String) -> Self {
        Self::try_create_in(base, max_retries, name_gen).unwrap_or_else(|err| {
            panic!(
                "failed to create test directory in {}: {err}",
                base.display()
            )
        })
    }

    pub fn try_create_in(
        base: &Path,
        max_retries: usize,
        mut name_gen: impl FnMut() -> String,
    ) -> std::io::Result<Self> {
        for _ in 0..max_retries {
            let path = base.join(name_gen());
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("exhausted {max_retries} attempts creating unique test directory"),
        ))
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

        // Call the ACTUAL creation logic (TestDir::create_in), injecting the occupied path first
        let mut tried = false;
        let dir = TestDir::create_in(&base, 10, || {
            if !tried {
                tried = true;
                occupied_name.clone()
            } else {
                format!("agent-linker-occupied-test-{}-unique", timestamp_nanos())
            }
        });

        assert!(dir.path().is_dir());
        assert_ne!(dir.path(), occupied_path.as_path());
        assert!(sentinel.is_file());
        drop(dir);

        // Sentinel and occupied directory must still be intact
        assert!(occupied_path.is_dir());
        assert!(sentinel.is_file());
        assert_eq!(fs::read_to_string(&sentinel).unwrap(), "preserve");
        let _ = fs::remove_dir_all(&occupied_path);
    }

    #[test]
    fn test_dir_exhausts_retries_without_deleting_occupied_sentinel() {
        let base = std::env::temp_dir();
        let occupied_name = format!("agent-linker-always-occupied-{}", timestamp_nanos());
        let occupied_path = base.join(&occupied_name);
        fs::create_dir(&occupied_path).unwrap();
        let sentinel = occupied_path.join("sentinel.txt");
        fs::write(&sentinel, "preserve-always").unwrap();

        let mut attempts = 0;
        let result = TestDir::try_create_in(&base, 3, || {
            attempts += 1;
            occupied_name.clone()
        });
        let error = result
            .err()
            .expect("occupied directory must exhaust retries");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(attempts, 3);

        // Sentinel and occupied directory must still be completely untouched
        assert!(occupied_path.is_dir());
        assert!(sentinel.is_file());
        assert_eq!(fs::read_to_string(&sentinel).unwrap(), "preserve-always");
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
