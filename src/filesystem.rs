//

use std::io::prelude::*;
use std::path::{Path,PathBuf};
use walkdir::WalkDir;

use regex::Regex;

use crate::compress;

lazy_static! {
    static ref ISO_DATE_REGEX: Regex = Regex::new(r"-\d{4}-\d{2}-\d{2}$").unwrap();
}

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

pub fn degunzip_all_the_files(inpath: &Path) -> std::io::Result<()> {
    // Within all the folders, we need to find and de-gunzip all the files
    // That end with a `.gz` extension
    // Open the directory, and iterate
    for entry in WalkDir::new(inpath) {
        let entry = entry?.into_path();
        if let Some(ext) = entry.extension() {
            print!("{} ", entry.display());
            std::io::stdout().flush()?;
            if ext == "gz" {
                compress::degunzip(&entry)?;
                println!("\u{2713}");
            } else {
                println!("-");
            }
        }
    }
    Ok(())
}

pub fn gunzip_all_the_files(outpath: &Path) -> std::io::Result<()> {
    //
    for entry in WalkDir::new(outpath) {
        let pbuf = entry?.into_path();
        let entry = pbuf.to_str().unwrap();
        if ISO_DATE_REGEX.is_match(entry) {
            print!("Compressing {}... ", entry);
            std::io::stdout().flush()?;
            compress::gunzip(&pbuf)?;
            println!("\u{2713}");
        }
    }
    Ok(())
}
