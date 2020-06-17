extern crate clap;
extern crate tempfile;
extern crate walkdir;
extern crate zip;

use clap::App;
use clap::Arg;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    let matches = App::new("epub-optimizer")
        .about("A command-line app that optimizes and edits metadata of .epub files")
        .arg(
            Arg::with_name("no-optimize")
                .help("Disables optimization")
                .long("no-optimize"),
        )
        .arg(
            Arg::with_name("files")
                .help("List of files to optimize")
                .required(true)
                .min_values(1),
        )
        .get_matches();

    let optimize = !matches.is_present("no-optimize");

    let mut bytes_saved: i64 = 0;
    for path in matches.values_of("files").unwrap() {
        println!("{}:", path);
        let original_len = fs::metadata(path).unwrap().len() as i64;
        process(path, optimize);
        let optimized_len = fs::metadata(path).unwrap().len() as i64;
        bytes_saved += original_len - optimized_len;

        println!();
    }

    if optimize {
        println!("{}KiB saved in total.", bytes_saved / 1024);
    } else {
        println!("Done.")
    }
}

fn process(path: &str, optimize: bool) {
    let tmp = unzip(path);
    if optimize {
        minify(&tmp);
    }
    gen_epub(path, &tmp);
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
    let mut cmd = Command::new("zip");
    cmd.arg("-9r");
    cmd.arg(&path_abs);
    for path in fs::read_dir(".").unwrap() {
        cmd.arg(path.unwrap().path());
    }
    cmd.output().unwrap();
    env::set_current_dir(wd).unwrap();
}
