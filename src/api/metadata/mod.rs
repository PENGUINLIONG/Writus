use std::sync::{Arc, RwLock};
use serde_json::Value as JsonValue;
use writium_framework::prelude::*;
use self::header::ContentType;
use writium_auth::{Authority, DumbAuthority};
use writium_cache::{Cache, DumbCacheSource};

mod source;

use self::source::DefaultSource;

const ERR_MISSING_CONTENT_TYPE: &'static str = "Content type should be denoted for verification use.";
const ERR_JSON: &'static str = "Invalid JSON.";

pub struct MetadataApi {
    cache: Cache<JsonValue>,
    auth: Arc<Authority<Privilege=()>>,
}
impl MetadataApi {
    pub fn new() -> MetadataApi {
        MetadataApi {
            cache: Cache::new(0, DumbCacheSource::new()),
            auth: Arc::new(DumbAuthority::new()),
        }
    }
    pub fn set_cache_default(&mut self, published_dir: &str) {
        self.cache = Cache::new(10, DefaultSource::new(published_dir));
    }
    pub fn set_cache(&mut self, cache: Cache<JsonValue>) {
        self.cache = cache;
    }
    pub fn set_auth(&mut self, auth: Arc<Authority<Privilege=()>>) {
        self.auth = auth;
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
