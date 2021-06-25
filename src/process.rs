//! Processing module, defining the mechanism to recognize log formats,
//! extract dates, process individual files and write them to the output
//! directory.
//!
//! # Provided
//!
//! This module provides the core processing logic, with regexes, enums and
//! structures created to analyse and parse files into the desired output.
//!
//! The [`FileProcessor`] structure is the core of this logic, but the endpoints
//! that should be used directly are [`one_file`] to process one file and
//! [`all_files`] for the recursive processing of a directory.
//!
//! # Imports
//! ## Standard library imports
//! We need to accomplish all sorts of I/O and file operations, so
//!  - The entire [I/O prelude](std::io::prelude) is imported
//!  - [`File`], [`OpenOptions`], [`remove_file`] and [`create_dir_all`] from
//!  the [`std::fs`] module
//!  - [`BufReader`] and [`BufWriter`], buffered writers from the I/O module
//!  - Both [`Path`] and [`PathBuf`] for path manipulation
//!  - Finally, the [`HashMap`] collection to store regexes supposed to match
//!  a given [`LogType`]
//!
//! ## Crate imports
//! In order to conduct our business, we import
//!  - [`Regex`]
//!  - [`WalkDir`]
//!  - [`Datelike`], the trait needed to make [`NaiveDate`] format from dates
//!  using [`StrftimeItems`]
use std::io::prelude::*;
use std::fs::{File, OpenOptions, remove_file, create_dir_all};
use std::io::{BufReader,BufWriter};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use regex::Regex;
use walkdir::WalkDir;
use chrono::Datelike;
use chrono::NaiveDate;
use chrono::format::strftime::StrftimeItems;

// Define the dictionary of matching regexes for data
lazy_static! {
    #[doc(hidden)]
    static ref REGEXES: HashMap<LogType, Regex> = vec![
        (LogType::Syslog, Regex::new(r"^(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) ([012 ]\d|3[01])").unwrap()),
        (LogType::Iso, Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap()),
        (LogType::ApacheAccess, Regex::new(r"\[\d{2}/(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)/\d{4}:").unwrap()),
        (LogType::ApacheError, Regex::new(r"\[(Mon|Tue|Wed|Thu|Fri|Sat|Sun) (Jan|Feb||Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) \d{2} \d{2}:\d{2}:\d{2}.\d{6} \d{4}]").unwrap()),
        (LogType::GrafanaLogs, Regex::new(r"^t=\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\+|-)\d{4} lvl=").unwrap())
    ]
    .into_iter().collect::<HashMap<LogType, Regex>>();
    #[doc(hidden)]
    static ref NUMBER_REGEX: Regex = Regex::new(r"^\d+$").unwrap();
    // And this is the format (StrFtimeItems) for ISO 8601 dates
}

/// An enumeration representing possible log types
///
/// This enum has different values, each one representing a different format
/// of logs detected by the program while scanning a file.
#[derive(std::hash::Hash, std::cmp::Eq, std::cmp::PartialEq, std::fmt::Debug)]
enum LogType {
    /// This format is commonly used by system logging utilities
    /// (`/var/log/messages`, `/var/log/debug`, etc...), and consists of the
    /// abbreviated month name, followed by the number of the day of the month,
    /// without a trailing 0.
    ///
    /// The extreme disadvantage of this format is that it gives no information
    /// about the year those logs were written. Provided with no information,
    /// we assume that the year those logs were taken is the current one
    /// (even in cases where that would give dates in the future, although that
    /// could be a check implemented in future versions).
    Syslog,
    /// Some logging systems will have log lines begin with a calendar date
    /// following ISO 8601 standards (`YYYY-MM-DD`). For me, `fail2ban` is the
    /// main reason I need this format.
    Iso,
    /// Apache follows a particular standard for its log formats, where lines
    /// begin with a ton of information (IP of the client, codes, etc).
    /// The date is present, but in the format `[%d/%b/%Y`..., for example
    /// `[17/May/2020`.
    ApacheAccess,
    /// Since apache couldn't follow one standard, error logs follow another
    /// format.
    /// This one puts the date at the beginning of the lines, but sadly
    /// separates the various items needed to build a day :
    /// ```txt
    /// [Sat May 16 02:07:16.656808 2020] ...
    /// ```
    /// 
    /// This isn't too much of an issue since [`NaiveDate`] can be built
    /// with the rest of that information we don't need.
    ApacheError,
    /// Grafana already categorizes its logs by date of rotation, but a file
    /// can and will sometimes contain multiple days.
    ///
    /// Every line begins with the precise time formatted according to ISO 8601,
    /// prefixed with `t=`, and followed by `lvl=` showing the log level.
    /// ```
    /// t=2020-05-12T18:14:21+0200 lvl=...
    /// ```
    /// So we can analyze those easily.
    GrafanaLogs
}

/// File processing data structure
///
/// This data structure processes a file at a given location with
/// the objective to output multiple files based on a pre-computed
/// output path.
///
/// # Example
///
/// This is how a `FileProcessor` is used in `LogBoop`.
///
/// ```
/// // Building file processor
/// // We need to have two PathBuf, and here `path` isn't one
/// let mut proco = FileProcessor::new(path.to_path_buf(), outroot);
/// // Second, we need to determine the type of the file we process
/// proco.determine_type()?;
/// // It could very well fail, and it could find no compatible type
/// // Meaning that it'll keep the `logtype` field at `None`,
/// // Then, process if there was a compatible log type found.
/// proco.process()
/// // That method returns an io Result, so you can just return from it
/// ```
/// # Creating one
///
/// A `FileProcessor` is created from the combination of an input path
/// (a [`PathBuf`] pointing to the file being processed) and an output
/// path (another [`PathBuf`] giving the root path to which dates will
/// be added while extracting).
struct FileProcessor {
    /// An owned path to the file being processed
    path: PathBuf,
    /// An owned path to the root path of the output data
    outroot: PathBuf,
    /// An optional log type, if one has been determined
    logtype: Option<LogType>
}

impl FileProcessor {
    /// Constructor for the `FileProcessor`
    fn new(path: PathBuf, outroot: PathBuf) -> FileProcessor {
        FileProcessor { path, outroot, logtype: None }
    }

    /// Determine a type for the current file.
    ///
    /// This method opens the file, reads the first line, and tries to
    /// match it with known types using regular expressions.
    ///
    /// # Errors
    ///
    /// Of course, if any I/O operation fails for some reason (file not
    /// existing, no data in the file, permissions, disk failure, etc...),
    /// `determine_type` will throw an I/O Error. Otherwise, it will return
    /// the `Ok` variant of [`std::io::Result<()>`](std::io::Result).
    fn determine_type(&mut self) -> std::io::Result<()> {
        // We need to open the file and get the first line
        let fptr = File::open(self.path.to_str().unwrap())?;
        let mut bufr = BufReader::new(fptr);
        let mut first_line = String::new();

        // Read the first line
        let _ = bufr.read_line(&mut first_line)?;
        // Match it
        let types = vec![LogType::Syslog,
            LogType::Iso, LogType::ApacheAccess,
            LogType::ApacheError, LogType::GrafanaLogs
        ].into_iter();
        for logtype in types {
            if REGEXES[&logtype].is_match(&first_line) {
                self.logtype = Some(logtype);
                return Ok(());
            }
        }
        Ok(())
    }

    /// Perform the processing, line by line, of the file.
    ///
    /// Once the log type is determined, process the file and
    /// write the output files. We also create the necessary output folders
    /// recursively to write our output.
    /// 
    /// Every line is read, matched with the regex, and a method
    /// determines the date using a Date format string (using `determine_date`).
    ///
    /// If everything is successful, the file is deleted.
    ///
    /// # Errors
    ///
    /// If at any point, any I/O operation fails, the error will flow upwards.
    /// Otherwise, the `Ok` variant of a
    /// [`std::io::Result<()>`](std::io::Result).
    fn process(&mut self) -> std::io::Result<()> {
        // Redo the opening procedure, and read line by line
        print!("{} ", self.path.to_str().unwrap());
        std::io::stdout().flush()?;
        if self.logtype.is_none() {
            println!("?");
            return Ok(());
        }
        let prepared_path_out = self.outroot.to_str().unwrap();
        // Ensure that the directory containing that output exists
        create_dir_all(self.outroot.parent().unwrap())?;
        let logtype = self.logtype.as_ref().unwrap();
        let fptr = File::open(self.path.to_str().unwrap())?;
        let bufr = BufReader::new(fptr);
        bufr.lines()
            .filter_map(|line|
                        line.map(|l|
                              (determine_date(&logtype, &l), l)
                        ).ok()
            )
            .try_fold(
                (String::new(), None),
                |(mut odp, mut nbufw), (date_postfix, line)| -> std::io::Result<_> {
                    if let Some(date_postfix) = date_postfix {
                        if date_postfix != odp {
                            let new_fname = format!("{}-{}",
                                                    prepared_path_out,
                                                    date_postfix);
                            nbufw = Some(BufWriter::new(
                                OpenOptions::new()
                                    .append(true)
                                    .create(true)
                                    .open(new_fname)?));
                            odp = date_postfix;
                        }
                    }
                    // Write
                    if let Some(ref mut writer) = nbufw {
                        writeln!(writer, "{}", line)?;
                    }
                    Ok((odp, nbufw))
                }
            )?;
        println!("\u{2713} -> {}", prepared_path_out);
        remove_file(&self.path)
    }
}

/// Process exactly one file using the `FileProcessor` structure
///
/// # Arguments
///
/// This method takes a [`&Path`](std::path::Path) and a
/// [`PathBuf`](std::path::PathBuf). The former is a reference to the file
/// path that will be turned into a `PathBuf` for the `FileProcessor`. The
/// latter is simply the output path prefix for the processor.
///
/// # Errors
///
/// If anything fails during processing, the error will flow upwards.
pub fn one_file(path: &Path, outroot: PathBuf) -> std::io::Result<()> {
    // Building file processor
    let mut proco = FileProcessor::new(path.to_path_buf(), outroot);
    proco.determine_type()?;
    proco.process()
}

/// Recursively process all of the files in an input directory
///
/// # Arguments
/// This method takes two arguments :
///  - a [`&Path`](std::path::Path) which is the root of the input directory
///  - another [`&Path`](std::path::Path) which is the root of the output
///  directory
///
/// # Behaviour
///
/// When given a path, this method recursively iterates all files in the
/// folder (and at this point in the program it must be a folder),
/// checks their extension (if any) with a regex matching for digits (in the
/// style of ".1", ".3", ".12" and so on). When a file matching this regex
/// is found, the [`one_file`] method is called.
///
/// # Errors
/// This method will return a `std::io::Result<()>`, and can be invoked
/// with the `?` syntax sugar. When an internal error occurs
/// (with [`one_file`]), that error will flow upwards.
///
/// # Example
/// This method can be used thusly.
/// ```
/// let my_files_path = Path::new("var/log");
/// let output_path = Path::new("/tmp/processed/var/log");
/// all_files(&my_files_path, &output_path)?;
/// ```
pub fn all_files(inpath: &Path, outpath: &Path) -> std::io::Result<()> {
    WalkDir::new(inpath)
        .into_iter()
        .filter_map(|entry| entry.map(walkdir::DirEntry::into_path).ok())
        .filter(|ent| match ent.extension() {
            Some(ext) => ext
                .to_str()
                .map_or(false, |e| NUMBER_REGEX.is_match(e)),
            None => false
        })
        .try_for_each(|entry| -> std::io::Result<_> {
            match entry.strip_prefix(inpath) {
                Ok(suffix) => {
                    // First, join the outpath root and suffix
                    // Second, remove the extension (i.e. the digit)
                    let base_output_path = outpath.join(suffix)
                        .with_extension("");
                    if let Err(e) = one_file(entry.as_path(), base_output_path) {
                        eprintln!("Error while processing {} : {}",
                                  entry.display(), e);
                    }
                },
                Err(e) => {
                    eprintln!("Error in suffix determination : {}", e);
                }
            }
            Ok(())
        })
}

/// Given a line and assumed log type, determine the date of that log line
///
/// # Arguments
///
/// Determining the date of a line requires :
///  - a reference to a [`LogType`] assumed to be valid for our line
///  - the line as a [`&str`]
///
/// # Return value
///
/// This method returns an [`Option<String>`](Option) with the determined date
/// in [simple ISO 8601 calendar date
/// format](https://en.wikipedia.org/wiki/ISO_8601#Calendar_dates).
/// It literally cannot return anything else. Panicking behaviours are not
/// exactly handled just yet.
///
/// # Behaviour
///
/// Using the same list of regexes used to determine the log type, this method
/// first extracts the exact region matched, which must contain all of the
/// information needed to determine one unique calendar date (except for one
/// case but more on that later).
/// That exact portion is parsed, depending on the type, to build a
/// [`NaiveDate`].
///
/// There is technically a fallback if the parsing fails (for example, logs
/// that have been tampered with contain an impossible date) that assigns
/// the day "0001-01-01" in case of failure.
///
/// Once that [`NaiveDate`] is built, it is converted to the format we want,
/// and returned in the [`Option`].
///
/// ## A note on the `Syslog` format
///
/// The default format used by system logs ([`LogType::Syslog`]) commonly does
/// not indicate the year. This is a huge issue, because we cannot infer an
/// exact date. As such, **we assume that the year of the logs is the current
/// one**, and append it to the portion of the line we extracted before trying
/// to build our [`NaiveDate`].
fn determine_date(lt: &LogType, line: &str) -> Option<String> {
    // Create the moment
    let matched_part = REGEXES[lt].find(line)?;
    let match_start = matched_part.start();
    let match_end = matched_part.end();
    let line = &line[match_start..match_end];
    let iso_8601_fmt: StrftimeItems = StrftimeItems::new("%Y-%m-%d");

    // Depending on the type, parse into a Date
    Some(match lt {
        LogType::Syslog => { 
            // What is the current year?
            let year = chrono::Utc::now().year();
            let line = &format!("{} {}", line, year);
            NaiveDate::parse_from_str(line, "%b %d %Y")
        },
        LogType::Iso => {
            NaiveDate::parse_from_str(line, "%Y-%m-%d")
        },
        LogType::ApacheAccess => {
            NaiveDate::parse_from_str(line, "[%d/%b/%Y:")
        },
        LogType::ApacheError => {
            NaiveDate::parse_from_str(line, "[%a %b %d %H:%M:%s%.6f %Y]")
        },
        LogType::GrafanaLogs => {
            NaiveDate::parse_from_str(line, "t=%Y-%m-%dT%H:%M:%S%z lvl=")
        }
    }.unwrap_or_else(|_| chrono::NaiveDate::from_ymd(0, 1, 1))
        .format_with_items(iso_8601_fmt)
        .to_string())
}

