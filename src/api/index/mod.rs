use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, RwLock};
use serde_json::Value as JsonValue;
use walkdir::WalkDir;

mod index_map;
use self::index_map::{DateTime, DumbIndexCollection,
    DefaultIndexCollection};
pub use self::index_map::IndexCollection;

fn mk_idx(key: &str, mut col: Box<IndexCollection>, dir: Option<&str>)
    -> Index {
    Index {
        index: {
            if let Some(dir) = dir {
                make_index(dir, key, &mut *col);
            }
            Arc::new(RwLock::new(col))
        },
        key: key.to_owned(),
    }
}
#[derive(Clone)]
pub struct Index {
    index: Arc<RwLock<Box<IndexCollection>>>,
    key: String,
}
impl Index {
    /// Make a new `Index` with given index collection and index key.
    pub fn with_index_collection<T>(key: &str, col: T, dir: Option<&str>)
        -> Index where T: 'static + IndexCollection {
        mk_idx(key, Box::new(col), dir)
    }
    /// Make a new `Index` with given index key and corresponding default index
    /// collection. If `dir` has a value, index will be generated from local
    /// storage, searching for articles in that directory and its subdirectory.
    pub fn new(key: &str, mut ty: &str, dir: Option<&str>) -> Index {
        let ascending = if ty.starts_with('+') {
            // Prefixed by '+', the index order is ascending. Larger value will
            // be placed at the back.
            ty = &ty[1..];
            true
        } else if ty.starts_with('-') {
            // Prefixed by '-', the index order is descending, Larger value will
            // be placed at the front.
            ty = &ty[1..];
            false
        } else {
            // Prefixed by nothing, the index order is by default ascending.
            true
        };
        let col: Box<IndexCollection> = match ty {
            "string" => Box::new(
                DefaultIndexCollection::<String>::new(ascending)
            ),
            "integer" => Box::new(
                DefaultIndexCollection::<i64>::new(ascending)
            ),
            "datetime" => Box::new(
                DefaultIndexCollection::<DateTime>::new(ascending)
            ),
            _ => panic!("Index key type should be one of `datetime`, `string`, \
                or `integer`."),
        };
        mk_idx(key, col, dir)
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
                index.insert(&path, &val);
            } else {
                warn!("Article is not indexed.");
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
    // Find `metadata.json`.
    let mut text = Vec::new();
    let mut file = File::open(path_buf![parent, "metadata.json"])
        .map_err(|err| error!("Unable to open metadata from '{}': {}",
            parent.to_string_lossy(), err))
        .ok()?;
    file.read_to_end(&mut text)
        .map_err(|err| error!("Unable to read metadata from '{}': {}",
            parent.to_string_lossy(), err))
        .ok()?;
    let json = ::serde_json::from_slice::<JsonValue>(&text)
        .map_err(|err| warn!("Unable to serialize content of '{}': {}",
            parent.to_string_lossy(), err))
        .ok()?;
    // If field `noIndex` presents and is set true, ignore the article.
    if let Some(&JsonValue::Bool(true)) = json.get("noIndex") {
        return None
    }
    json.get(key)
        .map(|x| x.to_owned())
}
