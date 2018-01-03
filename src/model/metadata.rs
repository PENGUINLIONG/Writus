use std::io::{Read, Write, BufReader, BufWriter};
use std::fs::{File, OpenOptions};
use writium::prelude::*;
use writium_cache::CacheSource;
use serde_json::Value as JsonValue;

pub struct MetadataSource {
    dir: String,
}
impl MetadataSource {
    pub fn new(dir: &str) -> MetadataSource {
        MetadataSource {
            dir: dir.to_string(),
        }
    }
    fn open_metadata(&self, id: &str, read: bool) -> ::std::io::Result<File> {
        info!("Try openning file of ID: {}", id);
        OpenOptions::new()
            .create(!read)
            .read(read)
            .write(!read)
            .open(path_buf![&self.dir, id, "metadata.json"])
    }
}
impl CacheSource for MetadataSource {
    type Value = JsonValue;
    fn load(&self, id:&str, create: bool) -> Result<Self::Value> {
        fn _load(file: ::std::io::Result<File>) -> Option<JsonValue> {
            let file = file.ok()?;
            let mut text = Vec::new();
            BufReader::new(file).read_to_end(&mut text).ok()?;
            let json = ::serde_json::from_slice(&text);
            if json.is_err() {
                println!("{}", json.unwrap_err());
                return None
            }
            let json = json.ok()?;
            Some(json)
        }
        if let Some(json) = _load(self.open_metadata(id, true)) {
            if json.is_object() {
               Ok(json)
            } else if create {
                warn!("Metadata in '{}' should be an object but create flag was set. A new value will replace the invalid data.", id);
               Ok(::serde_json::from_str("{}").unwrap())
            } else {
               Err(Error::internal("Metadata should be an object."))
            }
        } else {
            if create {
               Ok(::serde_json::from_str("{}").unwrap())
            } else {
               Err(Error::internal("Unable to load metadata."))
            }
        }
    }
    fn unload(&self, id: &str, val: &Self::Value) -> Result<()> {
        if let Err(_) = self.open_metadata(id, false)
            .and_then(|file| {
                let json = ::serde_json::to_string_pretty(&val).unwrap();
                BufWriter::new(file).write_all(json.as_bytes())
            }) {
           Err(Error::internal("Unable to write to metadata file. New data is not written."))
        } else {
           Ok(())
        }
    }
    fn remove(&self, id: &str) -> Result<()> {
        use std::fs::remove_file;
        if let Err(_) = remove_file(path_buf![&self.dir, id, "metadata.json"]) {
           Err(Error::internal("Unable to remove metadata file."))
        } else {
           Ok(())
        }
    }
}
