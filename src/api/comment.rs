///! Comments API.
///! All comments of an article are stored in `comments.json`.
use std::io::{BufReader, BufWriter};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::sync::Arc;
use serde_json as json;
use writium_framework::prelude::*;
use writium_cache::{Cache, CacheSource};
use writium_auth::Authority;
use auth::SimpleAuthority;
use super::V1Extra;

const COMMENTS_PER_QUERY: usize = 5;

const ERR_ACCESS: &'static str = "Cannot access to requested resource.";
const ERR_PRIVILEGE: &'static str = "Requested operation need a matching privilege token to execute.";
const ERR_NOT_FOUND: &'static str = "Cannot find a comment matching the requested index. Maybe it's been deleted already.";

#[derive(Clone, Deserialize, Serialize)]
struct CommentJson {
    pub metadata: HashMap<String, String>,
    pub content: String,
}

pub struct CommentApi {
    cache: Cache<HashMap<usize, CommentJson>>,
    auth: Arc<SimpleAuthority>,
}
impl CommentApi {
    pub fn new(extra: &V1Extra) -> CommentApi {
        CommentApi {
            cache: Cache::new(10, CommentSource::new(&extra.published_dir)),
            auth: extra.auth.clone(),
        }
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
        } else if param.from.is_some() && param.to.is_some() {
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
                .map(|auth| &auth.token)
                .and_then(|auth_token| {
                    if priv_token == auth_token {
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
        for index in from.unwrap_or(0)..to.unwrap_or(cache.len()) {
            cache.remove(&index);
        }
        Ok(Response::new())
    }
    fn delete_all(&self, req: &mut Request) -> ApiResult {
        self.auth.authorize((), req)?;
        let id = req.path_segs().join("/");
        self.cache.remove(&id)
            .map(|_| Response::new())
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
            .ok_or(Error::bad_request(ERR_NOT_FOUND))
            .and_then(|comment| Response::new().with_json(comment))
    }
    /// If from specifier present, get the item indexed form and several
    /// following items. In case some of the items are missing, ignore
    /// them.
    fn get_from(&self, req: &mut Request, from: usize) -> ApiResult {
        let id = req.path_segs().join("/");
        let lock = self.cache.get(&id)?;
        let cache = lock.read().unwrap();
        let mut res_content = HashMap::new();
        for i in from..(from + COMMENTS_PER_QUERY) {
            if let Some(comment) = cache.get(&i) {
                res_content.insert(i, comment);
            }
        }
        Response::new().with_json(&res_content)
    }

    /// POST `/comments/<path..>`
    fn post(&self, req: &mut Request) -> ApiResult {
        let id = req.path_segs().join("/");
        let comment = req.to_json::<CommentJson>()?;
        let cache = self.cache.get(&id)?;
        // There are comments already loaded, start indexing from the last one.
        if let Some((id, _)) = cache.read().unwrap().iter().last() {
            cache.write().unwrap().insert(id + 1, comment);
        } else {
            cache.write().unwrap().insert(0, comment);
        }
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

struct CommentSource {
    dir: String,
}
impl CommentSource {
    fn new(dir: &str) -> CommentSource {
        CommentSource {
            dir: dir.to_string(),
        }
    }
    fn open_comment(&self, id: &str, read: bool) -> ::std::io::Result<File> {
        info!("Try openning file of ID: {}", id);
        OpenOptions::new()
            .create(true)
            .read(read)
            .write(!read)
            .open(path_buf![&self.dir, id, "comments.json"])
    }
}
impl CacheSource for CommentSource {
    type Value = HashMap<usize, CommentJson>;
    fn load(&self, id: &str, create: bool) -> Result<Self::Value> {
        self.open_comment(id, true)
            .and_then(|file| {
                let reader = BufReader::new(file);
                let json: Self::Value = json::from_reader(reader)?;
                Ok(json)
            })
            .or_else(|_err| {
                if create {
                    Ok(HashMap::new())
                } else {
                    Err(Error::internal(ERR_ACCESS))
                }
            })
    }
    fn unload(&self, id: &str, obj: &Self::Value) -> Result<()> {
        let file = self.open_comment(id, false)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        let writer = BufWriter::new(file);
        json::to_writer_pretty(writer, obj)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
    }
    fn remove(&self, id: &str) -> Result<()> {
        use std::fs::remove_file;
        remove_file(path_buf![&self.dir, id, "comments.json"])
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
    }
}
