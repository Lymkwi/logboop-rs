/// Processing module
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
    static ref REGEXES: HashMap<LogType, Regex> = vec![
        (LogType::Syslog, Regex::new(r"^(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) ([012 ]\d|3[01])").unwrap()),
        (LogType::Iso, Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap()),
        (LogType::ApacheAccess, Regex::new(r"\[\d{2}/(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)/\d{4}:").unwrap()),
        (LogType::ApacheError, Regex::new(r"\[(Mon|Tue|Wed|Thu|Fri|Sat|Sun) (Jan|Feb||Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) \d{2} \d{2}:\d{2}:\d{2}.\d{6} \d{4}]").unwrap()),
        (LogType::GrafanaLogs, Regex::new(r"^t=\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\+|-)\d{4} lvl=").unwrap())
    ]
    .into_iter().collect::<HashMap<LogType, Regex>>();
    static ref NUMBER_REGEX: Regex = Regex::new(r"^\d+$").unwrap();
}

#[derive(std::hash::Hash, std::cmp::Eq, std::cmp::PartialEq, std::fmt::Debug)]
enum LogType {
    Syslog,
    Iso,
    ApacheAccess,
    ApacheError,
    GrafanaLogs
}

struct FileProcessor {
    path: PathBuf,
    outroot: PathBuf,
    logtype: Option<LogType>
}

impl FileProcessor {
    fn new(path: PathBuf, outroot: PathBuf) -> FileProcessor {
        FileProcessor { path, outroot, logtype: None }
    }

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
        // Variables that are subject to change
        let mut nbufw = None;
        let mut odp = String::new();

        for line in bufr.lines() {
            let line = line.unwrap();
            let date_postfix = determine_date(&logtype, &line);
            if let Some(date_postfix) = date_postfix {
                if date_postfix != odp {
                    let new_fname = format!("{}-{}",
                                            prepared_path_out, date_postfix);
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
        }
        println!("\u{2713} -> {}", prepared_path_out);
        self.delete()?;
        Ok(())
    }

    fn delete(&self) -> std::io::Result<()> {
        remove_file(&self.path)?;
        Ok(())
    }
}

pub fn one_file(path: &Path, outroot: PathBuf) -> std::io::Result<()> {
    // Building file processor
    let mut proco = FileProcessor::new(path.to_path_buf(), outroot);
    proco.determine_type()?;
    proco.process()?;
    Ok(())
}

pub fn all_files(inpath: &Path, outpath: &Path) -> std::io::Result<()> {
    for entry in WalkDir::new(inpath) {
        let entry = entry?.into_path();
        if let Some(ext) = entry.extension() {
            if NUMBER_REGEX.is_match(ext.to_str().unwrap()) {
                // Strip the prefix and add our outpath
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
                Err(e) => { eprintln!("ERR: {:?}", e); }
                }
            }
        }
    }
    Ok(())
}

fn determine_date(lt: &LogType, line: &str) -> Option<String> {
    // Create the moment
    //println!("{:?} {:?}", lt, line);
    let matched_part = REGEXES[lt].find(line)?;
    let match_start = matched_part.start();
    let match_end = matched_part.end();
    let line = &line[match_start..match_end];

    // Depending on the type, parse into a Date
    let date = match lt {
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
    }.unwrap_or_else(|_| chrono::NaiveDate::from_ymd(0, 1, 1));

    let fmt = StrftimeItems::new("%Y-%m-%d");
    Some(date.format_with_items(fmt).to_string())
}

