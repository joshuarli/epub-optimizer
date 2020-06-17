extern crate fastrand;

use std::{env::temp_dir, fs, io, iter::repeat_with, path, path::PathBuf};

pub struct TempDir {
    path: Option<PathBuf>,
}

impl TempDir {
    pub fn new() -> io::Result<TempDir> {
        let path = temp_dir().join(
            repeat_with(fastrand::alphanumeric)
                .take(8)
                .collect::<String>(),
        );
        match fs::create_dir(&path) {
            Ok(_) => Ok(TempDir { path: Some(path) }),
            Err(e) => Err(e),
        }
        // TODO: retry if already exists
    }

    pub fn path(&self) -> &path::Path {
        self.path.as_ref().unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(self.path()).unwrap();
    }
}
