use std::sync::Arc;
use serde_json::Value as JsonValue;
use writium::prelude::*;
use self::header::ContentType;
use writium_auth::{Authority, DumbAuthority};
use writium_cache::{Cache, DumbCacheSource};
use super::Index;

#[cfg(test)]
mod tests;

const ERR_MISSING_CONTENT_TYPE: &'static str = "Content type should be denoted for verification use.";
const ERR_JSON: &'static str = "Invalid JSON.";

pub struct MetadataApi {
    cache: Arc<Cache<JsonValue>>,
    auth: Arc<Authority<Privilege=()>>,
    index: Index,
}
impl MetadataApi {
    pub fn new() -> MetadataApi {
        MetadataApi {
            cache: Arc::new(Cache::new(0, DumbCacheSource::new())),
            auth: Arc::new(DumbAuthority::new()),
            index: Index::default(),
        }
    }
    pub fn set_cache(&mut self, cache: Arc<Cache<JsonValue>>) {
        self.cache = cache;
    }
    pub fn set_auth(&mut self, auth: Arc<Authority<Privilege=()>>) {
        self.auth = auth;
    }
    pub fn set_index(&mut self, index: Index) {
        self.index = index;
    }

    fn parse_json(&self, req: &mut Request) -> Result<JsonValue> {
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
            Ok(json)
        } else {
            Err(Error::bad_request(ERR_JSON))
        }
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
            let cache = self.cache.get(&id)?;
            // If the index key is removed, remove it from index.
            if keys.contains(self.index.index_key()) {
                self.index.write().unwrap().remove(&id);
            // If `noIndex` is removed and the key is not removed, add it to
            // index.
            } else if keys.iter().any(|x| x == "noIndex") {
                let guard = cache.read().unwrap();
                // The original value is true, so it's indexing is suppressed
                // before,
                if let Some(&JsonValue::Bool(true)) = guard.get("noIndex") {
                    // There is a index key in metadata.
                    if let Some(key) = guard.get(self.index.index_key()) {
                        self.index.write().unwrap().insert(&id, key);
                    }
                }
            }
            let mut guard = cache.write().unwrap();
            let obj_ref = guard.as_object_mut().unwrap();
            for key in keys {
                obj_ref.remove(&key);
            }
        } else {
            // All metadata are removed, remove it from index.
            self.index.write().unwrap().remove(&id);
            self.cache.remove(&id)?
        }
        Ok(Response::new())
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
        let json = self.parse_json(req)?;
        let mut guard = cache.write().unwrap();
        // If `noIndex` flag is set `true`, remove the article from index and
        // stop from index updating. If it's set to `false` and there is a valid
        // index key stored in metadata, the article should be indexed.
        if let Some(&JsonValue::Bool(true)) = json.get("noIndex") {
            self.index.write().unwrap().remove(&id);
        // Update the index.
        } else if let Some(ref key) = json.get(self.index.index_key()) {
            self.index.write().unwrap().insert(&id, key);
        }
        // Replace metadata with the new one.
        *guard = json;
        Ok(Response::new())
    }
    /// PATCH `metadata/<path..>`
    fn patch(&self, req: &mut Request) -> ApiResult {
        let id = req.path_segs().join("/");
        let cache = self.cache.get(&id)?;
        let json = self.parse_json(req)?;
        let mut guard = cache.write().unwrap();
        let obj = guard.as_object_mut().unwrap();
        for item in json.as_object().unwrap() {
            obj.insert(item.0.to_string(), item.1.clone());
        }
        
        if let Some(&JsonValue::Bool(true)) = json.get("noIndex") {
            self.index.write().unwrap().remove(&id);
        // If index key has been changed, update the index.
        } else if let Some(ref key) = json.get(self.index.index_key()) {
            self.index.write().unwrap().insert(&id, key);
        // No index key update, check if there is already a index key in the
        // unchanged portion of metadata.
        } else if let Some(ref key) = obj.get(self.index.index_key()) {
            self.index.write().unwrap().insert(&id, key);
        }
        Ok(Response::new())
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
