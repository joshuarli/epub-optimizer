extern crate pico_args;
extern crate walkdir;

use std::{fs, io, io::Write, process, process::Command};

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
    // TODO: validate extension is at least epub jsut to guard against accidental unzipping
    //       we could also additionally check for the manifest after unzipping to be even more correct
    let metadata = fs::metadata(&path).unwrap_or_else(|_| {
        eprintln!("{} doesn't exist.", path);
        process::exit(1)
    });

    let old_size_bytes = metadata.len();
    let workdir = zip::unzip(&path);
    minify(&workdir, &verbose);
    zip::zip(&path, &workdir);

    let new_size_bytes = fs::metadata(&path).unwrap().len();
    println!(
        "{}: {} KiB ({:.2}%) saved.",
        path,
        (old_size_bytes - new_size_bytes) / 1024,
        100 * (old_size_bytes - new_size_bytes) / old_size_bytes
    );
}

fn minify(tmpdir: &TempDir, verbose: &bool) {
    for entry in walkdir::WalkDir::new(tmpdir.path()) {
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

        let old_size_bytes = metadata.len();
        match ext.to_str().unwrap().to_ascii_lowercase().as_str() {
            "html" | "htm" | "css" | "svg" | "xml" => {
                Command::new("minify")
                    .arg(path)
                    .arg("-o")
                    .arg(path)
                    .output()
                    .unwrap();
            }
            // XXX: Upstream minify-cli doesn't currently infer mimetypes for opf and xhtml.
            // The correct mimetype for xhtml is text/xhtml+xml, but text/xml works fine.
            // My own extracted minify-cli at https://github.com/joshuarli/minify-cli supports this.
            "opf" | "xhtml" => {
                Command::new("minify")
                    .arg("--mime=text/xml")
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
            _ => {
                continue;
            }
        }

        if *verbose {
            let new_metadata = entry.metadata().unwrap();
            let new_size_bytes = new_metadata.len();
            println!(
                "{} {} KiB -> {} KiB ({:.2}% smaller)",
                path.display(),
                old_size_bytes / 1024,
                new_size_bytes / 1024,
                100 * (old_size_bytes - new_size_bytes) / old_size_bytes,
            );
            io::stdout().flush().unwrap();
        }
    }
}
