use std::io::{Read, Write, BufReader, BufWriter};
use std::sync::{Arc, RwLock};
use std::fs::{File, OpenOptions};
use toml::Value as TomlValue;
use serde_json::Value as JsonValue;
use writium_framework::prelude::*;
use self::header::ContentType;
use writium_auth::Authority;
use writium_cache::{Cache, CacheSource};
use auth::SimpleAuthority;

const ERR_MISSING_CONTENT_TYPE: &'static str = "Content type should be denoted for verification use.";
const ERR_JSON: &'static str = "Invalid JSON.";

pub struct MetadataApi {
    cache: Arc<Cache<JsonValue>>,
    auth: Arc<SimpleAuthority>,
}
impl MetadataApi {
    pub fn new(extra: &super::V1Extra) -> MetadataApi {
        MetadataApi {
            cache: Arc::new(Cache::new(
                10,
                MetadataSource::new(&extra.published_dir)
            )),
            auth: extra.auth.clone(),
        }
    }

    fn patch_cache<F>(&self, req: &mut Request, f: F) -> ApiResult
        where F: 'static + FnOnce(Arc<RwLock<JsonValue>>, JsonValue) {
        use writium_framework::hyper::header::ContentType;

        self.auth.authorize((), &req)?;
        
        let id = req.path_segs().join("/");
        let ctype = req.header::<ContentType>()
            .ok_or(Error::bad_request(ERR_MISSING_CONTENT_TYPE))?;
        if ctype.0.type_() != "application" || ctype.0.subtype() != "json" {
            return Err(Error::new(StatusCode::UnsupportedMediaType, ERR_JSON))
        }

        let cache = self.cache.get(&id)
            .or(self.cache.create(&id))?;
        let json = ::serde_json::from_slice::<JsonValue>(req.body())
            .map_err(|err| Error::bad_request(ERR_JSON).with_cause(err))?;
        if json.is_object() {
            Ok(json)
        } else {
            Err(Error::bad_request(ERR_JSON))
        }
        .map(|json| {
            f(cache, json);
            Response::new()
        })
    }

    /// DELETE `metadata/<path..>?keys=<key>`
    /// DELETE `metadata/<path..>`
    fn delete(&self, req: &mut Request) -> ApiResult {
        #[derive(Deserialize)]
        struct Param {
            #[serde(rename="key")]
            pub keys: Option<Vec<String>>,
        }
        self.auth.authorize((), &req)?;

        let id = req.path_segs().join("/");
        let param = req.to_param::<Param>()?;
        if let Some(keys) = param.keys {
            self.cache.get(&id)
                .map(|cache| {
                    let mut guard = cache.write().unwrap();
                    for key in keys {
                        guard.as_object_mut().unwrap().remove(&key);
                    }
                })
        } else {
            self.cache.remove(&id)
        }
        .map(|_| Response::new())
    }

    /// GET `metadata/<path..>?key=<key>`;
    /// GET `metadata/<path..>`;
    fn get(&self, req: &mut Request) -> ApiResult {
        let id = req.path_segs().join("/");
        #[derive(Deserialize)]
        struct Param {
            #[serde(rename="key")]
            pub keys: Option<Vec<String>>,
        }
        let cache = self.cache.get(&id)?;
        let  param = req.to_param::<Param>()?;

        let guard = cache.read().unwrap();
        let json = if param.keys.is_none() {
            guard.clone()
        } else {
            use serde_json::Map;
            let keys = param.keys.unwrap();
            let mut rv = Map::with_capacity(keys.len());
            rv.extend(guard.as_object().unwrap().iter()
                .filter(|&(ref x, _)| keys.contains(x))
                .map(|(x, y)| (x.clone(), y.clone())));
            JsonValue::Object(rv)
        };
        Response::new()
            .with_header(ContentType("application/json; charset=UTF-8".parse().unwrap()))
            .with_json(&json)
    }

    /// PUT `metadata/<path..>`
    fn put(&self, req: &mut Request) -> ApiResult {
        self.patch_cache(req, |cache, json| {
            let mut guard = cache.write().unwrap();
            *guard = json;
        })
    }
    /// PATCH `metadata/<path..>`
    fn patch(&self, req: &mut Request) -> ApiResult {
        self.patch_cache(req, |cache, json| {
            let mut guard = cache.write().unwrap();
            let obj = guard.as_object_mut().unwrap();
            for item in json.as_object().unwrap() {
                obj.insert(item.0.to_string(), item.1.clone());
            }
        })
    }
}
impl Api for MetadataApi {
    fn name(&self) -> &[& str] {
        &["metadata"]
    }

    fn route(&self, req: &mut Request) -> ApiResult {
        use self::header::Allow;
        use self::Method::*;
        match req.method() {
            Options => Ok(Response::new()
                .with_header(Allow(vec![Options, Get, Patch, Put, Delete]))),
            Get => self.get(req),
            Patch => self.patch(req),
            Put => self.put(req),
            Delete => self.delete(req),
            _ => Err(Error::method_not_allowed()),
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

struct MetadataSource {
    dir: String,
}
impl MetadataSource {
    fn new(dir: &str) -> MetadataSource {
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
            .open(path_buf![&self.dir, id, "metadata.toml"])
    }
}
impl CacheSource for MetadataSource {
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
