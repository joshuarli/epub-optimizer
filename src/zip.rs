use std::{env, fs, fs::File, io, process::Command};

extern crate zip;

use tempdir::TempDir;

pub fn unzip(path: &str) -> TempDir {
    let file = File::open(&path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();

    let tmpdir = TempDir::new().unwrap();
    let prefix = tmpdir.path();
    for i in 0..zip.len() {
        let mut input = zip.by_index(i).unwrap();
        if input.name().ends_with('/') {
            continue;
        }
        let output_path = prefix.join(input.sanitized_name());
        // TODO: We can do better than this.
        fs::create_dir_all(output_path.parent().unwrap()).unwrap();
        let mut output = File::create(output_path).unwrap();
        io::copy(&mut input, &mut output).unwrap();
    }

    tmpdir
}

pub fn zip(dest: &str, src: &TempDir) {
    let wd = env::current_dir().unwrap();
    let path_abs = fs::canonicalize(&dest).unwrap();

    let _ = fs::remove_file(&dest);
    env::set_current_dir(src.path()).unwrap();
    // TODO: use the zip crate to do this
    let mut cmd = Command::new("zip");
    cmd.arg("-9r");
    cmd.arg(&path_abs);
    for path in fs::read_dir(".").unwrap() {
        cmd.arg(path.unwrap().path());
    }
    cmd.output().unwrap();
    env::set_current_dir(wd).unwrap();
}
