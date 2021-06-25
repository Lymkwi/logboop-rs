//! Module that handles every operation for compression/deflation of
//! individual files
//!
//! ## Provided by this module
//!
//! This module provides two methods to inflate/compress individual files
//! as provided by [`&Path`](std::path::Path) references, called
//! [`degunzip`](degunzip) and [`gunzip`](gunzip)
//! (named after their original counterparts in my script,
//! themself named after the command typically used to perform this operation).
//!
//! ## Example
//!
//! They can be invoked thusly :
//! ```rust
//! fn function_that_returns_error() -> std::io::Result<()> {
//!     let p = Path::new("my_file.gz");
//!     degunzip(&p)?;
//!     let u = Path::new("my_file");
//!     gunzip(&u)
//! }
//! ```
//!
//! ## Details of imports and crates
//!
//! ### Imports from the standard library
//!
//! We need things to do I/O, and some fs manipulation
//!  - The [I/O prelude](std::io::prelude)
//!  - [Paths](std::path::Path)
//!  - filesystem manipulation tools like [`OpenOptions`](std::fs::OpenOptions)
//!  (used to chose write/create modes), [`File`](std::fs::File), and
//!  [`remove_file`](std::fs::remove_file)
//!
//! ### Crate imports
//!
//! In line with the statements from the previous section, we also import
//!  - Our own [`filesystem`](crate::filesystem), to use the [`add_extension`](crate::filesystem::add_extension)
//!  method when creating the compressed file
//!  - The [`GzEncoder`] and [`GzDecoder`]
//!  - The structure [`Compression`] from `flate2` to
//!  indicate a default level of compression
use std::io::prelude::*;
use std::path::Path;
use std::fs::{OpenOptions, File, remove_file};

use crate::filesystem;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

/// Inflate a given file with default GZ compression
///
/// # Arguments
/// Given a [`&Path`](std::path::Path), find and inflate the contents
/// using a GZ decoder.
///
/// # Exceptions
/// This method may throw an I/O [`Error`](std::io::Error) when opening
/// the file, reading its content, decoding said contents, creating the
/// output file, writing to it, or removing the original file.
///
/// # Example
/// This is a minimal example.
/// ```
/// let p: Path = Path::new("my_file.gz");
/// if let Err(e) = degunzip(&p) {
///     eprintln!("Error when inflating : {}", e);
/// }
/// // There must now be a file called "my_file"
/// ```
pub fn degunzip(filepath: &Path) -> std::io::Result<()> {
    let mut fptr = File::open(filepath)?;
    let mut outbuf: Vec<u8> = Vec::new();
    fptr.read_to_end(&mut outbuf)?;
    // Get a GZ decoder
    let mut decoder = GzDecoder::new(&outbuf[..]);
    let mut sout: Vec<u8> = Vec::new();
    decoder.read_to_end(&mut sout)?;

    // Build the file name of the destination
    let final_destination = filepath.with_extension("");
    let mut out_fptr = File::create(final_destination)?;
    out_fptr.write_all(&sout)?;

    // And remove the original
    remove_file(filepath)
}

/// Compress a given file with default GZ compression
///
/// # Arguments
/// Given a [`&Path`](std::path::Path), find and deflate the contents
/// using a GZ decoder.
///
/// # Exceptions
/// This method may throw an I/O [`Error`](std::io::Error) when opening
/// the file, reading its content, creating the output file and opening it,
/// writing the content of the first file into the encoder, finalizing the
/// encoding, and removing the original file.
///
/// # Example
/// This is a minimal example.
/// ```
/// let p: Path = Path::new("my_file");
/// if let Err(e) = gunzip(&p) {
///     eprintln!("Error when compressing : {}", e);
/// }
/// // There must now be a file called "my_file.gz"
/// ```
pub fn gunzip(filepath: &Path) -> std::io::Result<()> {
    // Read the data from the raw file
    let mut fptr = File::open(filepath)?;
    let mut outbuf: Vec<u8> = Vec::new();
    fptr.read_to_end(&mut outbuf)?;
    // Open the output file
    let mut owned_path = filepath.to_path_buf();
    filesystem::add_extension(&mut owned_path, "gz");
    let out_fptr = OpenOptions::new()
        .write(true)
        .create(true)
        .open(owned_path)?;

    // Get a GZ encoder
    let mut encoder = GzEncoder::new(out_fptr, Compression::default());
    encoder.write_all(&outbuf)?;
    encoder.finish()?;

    // Remove the file
    remove_file(&filepath)
}
