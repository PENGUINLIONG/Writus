use std::sync::Arc;
use serde_json::Value as JsonValue;
use writium::prelude::*;
use self::header::ContentType;
use writium_auth::{Authority, DumbAuthority};
use writium_cache::{Cache, DumbCacheSource};
use super::Index;

mod source;
#[cfg(test)]
mod tests;

use self::source::DefaultSource;

const ERR_MISSING_CONTENT_TYPE: &'static str = "Content type should be denoted for verification use.";
const ERR_JSON: &'static str = "Invalid JSON.";

pub struct MetadataApi {
    cache: Cache<JsonValue>,
    auth: Arc<Authority<Privilege=()>>,
    index: Index,
}
impl MetadataApi {
    pub fn new() -> MetadataApi {
        MetadataApi {
            cache: Cache::new(0, DumbCacheSource::new()),
            auth: Arc::new(DumbAuthority::new()),
            index: Index::default(),
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
    pub fn set_index(&mut self, index: Index) {
        self.index = index;
    }

    fn patch_cache<F>(&self, req: &mut Request, id: &str, f: F) -> ApiResult
        where F: 'static + FnOnce(JsonValue) {
        use writium::hyper::header::ContentType;

        self.auth.authorize((), &req)?;
        
        let ctype = req.header::<ContentType>()
            .ok_or(Error::bad_request(ERR_MISSING_CONTENT_TYPE))?;
        if ctype.0.type_() != "application" || ctype.0.subtype() != "json" {
            return Err(Error::new(StatusCode::UnsupportedMediaType, ERR_JSON))
        }

        let json = ::serde_json::from_slice::<JsonValue>(req.body())
            .map_err(|err| Error::bad_request(ERR_JSON).with_cause(err))?;
        if json.is_object() {
            // If index key value is changed, update the index.
            if let Some(ref key) = json.get(self.index.index_key()) {
                self.index.write().unwrap().insert(&id, key);
            }
            Ok(json)
        } else {
            Err(Error::bad_request(ERR_JSON))
        }
        .map(|json| {
            f(json);
            Response::new()
        })
    }

    /// DELETE `metadata/<path..>?keys=<key>`
    /// DELETE `metadata/<path..>`
    fn delete(&self, req: &mut Request) -> ApiResult {
        #[derive(Deserialize)]
        struct Param {
            pub keys: Option<Vec<String>>,
        }
        self.auth.authorize((), &req)?;

        let id = req.path_segs().join("/");
        let param = req.to_param::<Param>()?;
        if let Some(keys) = param.keys {
            // If the index key is removed, remove it from index.
            if keys.contains(self.index.index_key()) {
                self.index.write().unwrap().remove(&id);
            }
            self.cache.get(&id)
                .map(|cache| {
                    let mut guard = cache.write().unwrap();
                    let obj_ref = guard.as_object_mut().unwrap();
                    for key in keys {
                        obj_ref.remove(&key);
                    }
                })
        } else {
            // All metadata are removed, remove it from index.
            self.index.write().unwrap().remove(&id);
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
            pub keys: Option<Vec<String>>,
        }
        let cache = self.cache.get(&id)?;
        let param = req.to_param::<Param>()?;

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
        let id = req.path_segs().join("/");
        let cache = self.cache.create(&id)?;
        self.patch_cache(req, &id, move |json| {
            let mut guard = cache.write().unwrap();
            *guard = json;
        })
    }
    /// PATCH `metadata/<path..>`
    fn patch(&self, req: &mut Request) -> ApiResult {
        let id = req.path_segs().join("/");
        let cache = self.cache.get(&id)?;
        self.patch_cache(req, &id, move |json| {
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
