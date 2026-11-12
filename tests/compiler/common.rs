use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

pub struct TestDirectory(PathBuf);

impl TestDirectory {
    pub fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zenvik-e2e-{}-{sequence}-with spaces",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn zenvik(arguments: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zenvik"))
        .args(arguments)
        .output()
        .unwrap()
}

pub fn arg(path: &std::path::Path) -> &std::ffi::OsStr {
    path.as_os_str()
}

pub struct OsStrExt;

impl OsStrExt {
    pub fn as_os_str(value: &str) -> &std::ffi::OsStr {
        std::ffi::OsStr::new(value)
    }
}
