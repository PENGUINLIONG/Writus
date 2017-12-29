use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, RwLock};
use serde_json::Value as JsonValue;
use walkdir::WalkDir;

mod index_map;
use self::index_map::{DateTime, DumbIndexCollection,
    DefaultIndexCollection};
pub use self::index_map::IndexCollection;

#[derive(Clone)]
pub struct Index {
    index: Arc<RwLock<Box<IndexCollection>>>,
    key: String,
}
impl Index {
    /// Make a new `Index` with given index collection and index key.
    pub fn new<T>(col: T, key: &str) -> Index
        where T: 'static + IndexCollection {
        Index {
            index: Arc::new(RwLock::new(Box::new(col))),
            key: key.to_owned(),
        }
    }
    /// Generate index from local storage.
    pub fn gen(dir: &str, key: &str, ty: &str) -> Index {
        fn create_str(dir: &str, key: &str) -> Arc<RwLock<Box<IndexCollection>>> {
            let mut col = DefaultIndexCollection::<String>::new();
            make_index(dir, key, &mut col);
            Arc::new(RwLock::new(Box::new(col)))
        }
        fn create_i(dir: &str, key: &str) -> Arc<RwLock<Box<IndexCollection>>> {
            let mut col = DefaultIndexCollection::<i64>::new();
            make_index(dir, key, &mut col);
            Arc::new(RwLock::new(Box::new(col)))
        }
        fn create_dt(dir: &str, key: &str) -> Arc<RwLock<Box<IndexCollection>>> {
            let mut col = DefaultIndexCollection::<DateTime>::new();
            make_index(dir, key, &mut col);
            Arc::new(RwLock::new(Box::new(col)))
        }
        Index {
            index: match ty {
                "string" => create_str(dir, key),
                "integer" => create_i(dir, key),
                "datetime" => create_dt(dir, key),
                _ => panic!("Index key type should be one of `datetime`, `string`, or `integer`."),
            },
            key: key.to_owned(),
        }
    }
    /// Get the index key of the current index.
    pub fn index_key(&self) -> &String {
        &self.key
    }
}
impl Default for Index {
    /// Make a `Index` that do literally nothing.
    fn default() -> Index {
        Index {
            index: Arc::new(RwLock::new(Box::new(DumbIndexCollection::new()))),
            key: String::new(),
        }
    }
}
impl Deref for Index {
    type Target = RwLock<Box<IndexCollection>>;
    fn deref(&self) -> &Self::Target {
        &*self.index
    }
}
fn make_index(dir: &str, key: &str, index: &mut IndexCollection) {
    info!("Indexing files with key '{}'.", key);
    for entry in WalkDir::new(&dir)
        .into_iter()
        .filter_map(|x| x.ok()) {
        // Seek for `content.md`.
        if !entry.file_type().is_file() ||
            entry.file_name() != "content.md" {
            continue
        }
        if let Some(parent) = entry.path().parent() {
            info!("Indexing article '{}'...", &parent.to_string_lossy());
            if let Some(val) = get_index_val_for(parent, key) {
                let path = parent.strip_prefix(&dir).unwrap()
                    .to_string_lossy()
                    .to_string();
                index.insert(path, &val);
            } else {
                warn!("Article is not indexed: index key is absent.");
            }
        } else {
            error!("Unexpected error accessing parent of: {}",
                &entry.path().to_string_lossy());
        }
    }
}

fn get_index_val_for(parent: &Path, key: &str) -> Option<JsonValue> {
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
    let json = ::toml::from_slice::<JsonValue>(&text)
        .map_err(|err| warn!("Unable to serialize content of '{}': {}",
            parent.to_string_lossy(), err))
        .ok()?;
    json.get(key)
        .map(|x| x.to_owned())
}
