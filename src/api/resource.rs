use std::fs::File;
use std::io::{Read, Write};
use std::collections::HashMap;
use std::sync::Arc;
use writium::prelude::*;
use self::header::ContentType;
use writium::hyper::mime::Mime;
use writium_auth::{Authority, DumbAuthority};

const ERR_MISSING_CONTENT_TYPE: &'static str = "Content type should be denoted for verification use.";
const ERR_MIME_NOT_FOUND: &'static str = "No corresponding MIME matches the inquired file type (extension). Maybe the file type is intentionally prevented from being transferred.";
const ERR_MIME_EXT_MISMATCH: &'static str = "Path extension doesn't accord with content type denoted.";
const ERR_ACCESS: &'static str = "Cannot access to requested resource.";

pub struct ResourceApi {
    auth: Arc<Authority<Privilege=()>>,
    published_dir: String,
    allowed_exts: HashMap<String, Mime>,
}

impl ResourceApi {
    pub fn new() -> ResourceApi {
        ResourceApi {
            auth: Arc::new(DumbAuthority::new()),
            published_dir: String::new(),
            allowed_exts: HashMap::new(),
        }
    }
    pub fn set_auth(&mut self, auth: Arc<Authority<Privilege=()>>) {
        self.auth = auth;
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

        let path = path_buf![&self.published_dir, &id];
        let mut file = File::open(&path)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        let file_len = file.metadata()
            .map(|meta| meta.len())
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        let mut vec = Vec::with_capacity(file_len as usize);
        file.read_to_end(&mut vec)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        Ok(Response::new()
            .with_header(ContentType(mime.clone()))
            .with_body(vec))
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

        let path = path_buf![&self.published_dir, &id];
        let mut file = File::create(path)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        file.write_all(req.body())
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        Ok(Response::new())
    }

    fn delete(&self, req: &mut Request) -> ApiResult {
        self.auth.authorize((), &req)?;

        let id = req.path_segs().join("/");
        let path = path_buf![&self.published_dir, &id];
        ::std::fs::remove_file(&path)
            .map(|_| Response::new())
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
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
