use std::{
    cell::OnceCell,
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use nym::glob::Glob;
use serde::{Deserialize, Serialize};

use crate::error::{BlogError, FormatError};

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct IndexData {
    created: Option<DateTime<Utc>>,
    modified: Option<DateTime<Utc>>,
}

impl Default for IndexData {
    fn default() -> Self {
        IndexData {
            created: None,
            modified: None,
        }
    }
}

impl IndexData {
    pub fn new(path: impl AsRef<Path>) -> IndexData {
        let path = path.as_ref();

        let (created, modified) = if let Ok(metadata) = path.metadata() {
            (
                metadata.created().ok().map(|time| DateTime::from(time)),
                metadata.modified().ok().map(|time| DateTime::from(time)),
            )
        } else {
            (None, None)
        };

        /*
        Args: repository: Option<&Repository>
        if let Some(min_max) = repository.map(|it| {
            it.file_history(it.workdir().unwrap_or_else(|| Path::new(".")))
                .into()
        }) {
            match min_max {
                MinMax::One(create) => {
                    created = created.map(|it| create.min(it)).or(Some(create));
                }
                MinMax::Complete {
                    min: create,
                    max: modify,
                } => {
                    created = created.map(|it| create.min(it)).or(Some(create));
                    modified = created.map(|it| modify.min(it)).or(Some(modify));
                }
                MinMax::Empty => {}
            }
        }
        */

        IndexData { created, modified }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileIndex {
    files: HashMap<PathBuf, IndexData>,
}

impl Into<HashMap<PathBuf, IndexData>> for FileIndex {
    fn into(self) -> HashMap<PathBuf, IndexData> {
        self.files
    }
}

impl From<HashMap<PathBuf, IndexData>> for FileIndex {
    fn from(files: HashMap<PathBuf, IndexData>) -> Self {
        FileIndex { files }
    }
}

impl FileIndex {
    pub fn new() -> FileIndex {
        FileIndex {
            files: HashMap::with_capacity(256),
        }
    }

    pub fn note(&mut self, file: impl AsRef<Path>) {
        let file = file.as_ref();
        self.files.insert(file.to_path_buf(), IndexData::new(file));
    }

    pub fn get(&self, file: impl AsRef<Path>) -> Option<&IndexData> {
        self.files.get(file.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &IndexData)> + '_ {
        self.files.iter()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Blog {
    #[serde(skip)]
    pub source_dir: PathBuf,

    #[serde(skip)]
    pub file_index: Option<FileIndex>,
}

impl Blog {
    pub fn open(path: impl AsRef<Path>) -> Result<Blog, BlogError> {
        Ok(Blog {
            source_dir: path.as_ref().to_path_buf(),
            file_index: None,
        })
    }

    pub fn sources(&self) -> impl Iterator<Item = nym::glob::Entry> + '_ {
        static mut MD_GLOB: OnceCell<Glob> = OnceCell::new();
        let glob = unsafe { MD_GLOB.get_or_init(|| Glob::new("**/*.md").unwrap()) };

        glob.read(self.source_dir.clone(), 8)
            .filter_map(|it| it.ok())
    }

    pub fn load_target_metadata(&mut self, path: impl AsRef<Path>) -> Result<(), FormatError> {
        let index_path = path.as_ref().join(".index-file");
        if index_path.exists() {
            let reader = BufReader::new(File::open(&index_path)?);
            self.file_index = serde_json::from_reader(reader)?;
        }
        Ok(())
    }

    pub fn write_target_metadata(&self, path: impl AsRef<Path>) -> Result<(), FormatError> {
        if let Some(index) = &self.file_index {
            let index_path = path.as_ref().join(".index-file");
            if index_path.parent().map(|it| it.exists()) == Some(true) {
                let writer = BufWriter::new(File::create(&index_path)?);
                serde_json::to_writer(writer, index)?;
            }
        }
        Ok(())
    }
}
