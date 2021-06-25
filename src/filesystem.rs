//! Module for easy filesystem operations
//!
//! Filesystem contains all of the operations related to creating/deleting
//! files, triggering a global compression/deflation of a given directory,
//! or processing of a given directory.
//!
//! # Provided by this module
//! Various methods to simplify repetitive filesystem manipulation operations
//! are provided (adding an extension to a path, degunzip'ing all `.gz` files
//! in a folder, and gunzip'ing all files with the correct name format in
//! a directory).
//!
//! Examples are provided for each individual function.
//!
//! # Details of imports and crates
//!
//! ## Standard library imports
//!
//! We import things needed to manipulate I/O, Paths, and some OS-specific
//! strings :
//!  - The entire [I/O `prelude`](std::io::prelude)
//!  - [`Path`] and its owned version, [`PathBuf`]
//!  - The OS-specific [`OsString`], needed to specify one argument when
//!  extracting and inspecting extensions recursively (in
//!  [`degunzip_all_the_files`])
//!
//! ## Crate imports
//!
//! Some crate imports are needed as well :
//!  - We need to define a [`Regex`] to match the end of files we want
//!  to compress again
//!  - [`WalkDir`] will let us easily walk recursively in the directories
//!  we inspect
//!  - [`compress`] since we call [`gunzip`](crate::compress::gunzip)
//!  and [`degunzip`](crate::compress::degunzip) on individual
//!  files.

use std::io::prelude::*;
use std::path::{Path,PathBuf};
use std::ffi::OsString;

use regex::Regex;
use walkdir::WalkDir;

use crate::compress;

lazy_static! {
    /// Regex object used to match the ISO 8601 date format at the end of
    /// a file name
    ///
    /// Its exact regex is `-\d{4}-\d{2}-\d{2}` (a hyphen is added before
    /// the date when we create the file)
    static ref ISO_DATE_REGEX: Regex = Regex::new(r"-\d{4}-\d{2}-\d{2}$").unwrap();
}

/// Add an extension to a path
///
/// # Arguments
/// We receive two arguments :
/// - A mutable reference to an owned path
/// ([`&mut PathBuf`](std::path::PathBuf))
/// - The addition, a slice str [`&str`]
///
/// # Behaviour
/// If the path given does not already contain an extension, set the extension
/// to whatever was supposed to be added.
///
/// Giving an empty string changes nothing :
/// ```
/// let mut path_ex = PathBuf::from("a_file");
/// add_extension(&mut path_ex, "");
/// assert_eq!(path_ex, PathBuf::from("a_file"));
/// ```
pub fn add_extension(path: &mut PathBuf, addition: &str) {
    match path.extension() {
        Some(ext) => {
            let mut ext_os = ext.to_os_string();
            ext_os.push(".");
            ext_os.push(addition);
            path.set_extension(ext_os);
        },
        None => {path.set_extension(addition);}
    }
}

/// Recursively inflate all GZ files in a directory
///
/// # Arguments
/// This method only needs one argument, a [`&Path`](std::path::Path).
///
/// # Behaviour
///
/// When given a path, this method recursively iterates all files in the
/// folder (and at this point in the program it must be a folder),
/// inspects the extension (if any) of the file name, and if it is "gz",
/// trigger a [`degunzip`](crate::compress::degunzip).
///
/// # Errors
/// This method will return a `std::io::Result<()>`, and can be invoked
/// with the `?` syntax sugar. When an internal error occurs (with printing,
/// or with degunzip), that error will flow upwards.
///
/// # Example
/// This method can be used thusly.
/// ```
/// let my_files_path = Path::new("var/log");
/// degunzip_all_the_files(&my_files_path)?;
/// ```
pub fn degunzip_all_the_files(inpath: &Path) -> std::io::Result<()> {
    // Within all the folders, we need to find and de-gunzip all the files
    // That end with a `.gz` extension
    // Open the directory, and iterate
    WalkDir::new(inpath)
        .into_iter()
        .filter_map(|entry| entry.map(walkdir::DirEntry::into_path).ok())
        .filter(|entry| entry.is_file())
        .filter_map(|entry| entry.extension().map(|e| (entry.clone(), e.to_owned())))
        .try_for_each(
            |(entry, ext): (PathBuf, OsString)| -> std::io::Result<_> {
                print!("{} ", entry.display());
                std::io::stdout().flush()?;
                if ext == "gz" {
                    compress::degunzip(&entry)?;
                    println!("\u{2713}");
                } else {
                    println!("-");
                }
                Ok(())
            }
        )
}

/// Recursively compress the appropriate files in a directory
///
/// # Arguments
/// This method only needs one argument, a [`&Path`](std::path::Path).
///
/// # Behaviour
///
/// When given a path, this method recursively iterates all files in the
/// folder (and at this point in the program it must be a folder),
/// inspects the end of the file name, and if it matches a simple ISO 8601 date
/// format, compress it using [`gunzip`](crate::compress::gunzip).
///
/// # Errors
/// This method will return a `std::io::Result<()>`, and can be invoked
/// with the `?` syntax sugar. When an internal error occurs (with printing,
/// or with gunzip), that error will flow upwards.
///
/// # Example
/// This method can be used thusly.
/// ```
/// let my_files_path = Path::new("var/log");
/// gunzip_all_the_files(&my_files_path)?;
/// ```
pub fn gunzip_all_the_files(outpath: &Path) -> std::io::Result<()> {
    //
    WalkDir::new(outpath)
        .into_iter()
        .filter_map(|entry| entry.map(walkdir::DirEntry::into_path).ok())
        .filter(|entry| entry.is_file())
        .filter(|entry| entry
                .to_str()
                .map_or(false,
                        |fname| ISO_DATE_REGEX.is_match(fname)
        ))
        .try_for_each(|entry: PathBuf| -> std::io::Result<_> {
            print!("Compressing {}... ", entry.display());
            std::io::stdout().flush()?;
            compress::gunzip(&entry)?;
            println!("\u{2713}");
            Ok(())
        })
}
