use core::panic;
use std::{
    cmp::Ordering,
    collections::VecDeque,
    ffi::{OsStr, OsString},
    fmt::{format, Write},
    fs::ReadDir,
    io::{Error as IOError, ErrorKind},
    mem::MaybeUninit,
    ops::Range,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Once,
    vec,
};

use chrono::{format::Item, Date, NaiveDate, Utc};
use once_cell::unsync::OnceCell;
use pulldown_cmark::{Event, HeadingLevel};
use thiserror::Error;

pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Returns path to temp. dir
pub fn tmp_dir() -> PathBuf {
    #[cfg(target_family = "unix")]
    {
        PathBuf::from_str("/tmp").unwrap()
    }
    #[cfg(target_family = "windows")]
    {
        let profile = std::env::var("USERPROFILE").expect("%USERPROFILE% not defined");
        PathBuf::from(format!("{}\\AppData\\Local\\Temp", profile))
    }
}

macro_rules! glob_env_var_path {
    ($glob: ident, $env: literal, $depth: literal) => {
        $glob.read(
            std::env::var($env).expect(&($env.to_owned() + " not defined")),
            $depth,
        );
    };
}

pub fn path() -> &'static [PathBuf] {
    static mut PATH: OnceCell<Vec<PathBuf>> = OnceCell::new();
    unsafe {
        PATH.get_or_init(|| {
            let env_path = std::env::var("PATH").ok().map(|path_var| {
                path_var
                    .split(":")
                    .map(|path| PathBuf::from_str(path).expect("invalid path in PATH variable"))
                    .collect::<Vec<PathBuf>>()
            });

            match env_path {
                Some(path) => {
                    return path;
                }
                None => {
                    if cfg!(target_family = "unix") {
                        vec![
                            PathBuf::from("/bin"),
                            PathBuf::from("/sbin"),
                            PathBuf::from("/usr/bin"),
                            PathBuf::from("/usr/sbin"),
                            PathBuf::from("/usr/local/bin"),
                            PathBuf::from("/usr/local/sbin"),
                        ]
                    } else if cfg!(target_family = "windows") {
                        let bin_dir_glob = nym::glob::Glob::new("**/bin/`").unwrap();
                        let prog_files_64 = glob_env_var_path!(bin_dir_glob, "ProgramW6432", 8);
                        let prog_files_32 =
                            glob_env_var_path!(bin_dir_glob, "PROGRAMFILES(X86)", 8);
                        let prog_data = glob_env_var_path!(bin_dir_glob, "PROGRAMDATA", 8);

                        let mut found: Vec<PathBuf> = prog_files_64
                            .chain(prog_files_32)
                            .chain(prog_data)
                            .filter_map(|r| r.ok())
                            .map(|e| e.path().to_path_buf())
                            .collect();

                        found.extend_from_slice(&[
                            PathBuf::from(std::env::var("WINDIR").expect("%WINDIR% not defined")),
                            PathBuf::from(
                                std::env::var("WINDIR").unwrap_unchecked() + "\\System32",
                            ),
                        ]);
                        found
                    } else {
                        vec![]
                    }
                }
            }
        })
        .as_ref()
    }
}

/// Returns `Some(PathBuf)` to executable if system `PATH` contains requested program.
///
/// `.exe` file extension is automatically set on Windows.
///
/// Always false on WASM.
pub fn program_path(program_name: impl AsRef<str>) -> Option<PathBuf> {
    if cfg!(not(target_arch = "wasm32")) {
        let path = path();
        for p in path.iter() {
            let test_path = p.join(program_name.as_ref());
            #[cfg(target_os = "windows")]
            test_path.set_extension(".exe");
            if test_path.exists() {
                return Some(test_path);
            }
        }
    }

    return None;
}

pub trait WriteEvent<'a> {
    fn write_event(&mut self, event: &Event<'a>) -> std::fmt::Result;
}

impl<'a> WriteEvent<'a> for String {
    fn write_event(&mut self, event: &Event<'a>) -> std::fmt::Result {
        match event {
            Event::Text(text) => self.write_str(html_escape::encode_safe(text.as_ref()).as_ref()),
            Event::Code(code) => self.write_str(html_escape::encode_safe(code.as_ref()).as_ref()),
            Event::Html(html) => self.write_str(html_escape::encode_safe(html.as_ref()).as_ref()),
            _ => return Err(std::fmt::Error),
        };
        Ok(())
    }
}

pub(crate) enum MinMax<T: PartialOrd> {
    Empty,
    One(T),
    Complete { min: T, max: T },
}

impl<T: PartialOrd> MinMax<T> {
    pub fn new() -> MinMax<T> {
        MinMax::Empty
    }

    pub fn update(self, value: T) -> MinMax<T> {
        match self {
            MinMax::Empty => todo!(),
            MinMax::One(prev) => match prev.partial_cmp(&value) {
                Some(Ordering::Greater) | Some(Ordering::Equal) => MinMax::Complete {
                    min: value,
                    max: prev,
                },
                Some(Ordering::Less) => MinMax::Complete {
                    min: prev,
                    max: value,
                },
                None => MinMax::One(prev),
            },
            MinMax::Complete { min, max } => {
                if let Some(Ordering::Greater) = min.partial_cmp(&value) {
                    MinMax::Complete { min: value, max }
                } else if let Some(Ordering::Less) = max.partial_cmp(&value) {
                    MinMax::Complete { min, max: value }
                } else {
                    MinMax::Complete { min, max }
                }
            }
        }
    }

    pub fn min(&self) -> Option<&T> {
        match self {
            MinMax::Empty => None,
            MinMax::One(it) => Some(it),
            MinMax::Complete { min, .. } => Some(min),
        }
    }

    pub fn to_min(self) -> Option<T> {
        match self {
            MinMax::Empty => None,
            MinMax::One(it) => Some(it),
            MinMax::Complete { min, .. } => Some(min),
        }
    }

    pub fn max(&self) -> Option<&T> {
        match self {
            MinMax::Empty => None,
            MinMax::One(it) => Some(it),
            MinMax::Complete { max, .. } => Some(max),
        }
    }

    pub fn to_max(self) -> Option<T> {
        match self {
            MinMax::Empty => None,
            MinMax::One(it) => Some(it),
            MinMax::Complete { max, .. } => Some(max),
        }
    }
}

impl<T: PartialOrd + Ord> MinMax<T> {
    pub fn new_complete(a: T, b: T) -> MinMax<T> {
        match a.cmp(&b) {
            Ordering::Less | Ordering::Equal => MinMax::Complete { min: a, max: b },
            Ordering::Greater => MinMax::Complete { min: b, max: a },
        }
    }
}

impl<T: PartialOrd, I: Iterator<Item = T>> From<I> for MinMax<T> {
    fn from(mut value: I) -> Self {
        value
            .into_iter()
            .fold(MinMax::Empty, |acc, it| acc.update(it))
    }
}
