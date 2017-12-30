///! Comments API.
///! All comments of an article are stored in `comments.json`.
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use writium::prelude::*;
use writium_auth::Authority;
use writium_cache::Cache;

mod source;
#[cfg(test)]
mod tests;

use self::source::DefaultSource;

const DEFAULT_ENTRIES_PER_REQUEST: usize = 5;

const ERR_PRIVILEGE: &'static str = "Requested operation need a matching privilege token to execute.";
const ERR_NOT_FOUND: &'static str = "Cannot find a comment matching the requested index. Maybe it's been deleted already.";
const ERR_RANGE: &'static str = "The requested range is not valid. A valid range should be one of `from={from}`, `to={to}`, or `from={from}&to={to}` where `{from}` < `{to}`.";

#[derive(Clone, Deserialize, Serialize)]
pub struct Comment {
    pub metadata: HashMap<String, String>,
    pub content: String,
}

pub struct CommentApi {
    cache: Cache<BTreeMap<usize, Comment>>,
    auth: Arc<Authority<Privilege=()>>,
    entries_per_request: usize,
}
impl CommentApi {
    pub fn new() -> CommentApi {
        CommentApi {
            cache: Cache::new(0, ::writium_cache::DumbCacheSource::new()),
            auth: Arc::new(::writium_auth::DumbAuthority::new()),
            entries_per_request: DEFAULT_ENTRIES_PER_REQUEST,
        }
    }

    pub fn set_cache_default(&mut self, published_dir: &str) {
        self.cache = Cache::new(10, DefaultSource::new(published_dir));
    }
    pub fn set_cache(&mut self, cache: Cache<BTreeMap<usize, Comment>>) {
        self.cache = cache;
    }
    pub fn set_auth(&mut self, auth: Arc<Authority<Privilege=()>>) {
        self.auth = auth;
    }
    pub fn set_entries_per_request(&mut self, entries_per_request: usize) {
        self.entries_per_request = entries_per_request;
    }

    /// DELETE `/comments/<path..>?{index}`  
    /// DELETE `/comments/<path..>?[from][to][{from, to}]`  
    /// DELETE `/comments/<path..>?{all}`
    fn delete(&self, req: &mut Request) -> ApiResult {
        #[derive(Clone, Deserialize)]
        struct DeleteParam {
            pub index: Option<usize>,
            pub from: Option<usize>,
            pub to: Option<usize>,
        }
        let param = req.to_param::<DeleteParam>()?;
        // Fetch cache.
        // If index is present. Remove only the one indicated by the index.
        if let Some(index) = param.index {
            self.delete_one(req, index)
        // If any range specifier present, remove the range.
        } else if param.from.is_some() || param.to.is_some() {
            self.delete_range(req, param.from.clone(), param.to.clone())
        // Finally, if no delimiter was set, remove the entire `comments.json`
        // file. The indexing will start from 0 again next time it's generated.
        } else {
            self.delete_all(req)
        }
    }
    fn delete_one(&self, req: &mut Request, index: usize) -> ApiResult {
        fn auth(priv_token: &str, req: &Request) -> Result<()> {
            req.header::<Authorization<Bearer>>()
                .ok_or(Error::bad_request(ERR_PRIVILEGE))
                .and_then(|auth| {
                    if priv_token == &auth.token {
                        Ok(())
                    } else {
                        Err(Error::bad_request(ERR_PRIVILEGE))
                    }
                })
        }
        use self::header::{Authorization, Bearer};
        let id = req.path_segs().join("/");
        let lock = self.cache.get(&id)?;
        let mut cache = lock.write().unwrap();
        self.auth.authorize((), req)
            .or_else(|err| {
                let comment = cache.get(&index)
                    .ok_or(Error::not_found(ERR_NOT_FOUND))?;
                comment.metadata.get("privilege")
                    .ok_or(err)
                    .and_then(|priv_token| auth(priv_token, req))
            })?;
        cache.remove(&index);
        Ok(Response::new())
    }
    fn delete_range(&self, req: &mut Request,
        from: Option<usize>, to: Option<usize>) -> ApiResult {
        self.auth.authorize((), req)?;
        let id = req.path_segs().join("/");
        let lock = self.cache.get(&id)?;
        let mut cache = lock.write().unwrap();
        let from = from.unwrap_or(0);
        let to = if let Some(to) = to {
            to
        } else if let Some(ref last) = cache.iter().last() {
            last.0.to_owned() + 1
        } else {
            return Ok(Response::new())
        };
        if from >= to {
            return Err(Error::bad_request(ERR_RANGE));
        }
        for index in from..to {
            cache.remove(&index);
        }
        Ok(Response::new())
    }
    fn delete_all(&self, req: &mut Request) -> ApiResult {
        self.auth.authorize((), req)?;
        let id = req.path_segs().join("/");
        let lock = self.cache.get(&id)?;
        let mut cache = lock.write().unwrap();
        cache.clear();
        Ok(Response::new())
    }

    /// GET `/comments/<path..>?{index}`  
    /// GET `/comments/<path..>?[from]`
    fn get(&self, req: &mut Request) -> ApiResult {
        #[derive(Clone, Deserialize)]
        struct GetParam {
            pub index: Option<usize>,
            pub from: Option<usize>,
        }
        let param = req.to_param::<GetParam>()?;
        // Fetch cache.
        if let Some(index) = param.index {
            self.get_one(req, index as usize)
        } else {
            self.get_from(req, param.from.unwrap_or(0))
        }
    }
    /// If index is present. Remove only the one indicated by the index.
    fn get_one(&self, req: &mut Request, index: usize) -> ApiResult {
        let id = req.path_segs().join("/");
        let lock = self.cache.get(&id)?;
        let cache = lock.read().unwrap();
        cache.get(&index)
            .ok_or(Error::not_found(ERR_NOT_FOUND))
            .and_then(|comment| Response::new().with_json(comment))
    }
    /// If from specifier present, get the item indexed form and several
    /// following items. In case some of the items are missing, ignore
    /// them.
    fn get_from(&self, req: &mut Request, from: usize) -> ApiResult {
        let id = req.path_segs().join("/");
        let lock = self.cache.get(&id)?;
        let cache = lock.read().unwrap();

        let res_content: BTreeMap<&usize, &Comment> = cache.iter()
            .skip(from)
            .take(self.entries_per_request)
            .collect();
        Response::new().with_json(&res_content)
    }

    /// POST `/comments/<path..>`
    fn post(&self, req: &mut Request) -> ApiResult {
        let id = req.path_segs().join("/");
        let comment = req.to_json::<Comment>()?;
        let cache = self.cache.create(&id)?;
        // There are comments already loaded, start indexing from the last one.
        let index = if let Some((index, _)) = cache.read().unwrap().iter().last() {
            Some(index.to_owned())
        } else {
            None
        };
        cache.write().unwrap().insert(index.unwrap_or(0) + 1, comment);
        Ok(Response::new().with_status(StatusCode::Created))
    }
}
impl Api for CommentApi {
    fn name(&self) -> &[&str] {
        &["comments"]
    }

    fn route(&self, req: &mut Request) -> ApiResult {
        use self::header::Allow;
        use self::Method::*;
        match req.method() {
            Options => Ok(Response::new()
                .with_header(Allow(vec![Options, Get, Post, Delete]))),
            Get => self.get(req),
            Post => self.post(req),
            Delete => self.delete(req),
            _ => Err(Error::method_not_allowed()),
        }
    }
}
