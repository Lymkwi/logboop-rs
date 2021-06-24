//! `LogBoop`, a program to parse, split, and destroy old log files
//!
//! Author : Lux
//!
//! License : CC0

#[macro_use] extern crate lazy_static;
extern crate regex;
extern crate walkdir;
extern crate flate2;
extern crate chrono;

mod filesystem;
mod compress;
mod process;

use std::fs::create_dir_all;
use std::path::Path;
use std::env::{args, Args};

/// Main method. Entrypoint to the system.
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
