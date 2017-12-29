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
    fn insert(&mut self, path: String, key: &JsonValue);
    fn get_range(&self, skip: usize, take: usize) -> Vec<String>;
    fn remove(&mut self, path: &str);
}

pub struct DefaultIndexCollection<T: IndexKeyType> {
    index: Vec<IndexItem<T>>,
    reverse: bool,
}
impl<T: IndexKeyType> DefaultIndexCollection<T> {
    pub fn new() -> DefaultIndexCollection<T> {
        DefaultIndexCollection {
            index: Vec::new(),
            reverse: true,
        }
    }
}
impl<T: IndexKeyType> IndexCollection for DefaultIndexCollection<T> {
    fn insert(&mut self, path: String, key: &JsonValue) {
        match if self.reverse {
            self.index.binary_search_by(|item| item.path.cmp(&path).reverse())
        } else {
            self.index.binary_search_by(|item| item.path.cmp(&path))
        } {
            Ok(pos) => if let Some(key) = T::try_from_json(key) {
                self.index[pos] = IndexItem { key: key, path: path.to_owned() };
            } else {
                self.index.remove(pos);
                warn!("`{}` is updated with a new key which cannot be parsed into `DateTime`. So it's removed from the index.", path)
            },
            Err(pos) => if let Some(key) = T::try_from_json(key) {
                self.index.insert(pos, IndexItem { key: key, path: path.to_owned() })
            } else {
                warn!("`{}` has a key which cannot be parsed into `DateTime`. So it's not indexed.", path);
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
        if let Ok(pos) = if self.reverse {
            self.index.binary_search_by(|item| (&*item.path).cmp(path).reverse())
        } else {
            self.index.binary_search_by(|item| (&*item.path).cmp(path))
        } {
            self.index.remove(pos);
        }
    }
}

pub struct DumbIndexCollection();
impl DumbIndexCollection {
    pub fn new() -> DumbIndexCollection{
        DumbIndexCollection()
    }
}
impl IndexCollection for DumbIndexCollection {
    fn insert(&mut self, _path: String, _key: &JsonValue) {}
    fn get_range(&self, _skip: usize, _take: usize) -> Vec<String> { Vec::new() }
    fn remove(&mut self, _path: &str) {}
}
