extern crate fastrand;
extern crate pico_args;
extern crate walkdir;
extern crate zip;

use std::{
    env, env::temp_dir, fs, fs::File, io, io::Write, iter::repeat_with, path::PathBuf, process,
    process::Command,
};

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
    let tmp = unzip(&path);
    minify(&tmp, &verbose);
    gen_epub(&path, &tmp);
    // TODO: register this at exit. how did tempdir crate do this?
    fs::remove_dir_all(&tmp).unwrap();

    let new_size_bytes = fs::metadata(&path).unwrap().len();
    println!(
        "{}: {} KiB ({:.2}%) saved.",
        path,
        (old_size_bytes - new_size_bytes) / 1024,
        100 * (old_size_bytes - new_size_bytes) / old_size_bytes
    );
}

fn mktmpdir() -> PathBuf {
    let tmpdir = temp_dir().join(
        repeat_with(fastrand::alphanumeric)
            .take(8)
            .collect::<String>(),
    );
    fs::create_dir(&tmpdir).unwrap();
    tmpdir
}

fn unzip(path: &str) -> PathBuf {
    let file = File::open(&path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();

    let tmpdir = mktmpdir();
    for i in 0..zip.len() {
        let mut input = zip.by_index(i).unwrap();
        if input.name().ends_with('/') {
            continue;
        }
        let output_path = tmpdir.join(input.sanitized_name());
        // TODO: We can do better than this.
        fs::create_dir_all(output_path.parent().unwrap()).unwrap();
        let mut output = File::create(output_path).unwrap();
        io::copy(&mut input, &mut output).unwrap();
    }

    tmpdir
}

fn minify(tmpdir: &PathBuf, verbose: &bool) {
    for entry in walkdir::WalkDir::new(tmpdir) {
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
            "opf" | "xml" => {
                Command::new("minify")
                    .arg("--mime=text/xml")
                    .arg(path)
                    .arg("-o")
                    .arg(path)
                    .output()
                    .unwrap();
            }
            "html" | "htm" => {
                Command::new("minify")
                    .arg("--mime=text/html")
                    .arg(path)
                    .arg("-o")
                    .arg(path)
                    .output()
                    .unwrap();
            }
            "xhtml" => {
                Command::new("minify")
                    .arg("--mime=text/xhtml+xml")
                    .arg(path)
                    .arg("-o")
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

fn gen_epub(path: &str, tmpdir: &PathBuf) {
    let wd = env::current_dir().unwrap();
    let path_abs = fs::canonicalize(&path).unwrap();

    let _ = fs::remove_file(&path);
    env::set_current_dir(tmpdir).unwrap();
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
