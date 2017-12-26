use std::cmp::Ordering;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use toml::Value as TomlValue;
use toml::value::Datetime as TomlDateTime;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Index {
    index: Arc<RwLock<Vec<(TomlValue, PathBuf)>>>,
    key: String,
    ty: String,
}
impl Index {
    pub fn create(published_dir: &str, key: &str, ty: &str) -> Index {
        let index = make_index(published_dir, key, ty).unwrap_or_default();
        Index {
            index: Arc::new(RwLock::new(index)),
            key: key.to_owned(),
            ty: ty.to_owned()
        }
    }
}
impl Default for Index {
    fn default() -> Index {
        Index {
            index: Arc::new(RwLock::new(Vec::new())),
            key: String::new(),
            ty: String::new(),
        }
    }
}
impl Deref for Index {
    type Target = RwLock<Vec<(TomlValue, PathBuf)>>;
    fn deref(&self) -> &Self::Target {
        &self.index
    }
}
fn make_index(published_dir: &str, key: &str, ty: &str) -> Option<Vec<(TomlValue, PathBuf)>> {
    info!("Indexing files with key '{}' of type '{}'.", key, ty);
    // Because Windows NT allow non-disk paths, so we have to canonicalize so
    // that the paths of subitems literally start with this prefix.
    let mut unordered = Vec::new();
    for entry in WalkDir::new(&published_dir)
        .into_iter()
        .filter_map(|x| x.ok()) {
        // Seek for `content.md`.
        if entry.file_type().is_file() &&
            entry.file_name() == "content.md" {
            if let Some(parent) = entry.path().parent() {
                info!("Indexing article '{}'...", &parent.to_string_lossy());
                if let Some(val) = get_index_val_for(parent, key) {
                    if val.type_str() != ty {
                        warn!("Article is not indexed: index key of type '{}' was found, but it should be of type '{}'.",
                            ty, val.type_str());
                        continue
                    }
                    unordered.push((val, parent.strip_prefix(&published_dir).unwrap().to_owned()));
                } else {
                    warn!("Article is not indexed: index key is inaccessible.");
                }
            } else {
                error!("Unexpected error accessing parent of: {}",
                    &entry.path().to_string_lossy());
            }
        }
    }
    unordered.sort_by(|&(ref a, _), &(ref b, _)| cast_lt(&a, &b, ty));
    Some(unordered)
}

fn lt_datetime(a: &TomlDateTime, b: &TomlDateTime) -> Ordering {
    let a = ::chrono::DateTime::parse_from_rfc3339(&a.to_string()).unwrap();
    let b = ::chrono::DateTime::parse_from_rfc3339(&b.to_string()).unwrap();
    a.cmp(&b).reverse()
}
fn cast_lt(a: &TomlValue, b: &TomlValue, ty: &str) -> Ordering {
    match ty {
        "string" => a.as_str().unwrap().cmp(b.as_str().unwrap()),
        "integer" => a.as_integer().unwrap().cmp(&b.as_integer().unwrap()),
        "datetime" => lt_datetime(a.as_datetime().unwrap(), b.as_datetime().unwrap()),
        _ => panic!("'{}' cannot be used as key type.", ty),
    }
}
fn get_index_val_for(parent: &Path, key: &str) -> Option<TomlValue> {
    use std::fs::File;
    use std::io::Read;
    // Find `metadata.toml`.
    let mut text = Vec::new();
    let mut file = File::open(path_buf![parent, "metadata.toml"])
        .map_err(|err| error!("Unable to open metadata from '{}': {}",
            parent.to_string_lossy(), err))
        .ok()?;
    file.read_to_end(&mut text)
        .map_err(|err| error!("Unable to read metadata from '{}': {}",
            parent.to_string_lossy(), err))
        .ok()?;
    let toml = ::toml::from_slice::<TomlValue>(&text)
        .map_err(|err| warn!("Unable to serialize content of '{}': {}",
            parent.to_string_lossy(), err))
        .ok()?;
    toml.get(key)
        .map(|x| x.to_owned())
}
