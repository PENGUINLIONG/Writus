use serde_json::Value as JsonValue;

pub type DateTime = ::chrono::DateTime<::chrono::FixedOffset>;

pub trait IndexKeyType: Sized + Send + Sync + Ord {
    fn try_from_json(json: &JsonValue) -> Option<Self>;
}
impl IndexKeyType for i64 {
    fn try_from_json(json: &JsonValue) -> Option<i64> {
        if let Some(ref int) = json.as_i64() {
            return Some(int.to_owned())
        }
        None
    }
}
impl IndexKeyType for String {
    fn try_from_json(json: &JsonValue) -> Option<String> {
        if let Some(ref string) = json.as_str() {
            return Some(string.to_string())
        }
        None
    }
}
impl IndexKeyType for DateTime {
    fn try_from_json(json: &JsonValue) -> Option<DateTime> {
        if let Some(ref rfc3339) = json.as_str() {
            if let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339) {
                return Some(dt)
            }
        }
        None
    }
}

struct IndexItem<T: IndexKeyType> {
    pub key: T,
    pub path: String,
}

pub trait IndexCollection: Send + Sync {
    fn insert(&mut self, path: &str, key: &JsonValue);
    fn get_range(&self, skip: usize, take: usize) -> Vec<String>;
    fn remove(&mut self, path: &str);
    fn len(&self) -> usize;
}

pub struct DefaultIndexCollection<T: IndexKeyType> {
    index: Vec<IndexItem<T>>,
    asc: bool,
}
impl<T: IndexKeyType> DefaultIndexCollection<T> {
    pub fn new(asc: bool) -> DefaultIndexCollection<T> {
        DefaultIndexCollection {
            index: Vec::new(),
            asc: asc,
        }
    }
    fn search_by_key(&self, key: &T) -> usize {
        match if self.asc {
            self.index.binary_search_by(|item| item.key.cmp(key))
        } else {
            self.index.binary_search_by(|item| item.key.cmp(key).reverse())
        } {
            Ok(pos) => pos,
            Err(pos) => pos,
        }
    }
}
impl<T: IndexKeyType> IndexCollection for DefaultIndexCollection<T> {
    fn insert(&mut self, path: &str, key: &JsonValue) {
        let key = if let Some(key) = T::try_from_json(key) {
            key
        } else {
            if let Ok(pos) = self.index.binary_search_by(|item| {
                let str_ref: &str = item.path.as_ref();
                str_ref.cmp(path)
            }) {
                self.index.remove(pos);
                warn!("`{}` is updated with a new key which cannot be parsed into `DateTime`. So it's removed from the index.", path)
            } else {
                warn!("`{}` has a key which cannot be parsed into `DateTime`. So it's not indexed.", path);
            }
            return
        };
        match self.index.iter().position(|item| (&*item.path).eq(path)) {
            Some(pos) => {
                let mut item = self.index.remove(pos);
                let new_pos = self.search_by_key(&key);
                item.key = key;
                self.index.insert(new_pos, item);
                info!("Updated index key for article: {}", path);
            },
            None => {
                let new_pos = self.search_by_key(&key);
                self.index.insert(new_pos,
                IndexItem {
                    key: key,
                    path: path.to_owned(),
                });
                info!("Indexed article: {}", path);
            },
        }
    }
    fn get_range(&self, skip: usize, take: usize) -> Vec<String> {
        self.index.iter()
            .skip(skip)
            .take(take)
            .map(|item| if cfg!(windows) {
                    item.path.replace('\\', "/")
                } else {
                    item.path.to_owned()
                }
            )
            .collect::<Vec<_>>()
    }
    fn remove(&mut self, path: &str) {
        if let Some(pos) = self.index.iter().position(|item| (&*item.path).eq(path)) {
            self.index.remove(pos);
        }
    }
    fn len(&self) -> usize {
        self.index.len()
    }
}

pub struct DumbIndexCollection();
impl DumbIndexCollection {
    pub fn new() -> DumbIndexCollection{
        DumbIndexCollection()
    }
}
impl IndexCollection for DumbIndexCollection {
    fn insert(&mut self, _path: &str, _key: &JsonValue) {}
    fn get_range(&self, _skip: usize, _take: usize) -> Vec<String> { Vec::new() }
    fn remove(&mut self, _path: &str) {}
    fn len(&self) -> usize { 0 }
}
