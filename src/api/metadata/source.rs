use std::io::{Read, Write, BufReader, BufWriter};
use std::fs::{File, OpenOptions};
use writium::prelude::*;
use writium_cache::CacheSource;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

pub struct DefaultSource {
    dir: String,
}
impl DefaultSource {
    pub fn new(dir: &str) -> DefaultSource {
        DefaultSource {
            dir: dir.to_string(),
        }
    }
    fn open_metadata(&self, id: &str, read: bool) -> ::std::io::Result<File> {
        info!("Try openning file of ID: {}", id);
        OpenOptions::new()
            .create(!read)
            .read(read)
            .write(!read)
            .open(path_buf![&self.dir, id, "metadata.toml"])
    }
}
impl CacheSource for DefaultSource {
    type Value = JsonValue;
    fn load(&self, id:&str, create: bool) -> Result<Self::Value> {
        fn _load(file: ::std::io::Result<File>) -> Option<JsonValue> {
            let file = file.ok()?;
            let mut text = Vec::new();
            BufReader::new(file).read_to_end(&mut text).ok()?;
            let toml = ::toml::from_slice(&text);
            if toml.is_err() {
                println!("{}", toml.unwrap_err());
                return None
            }
            let toml = toml.ok()?;
            Some(toml_to_json(&toml))
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
                let toml = ::toml::to_string_pretty(&json_to_toml(val)).unwrap();
                BufWriter::new(file).write_all(toml.as_bytes())
            }) {
           Err(Error::internal("Unable to write to metadata file. New data is not written."))
        } else {
           Ok(())
        }
    }
    fn remove(&self, id: &str) -> Result<()> {
        use std::fs::remove_file;
        if let Err(_) = remove_file(path_buf![&self.dir, id, "metadata.toml"]) {
           Err(Error::internal("Unable to remove metadata file."))
        } else {
           Ok(())
        }
    }
}

fn toml_to_json(toml: &TomlValue) -> JsonValue {
    use self::TomlValue as Toml;
    use self::JsonValue as Json;

    match toml {
        &Toml::String(ref s) => Json::String(s.clone()),
        &Toml::Integer(i) => Json::Number(i.into()),
        &Toml::Float(f) => {
            let n = if let Some(f) = ::serde_json::Number::from_f64(f) { f }
                else { warn!("Float infinite and nan are not allowed in metadata. Original data is replaced by 0."); 0.into() };
            Json::Number(n)
        }
        &Toml::Boolean(b) => Json::Bool(b),
        &Toml::Array(ref arr) => Json::Array(arr.iter().map(toml_to_json).collect()),
        &Toml::Table(ref table) => Json::Object(table.iter().map(|(k, v)| {
            (k.to_string(), toml_to_json(v))
        }).collect()),
        &Toml::Datetime(ref dt) => Json::String(dt.to_string()),
    }
}
fn json_to_toml(json: &JsonValue) -> TomlValue {
    use self::TomlValue as Toml;
    use self::JsonValue as Json;
    use std::str::FromStr;
    use toml::value::Datetime;

    match json {
        &Json::String(ref s) => if let Ok(d) = Datetime::from_str(&s) {
                Toml::Datetime(d)
            } else {
                Toml::String(s.clone())
            },
        &Json::Number(ref n) => if n.is_i64() {
                Toml::Integer(n.as_i64().unwrap())
            } else if n.is_u64() {
                Toml::Integer(n.as_u64().unwrap() as i64)
            } else {
                Toml::Float(n.as_f64().unwrap())
            },
        &Json::Bool(b) => Toml::Boolean(b),
        &Json::Array(ref arr) => Toml::Array(arr.iter().map(json_to_toml).collect()),
        &Json::Object(ref table) => Toml::Table(table.iter().map(|(k, v)| {
            (k.to_string(), json_to_toml(&v))
        }).collect()),
        &Json::Null => {
            warn!("Found null field. Empty string is inserted.");
            Toml::String(String::new())
        },
    }
}
