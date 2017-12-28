use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, RwLock};
use serde_json::Value as JsonValue;
use walkdir::WalkDir;

mod index_map;
use self::index_map::*;

#[derive(Clone)]
pub struct Index {
    index: Arc<RwLock<Box<IndexCollectionBase>>>,
    key: String,
}
impl Index {
    pub fn create(dir: &str, key: &str, ty: &str) -> Index {
        fn create_str(dir: &str, key: &str) -> Arc<RwLock<Box<IndexCollectionBase>>> {
            let mut col = IndexCollection::<String>::new();
            make_index(dir, key, &mut col);
            Arc::new(RwLock::new(Box::new(col)))
        }
        fn create_i(dir: &str, key: &str) -> Arc<RwLock<Box<IndexCollectionBase>>> {
            let mut col = IndexCollection::<i64>::new();
            make_index(dir, key, &mut col);
            Arc::new(RwLock::new(Box::new(col)))
        }
        fn create_dt(dir: &str, key: &str) -> Arc<RwLock<Box<IndexCollectionBase>>> {
            let mut col = IndexCollection::<DateTime>::new();
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
    pub fn index_key(&self) -> &str {
        &self.key
    }
}
impl Default for Index {
    fn default() -> Index {
        Index {
            index: Arc::new(RwLock::new(Box::new(DumbIndexCollection::new()))),
            key: String::new(),
        }
    }
}
impl Deref for Index {
    type Target = RwLock<Box<IndexCollectionBase>>;
    fn deref(&self) -> &Self::Target {
        &*self.index
    }
}
fn make_index(dir: &str, key: &str, index: &mut IndexCollectionBase) {
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
