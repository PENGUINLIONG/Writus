pub mod post;
pub mod metadata;
pub mod comment;

pub use self::post::PostSource;
pub use self::metadata::MetadataSource;
pub use self::comment::CommentSource;

use std::io::{BufReader, BufWriter};
use std::fs::{create_dir_all, remove_file, read_dir, remove_dir};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use writium::prelude::*;

const ERR_ACCESS: &str = "Unable to access local resource.";
const ERR_BUILD_DIR: &str = "Unable to build ancestor directory.";

/// `LocalFile` is used to access a file on local storage. Some of the files
/// have fixed names due to Writus design.
struct FileAccessor {
    dir: PathBuf,
    fixed_file_name: Option<&'static str>,
}
impl FileAccessor {
    pub fn new(dir: &str) -> FileAccessor {
        FileAccessor {
            dir: Path::new(dir).to_owned(),
            fixed_file_name: None,
        }
    }
    pub fn with_fixed_file_name(dir: &str, file: &'static str) -> FileAccessor {
        FileAccessor {
            dir: Path::new(dir).to_owned(),
            fixed_file_name: Some(file),
        }
    }

    #[inline]
    fn make_path(&self, id: &str) -> PathBuf {
        let mut path = self.dir.clone();
        // Clean ID, any leading '/' will lead operations to seek for files from
        // the root.
        path.push(clean_id(id));
        if let Some(file_name) = self.fixed_file_name {
            path.push(file_name);
        }
        path
    }

    pub fn read(&self, id: &str) -> Result<BufReader<File>> {
        let path = self.make_path(id);
        OpenOptions::new()
            .read(true)
            .open(&path)
            .map(|file| BufReader::new(file))
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
    }
    pub fn write(&self, id: &str) -> Result<BufWriter<File>> {
        let path = self.make_path(id);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                create_dir_all(&parent)
                    .map_err(|err| {
                        Error::internal(ERR_BUILD_DIR).with_cause(err)
                    })?
            }
        }
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .map(|file| BufWriter::new(file))
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
    }
    pub fn remove(&self, id: &str) -> Result<()> {
        let path_buf = self.make_path(id);
        let mut path: &Path = &path_buf;
        if !path.exists() {
            info!("File '{}' does not exist, so removal is ignored.",
                path.to_string_lossy());
            return Ok(())
        }
        remove_file(path)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        loop {
            path = match path.parent() {
                Some(parent) => parent,
                None => break,
            };
            match read_dir(path) {
                Err(e) => {
                    warn!("Unable to check if directory '{}' is empty: {}",
                        path.to_string_lossy(), e);
                    break
                },
                Ok(mut rd) => if rd.next().is_some() {
                    // Stop removing directories if they are not empty.
                    break
                },
            }
            match remove_dir(path) {
                Ok(_) => info!("Removed empty directory: {}",
                    path.to_string_lossy()),
                Err(err) => warn!("Unable to remove empty directory '{}': {}",
                    path.to_string_lossy(), err),
            }
        }
        return Ok(());
    }
}

#[inline]
fn clean_id<'a>(id: &'a str) -> &'a str {
    let pos_non_slash = id.bytes()
        .position(|x| x != b'/')
        .unwrap_or(0);
    &id[..pos_non_slash]
}
