use std::{fs, fs::File, io, io::Read, io::Write, path::Path};

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

pub fn zip(dest: &str, src: &Path) -> zip::result::ZipResult<()> {
    let f = File::create(&dest).unwrap();
    let mut zip = zip::ZipWriter::new(f);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut buffer = Vec::new();
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.strip_prefix(src).unwrap();

        if path.is_file() {
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;
            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
            buffer.clear();
            continue;
        }
        zip.add_directory_from_path(name, options)?;
    }

    zip.finish()?;
    Ok(())
}
