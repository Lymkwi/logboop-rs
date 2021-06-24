// Stuff
use std::io::prelude::*;
use std::path::Path;
use std::fs::{OpenOptions, File, remove_file};

use crate::filesystem;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

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
    remove_file(filepath)?;
    Ok(())
}

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
    remove_file(&filepath)?;

    Ok(())
}
