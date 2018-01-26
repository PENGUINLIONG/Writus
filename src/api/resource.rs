use std::collections::HashMap;
use std::sync::Arc;
use writium::prelude::*;
use self::header::ContentType;
use writium::hyper::mime::Mime;
use writium_auth::{Authority, DumbAuthority};
use writium_cache::{Cache, DumbCacheSource};

const ERR_MISSING_CONTENT_TYPE: &'static str = "Content type should be denoted for verification use.";
const ERR_MIME_NOT_FOUND: &'static str = "No corresponding MIME matches the inquired file type (extension). Maybe the file type is intentionally prevented from being transferred.";
const ERR_MIME_EXT_MISMATCH: &'static str = "Path extension doesn't accord with content type denoted.";

pub struct ResourceApi {
    auth: Arc<Authority<Privilege=()>>,
    cache: Arc<Cache<Vec<u8>>>,
    published_dir: String,
    allowed_exts: HashMap<String, Mime>,
}

impl ResourceApi {
    pub fn new() -> ResourceApi {
        ResourceApi {
            auth: Arc::new(DumbAuthority::new()),
            cache: Arc::new(Cache::new(0, DumbCacheSource::new())),
            published_dir: String::new(),
            allowed_exts: HashMap::new(),
        }
    }
    pub fn set_auth(&mut self, auth: Arc<Authority<Privilege=()>>) {
        self.auth = auth;
    }
    pub fn set_cache(&mut self, cache: Arc<Cache<Vec<u8>>>) {
        self.cache = cache;
    }
    pub fn set_published_dir(&mut self, published_dir: &str) {
        self.published_dir = published_dir.to_owned();
    }
    pub fn set_allowed_exts(&mut self, allowed_exts: HashMap<String, Mime>) {
        self.allowed_exts = allowed_exts;
    }

    fn get(&self, req: &mut Request) -> ApiResult {
        let id = req.path_segs().join("/");
        let ext = id.rsplitn(2, '.').next().unwrap_or_default();
        let mime = self.allowed_exts.get(ext)
            .ok_or(Error::new(StatusCode::UnsupportedMediaType, ERR_MIME_NOT_FOUND))?;

        let cache = self.cache.get(&id)?;
        let guard = cache.read().unwrap();
        let data = (*guard).clone();
        Ok(Response::new()
            .with_header(ContentType(mime.clone()))
            .with_body(data))
    }

    fn put(&self, req: &mut Request) -> ApiResult {
        use self::header::ContentType;
        self.auth.authorize((), &req)?;

        let id = req.path_segs().join("/");
        let ext = id.rsplitn(2, '.').next().unwrap_or_default();
        let mm = self.allowed_exts.get(ext)
            .ok_or(Error::new(StatusCode::UnsupportedMediaType, ERR_MIME_NOT_FOUND))?;
        let mime = req.header::<ContentType>()
            .ok_or(Error::bad_request(ERR_MISSING_CONTENT_TYPE))?;
        if mime.0 != *mm {
            return Err(Error::bad_request(ERR_MIME_EXT_MISMATCH))
        }

        self.cache.get(&id)
            .or(self.cache.create(&id))
            .and_then(|cache| Ok(*cache.write().unwrap() = req.body().to_owned()))
            .map(|_| Response::new())
    }

    fn delete(&self, req: &mut Request) -> ApiResult {
        self.auth.authorize((), &req)?;

        let id = req.path_segs().join("/");
        self.cache.remove(&id)
            .map(|_| Response::new())
    }
}
impl Api for ResourceApi {
    fn name(&self) -> &[&str] {
        &["resources"]
    }

    fn route(&self, req: &mut Request) -> ApiResult {
        use self::header::Allow;
        use self::Method::*;
        match req.method() {
            Options => Ok(Response::new()
                .with_status(StatusCode::Ok)
                .with_header(Allow(vec![Options, Get, Put, Delete]))),
            Get => self.get(req),
            Put => self.put(req),
            Delete => self.delete(req),
            _ => Err(Error::method_not_allowed()),
        }
    }
}
