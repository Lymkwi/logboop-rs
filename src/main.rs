//! `LogBoop`, a program to parse, split, and destroy rotated log files
//!
//! Author : Lux
//!
//! License : CC0
//!
//! `LogBoop` was adapted from a bash script hastely written to handle
//! an unexpected surplus of log files rotated from my personal VPS.
//!
//! # Using `LogBoop`
//! Using `LogBoop` is easy. Once the binary is compiled, simply invoke it
//! ```bash
//! logboop input_root output_root
//! ```
//!
//! Note that you will need the required privilege to read all files and folders
//! in the `input_root` directory, create directories and files in
//! `output_root`` (or create it as well if needed), and enough disk space to
//! duplicate the contents of `input_root` (roughly).
#![doc(issue_tracker_base_url = "https://github.com/Lymkwi/logboop/issues/")]

/* Crates used by this crate */
// Lazy static is used to define constant regexes at compile time
#[macro_use] extern crate lazy_static;
// Regexes are used to detect and match log types
extern crate regex;
// WalkDir is used to easily walk through a directory tree structure
// in order to operate on files in the input/output directories
extern crate walkdir;
// Flate2 is used for anything related to GZ compression/deflation
extern crate flate2;
// Chrono is used to manage, infer and format dates from the logs
extern crate chrono;

mod filesystem;
mod compress;
mod process;

/* Needed imports for the main module */
// We actually create the output directory here
use std::fs::create_dir_all;
// We manipulate paths
use std::path::Path;
// Arguments are used to retrieve the input/output directories
use std::env::{args, Args};

#[doc(hidden)]
fn main() {
    // Check that we have all of the arguments
    let mut argv: Args = args();
    let progname = argv.next().unwrap();

    // Check that we have an input folder
    let potential_path: Option<String> = argv.next();
    if potential_path.is_none() {
        eprintln!("{} : missing argument (input folder path)", progname);
        return;
    }
    
    let input_path_str: String = potential_path.unwrap();

    // Retrieve a potential second argument
    let output_path_str: String = argv.next()
        .unwrap_or_else(|| "output".to_owned());

    // Now, assess the input path
    let input_path = Path::new(&input_path_str);
    let output_path = Path::new(&output_path_str);

    // Input ok ?
    if !input_path.is_dir() {
        eprintln!("{} : input path (\"{}\") is not a directory", progname, input_path_str);
        return;
    }

    // Output ok ?
    if !output_path.is_dir() {
        // If the output folder does not exist, we can try and create it...
        if output_path.exists() {
            eprintln!("{} : output path (\"{}\") exists and is not a directory", progname, output_path_str);
            return;
        }
        if let Err(e) = create_dir_all(&output_path) {
            eprintln!("{} : error while creating output folder : {}",
                      progname, e);
            return;
        }
    }

    // Degunzip all the files
    println!("--- Beginning Degunzipping procedure ---");
    if let Err(e) = filesystem::degunzip_all_the_files(&input_path) {
        eprintln!("{} : terrible : {}", progname, e);
        return;
    }
    println!("--- All compressed files degunzipped ---");

    // Process all of the files
    println!("--- Processing all of the files ---");
    if let Err(e) = process::all_files(&input_path, &output_path) {
        eprintln!("{} : Error during file processing : {}", progname, e);
        return;
    }
    println!("--- All files processed ---");

    // Regunzip all the dated files
    println!("--- Compressing all of the output files ---");
    if let Err(e) = filesystem::gunzip_all_the_files(&output_path) {
        eprintln!("{} : Error during file compressing : {}", progname, e);
        return;
    }
    println!("--- All files successfully compressed ---");
}
