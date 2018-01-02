use std::sync::Arc;
use hyper::header::ContentType;
use writium::prelude::*;
use writium_auth::Authority;
use writium_cache::Cache;
use super::index::Index;

const ERR_MIME: &'static str = "Only data of type 'text/markdown' is accepted.";

const DEFAULT_ENTRIES_PER_REQUEST: u64 = 5;

mod source;
#[cfg(test)]
mod tests;

use self::source::DefaultSource;

pub struct PostApi {
    auth: Arc<Authority<Privilege=()>>,
    cache: Cache<String>,
    index: Index,
    entries_per_request: u64,
}

impl PostApi {
    pub fn new() -> PostApi {
        PostApi {
            auth: Arc::new(::writium_auth::DumbAuthority::new()),
            cache: Cache::new(0, ::writium_cache::DumbCacheSource::new()),
            index: Index::default(),
            entries_per_request: DEFAULT_ENTRIES_PER_REQUEST,
        }
    }
    pub fn set_cache_default(&mut self, dir: &str) {
        self.cache = Cache::new(10, DefaultSource::new(dir));
    } 
    pub fn set_cache(&mut self, cache: Cache<String>) {
        self.cache = cache;
    }
    pub fn set_auth(&mut self, auth: Arc<Authority<Privilege=()>>) {
        self.auth = auth;
    }
    pub fn set_entries_per_request(&mut self, entries_per_request: u64) {
        self.entries_per_request = entries_per_request;
    }
    pub fn set_index(&mut self, index: Index) {
        self.index = index;
    }
    pub fn clone_cache(&self) -> Cache<String> {
        self.cache.clone()
    }

    fn get_content(&self, req: &mut Request) -> ApiResult {
        let id = req.path_segs().join("/");
        let cache = self.cache.get(&id)?;
        let text = cache.read().unwrap();
        let text_ref: &[u8] = text.as_ref();
        let res = Response::new()
            .with_header(ContentType(
                "text/markdown; charset=UTF-8".parse().unwrap()))
            .with_body(text_ref);
        Ok(res)
    }
    fn get_index(&self, req: &mut Request) -> ApiResult {
        #[derive(Deserialize)]
        struct Param {
            /// The index of the first article to be included.
            from: Option<usize>,
        }
        let param = req.to_param::<Param>()?;
        let from = param.from.unwrap_or(0);
        let guard = self.index.read().unwrap();
        let entries = guard.get_range(from, self.entries_per_request as usize);
        Response::new()
            .with_header(ContentType(
                "application/json; charset=UTF-8".parse().unwrap()))
            .with_json(&entries)
    }
    /// `/v1/posts{/path..}?{digest}{raw}`
    /// `/v1/posts?{from}`
    fn get(&self, req: &mut Request) -> ApiResult {
        if req.path_segs().len() == 0 {
            self.get_index(req)
        } else {
            self.get_content(req)
        }
    }
    /// `/v1/posts{/path..}`
    fn put(&self, req: &mut Request) -> ApiResult {
        self.auth.authorize((), &req)?;

        // Check content type. A valid request can only contain `text/markdown`.
        let mime = req.header::<ContentType>()
            .ok_or(Error::bad_request("Content type not given."))?;
        if mime.0.type_() != "text" || mime.0.subtype() != "markdown" {
            return Err(Error::new(StatusCode::UnsupportedMediaType, ERR_MIME))
        }

        let id = req.path_segs().join("/");
        self.cache.get(&id)
            .or(self.cache.create(&id))
            .and_then(|cache| Ok(*cache.write().unwrap() = req.to_str()?.to_owned()))
            .map(|_| Response::new())
    }
    /// `/v1/posts{/path..}`
    fn delete(&self, req: &mut Request) -> ApiResult {
        self.auth.authorize((), &req)?;
        
        let id = req.path_segs().join("/");
        self.cache.remove(&id)
            .map(|_| Response::new())
    }
}
impl Api for PostApi {
    fn name(&self) -> &[&str] {
        &["posts"]
    }

    fn route(&self, req: &mut Request) -> ApiResult {
        use self::header::Allow;
        use self::Method::*;
        match req.method() {
            Options => Ok(Response::new()
                .with_header(Allow(vec![Options, Get, Put, Delete]))),
            Get => self.get(req),
            Put => self.put(req),
            Delete => self.delete(req),
            _ => Err(Error::method_not_allowed()),
        }
    }
}
