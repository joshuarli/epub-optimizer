extern crate pico_args;
extern crate walkdir;

use std::{env, fs, process, process::Command, thread};

mod tempdir;
use tempdir::TempDir;
mod zip;

const VERSION: &str = "0.0.0";
const USAGE: &str = "usage: epub-optimizer FILE [OPTIONS]

Arguments:
    FILE             Path to the EPUB file to optimize.

Options:
    -h, --help
    -V, --version
    -v, --verbose    Verbose output. This prints every optimized file in the EPUB archive.
";

fn main() {
    let mut args = pico_args::Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        eprintln!("{}", &USAGE);
        process::exit(1)
    }

    if args.contains(["-V", "--version"]) {
        eprintln!("{}", &VERSION);
        process::exit(1)
    }

    let verbose = args.contains(["-v", "--verbose"]);

    let matches = args.free().unwrap_or_else(|_| {
        eprintln!("{}", &USAGE);
        process::exit(1)
    });

    if matches.is_empty() {
        eprintln!("You must specify a FILE.\n\n{}", &USAGE);
        process::exit(1)
    }

    let path = &matches[0];
    let metadata = fs::metadata(&path).unwrap_or_else(|_| {
        eprintln!("{} doesn't exist.", path);
        process::exit(1)
    });

    if !path.ends_with(".epub") {
        // we could also additionally check for the manifest after unzipping to be even more correct
        eprintln!("{} does not have an .epub extension.", path);
        process::exit(1)
    }

    let old_size_bytes = metadata.len();
    let workdir = zip::unzip(&path);
    minify(&workdir, &verbose);
    zip::zip(&path, &workdir.path()).unwrap();

    let new_size_bytes = fs::metadata(&path).unwrap().len();
    println!(
        "{}: {} KiB ({:.2}%) saved.",
        path,
        (old_size_bytes - new_size_bytes) / 1024,
        100 * (old_size_bytes - new_size_bytes) / old_size_bytes
    );
}

fn minify(tmpdir: &TempDir, _verbose: &bool) {
    let pwd = env::current_dir().unwrap();
    env::set_current_dir(tmpdir.path()).unwrap();

    let mut jpgs = Vec::new();
    let mut pngs = Vec::new();

    for entry in walkdir::WalkDir::new(".") {
        // This loop is kept sequential to be simple.
        // In the future, we could do all this concurrently, and even take multiple EPUB files.
        // But passing this to fd like `fd -e epub -x epub-optimizer {}` lets us saturate all cores easily.
        // (By default, fd uses nproc execution threads, and jpegoptim and pngquant are single-threaded.)
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();
        if metadata.is_dir() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension();
        if ext == None {
            continue;
        }
        let ext = ext.unwrap();

        match ext.to_str().unwrap().to_ascii_lowercase().as_str() {
            // TODO: "svg" | "xml" | "opf"
            // This used to be https://github.com/tdewolff/minify (via joshuarli/minify-cli)
            // but it's just really painful to exec a heavy go binary.
            // "html" | "xhtml"
            // Using minify or wilsonzlin/minify-html results in corruption (Apple Books, Thorium)
            // TODO: I'll look into this in the future. Low priority; HTML minification barely saves any space.
            "jpeg" | "jpg" => {
                jpgs.push(path.to_owned());
            }
            "png" => {
                pngs.push(path.to_owned());
            }
            _ => {
                continue;
            }
        }
    }

    let mut workers = Vec::new();

    if !jpgs.is_empty() {
        workers.push(thread::spawn(|| {
            // TODO: Size reporting.
            // I haven't yet found a good rust-native jpeg optimizer.
            // Until then, per-file size reporting is going to be silly to implement.
            // I'd much rather pass all the filepaths for a single exec.
            // TODO: Do we need to guard against exceeding exec length limit?
            let mut jpegoptim = Command::new("jpegoptim");
            jpegoptim
                .arg("--strip-all")
                .arg("--max=90")
                .args(jpgs)
                .output()
                .unwrap();
        }));
    }

    if !pngs.is_empty() {
        workers.push(thread::spawn(|| {
            // TODO: Size reporting.
            // I haven't yet found a good rust-native png optimizer.
            // Until then, per-file size reporting is going to be silly to implement.
            // I'd much rather pass all the filepaths for a single exec.
            // TODO: Do we need to guard against exceeding exec length limit?
            let mut pngquant = Command::new("pngquant");
            pngquant
                .arg("--skip-if-larger")
                .arg("--force")
                .arg("--ext")
                .arg(".png")
                .arg("--quality=90")
                .args(pngs)
                .output()
                .unwrap();
        }));
    }

    for worker in workers {
        worker.join().unwrap();
    }

    env::set_current_dir(pwd).unwrap();
}
