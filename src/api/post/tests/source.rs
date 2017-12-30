use std::collections::HashMap;
use std::sync::Mutex;
use writium::prelude::*;
use writium_cache::CacheSource;

pub struct MockSource(Mutex<HashMap<String, String>>);
impl MockSource {
    pub fn new() -> MockSource {
        let mut map = HashMap::new();
        map.insert("foo".to_owned(), super::CONTENT_MARKDOWN.to_owned());
        MockSource(Mutex::new(map))
    }
}
impl CacheSource for MockSource {
    type Value = String;
    fn load(&self, id: &str, create: bool) -> Result<String> {
        if let Some(item) = self.0.lock().unwrap().get(id) {
            println!("Loading {} from source.", id);
            Ok(item.to_owned())
        } else if create {
            println!("Creating {}.", id);
            Ok(String::new())
        } else {
            println!("Cannot load.");
            Err(Error::not_found("x"))
        }
    }
    fn remove(&self, id: &str) -> Result<()> {
        println!("Removing {} from source.", id);
        self.0.lock().unwrap().remove(id).map(|_| println!("Something was removed."));
        Ok(())
    }
}
