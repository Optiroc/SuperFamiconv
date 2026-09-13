//! Support code for e2e tests.
//!
//! Set `SFC_KEEP_TMP` to skip tmp dir cleanup.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(name: &str) -> Self {
        let path = Path::new("tmp").join(format!("{name}-{}", test_id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub fn file(
        &self,
        name: &str,
    ) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        if std::env::var_os("SFC_KEEP_TMP").is_some() {
            return;
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn test_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let pid = u64::from(std::process::id());
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{:04x}-{:04x}-{:08x}", count as u16, pid as u16, nanos & 0xffff_ffff)
}

pub fn file_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap()
}

pub fn file_le_words(path: &Path) -> Vec<u16> {
    fs::read(path)
        .unwrap()
        .chunks_exact(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect()
}

pub fn file_len(path: &Path) -> u64 {
    fs::metadata(path).unwrap().len()
}
