extern crate pico_args;
extern crate tempfile;
extern crate walkdir;
extern crate zip;

use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::process;
use std::process::Command;
use walkdir::WalkDir;

const VERSION: &str = "0.0.0";
const USAGE: &str = "usage: epub-optimizer file... [OPTIONS]

Arguments:
    file             Path(s) to EPUB files to optimize.

Options:
    -h, --help
    -v, --version
";

fn main() {
    let mut args = pico_args::Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        eprintln!("{}", &USAGE);
        process::exit(1)
    }

    if args.contains(["-v", "--version"]) {
        eprintln!("{}", &VERSION);
        process::exit(1)
    }

    let matches = args.free().unwrap_or_else(|_| {
        eprintln!("{}", &USAGE);
        process::exit(1)
    });

    if matches.is_empty() {
        eprintln!("You must specify at least one file.\n\n{}", &USAGE);
        process::exit(1)
    }

    let mut bytes_saved: i64 = 0;
    for path in matches {
        println!("{}:", path);
        // TODO: let's give better error feedback here
        let original_len = fs::metadata(&path).unwrap().len() as i64;
        let tmp = unzip(&path);
        minify(&tmp);
        gen_epub(&path, &tmp);
        let optimized_len = fs::metadata(&path).unwrap().len() as i64;
        bytes_saved += original_len - optimized_len;
        println!();
    }

    println!("{}KiB saved in total.", bytes_saved / 1024);
}

fn unzip(path: &str) -> tempfile::TempDir {
    println!("Reading ZIP...");
    let file = File::open(&path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();

    println!("Extracting ZIP...");
    let tmp = tempfile::tempdir().unwrap();
    for i in 0..zip.len() {
        let mut input = zip.by_index(i).unwrap();
        if input.name().ends_with('/') {
            continue;
        }
        let input_path = input.sanitized_name();

        let output_path = tmp.path().join(input_path);
        let _ = fs::create_dir_all(output_path.parent().unwrap());
        let mut output = File::create(output_path).unwrap();

        io::copy(&mut input, &mut output).unwrap();
    }

    tmp
}

fn minify(tmp: &tempfile::TempDir) {
    println!("Minifying files...");
    let mut bytes_saved = 0;
    for entry in WalkDir::new(&tmp) {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();

        let ext = path.extension();
        if ext == None {
            continue;
        }
        let ext = ext.unwrap();

        let original_len = entry.metadata().unwrap().len();
        match ext.to_str().unwrap() {
            "opf" | "xml" | "html" | "htm" => {
                Command::new("minify")
                    .arg("--mime=text/xml")
                    .arg(path)
                    .output()
                    .unwrap();
            }
            "css" | "svg" => {
                Command::new("minify")
                    .arg(path)
                    .arg("-o")
                    .arg(path)
                    .output()
                    .unwrap();
            }
            "jpeg" | "jpg" => {
                Command::new("jpegoptim")
                    .arg("--strip-all")
                    .arg("--max=90")
                    .arg(path)
                    .output()
                    .unwrap();
            }
            "png" => {
                Command::new("pngquant")
                    .arg("--skip-if-larger")
                    .arg("--force")
                    .arg("--ext")
                    .arg(".png")
                    .arg("--quality=90")
                    .arg(path)
                    .output()
                    .unwrap();
            }
            _ => {}
        }
        bytes_saved += original_len - entry.metadata().unwrap().len();
        print!("\r{}KiB saved.", bytes_saved / 1024);
        io::stdout().flush().unwrap();
    }
    println!();
}

fn gen_epub(path: &str, tmp: &tempfile::TempDir) {
    println!("Zipping...");
    let wd = env::current_dir().unwrap();
    let path_abs = fs::canonicalize(&path).unwrap();

    let _ = fs::remove_file(&path);
    env::set_current_dir(&tmp).unwrap();
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
