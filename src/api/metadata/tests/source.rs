use std::collections::BTreeMap;
use std::sync::Mutex;
use writium_framework::prelude::*;
use writium_cache::CacheSource;
use serde_json::Value as JsonValue;

pub struct MockSource(Mutex<BTreeMap<String, JsonValue>>);
impl MockSource {
    pub fn new() -> MockSource {
        let mut map = ::serde_json::value::Map::new();
        map.insert("key".to_owned(), JsonValue::String("Boom!".to_owned()));
        map.insert("neko".to_owned(), JsonValue::Number(3.into()));
        let mut article_map = BTreeMap::new();
        article_map.insert("foo".to_owned(), JsonValue::Object(map));
        MockSource(Mutex::new(article_map))
    }
}
impl CacheSource for MockSource {
    type Value = JsonValue;
    fn load(&self, id: &str, create: bool) -> Result<JsonValue> {
        if create {
            let map = ::serde_json::value::Map::new();
            Ok(JsonValue::Object(map))
        } else {
            self.0.lock().unwrap().get(id)
                .ok_or(Error::not_found("..?"))
                .map(|x| x.clone())
        }
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.0.lock().unwrap().remove(id);
        Ok(())
    }
}
