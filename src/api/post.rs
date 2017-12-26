use std::fs::{File, OpenOptions};
use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use hyper::header::ContentType;
use pulldown_cmark::Parser;
use pulldown_cmark::{Options as ParserOptions, OPTION_ENABLE_TABLES};
use walkdir::WalkDir;
use toml::Value as TomlValue;
use toml::value::Datetime as TomlDateTime;
use writium_framework::prelude::*;
use writium_auth::Authority;
use writium_cache::{Cache, CacheSource};
use auth::SimpleAuthority;

const ERR_ACCESS: &'static str = "Cannot access to requested resource.";
const ERR_MIME: &'static str = "Only data of type 'text/markdown' is accepted.";

pub struct PostApi {
    auth: Arc<SimpleAuthority>,
    cache: Cache<String>,
    index: Arc<RwLock<Vec<(TomlValue, PathBuf)>>>,
    entries_per_request: u64,
}

fn make_index(published_dir: &str, key: &str, ty: &str) -> Option<Vec<(TomlValue, PathBuf)>> {
    fn lt_datetime(a: &TomlDateTime, b: &TomlDateTime) -> Ordering {
        let a = ::chrono::DateTime::parse_from_rfc3339(&a.to_string()).unwrap();
        let b = ::chrono::DateTime::parse_from_rfc3339(&b.to_string()).unwrap();
        a.cmp(&b).reverse()
    }
    fn cast_lt(a: &TomlValue, b: &TomlValue, ty: &str) -> Ordering {
        match ty {
            "string" => a.as_str().unwrap().cmp(b.as_str().unwrap()),
            "integer" => a.as_integer().unwrap().cmp(&b.as_integer().unwrap()),
            "datetime" => lt_datetime(a.as_datetime().unwrap(), b.as_datetime().unwrap()),
            _ => panic!("'{}' cannot be used as key type.", ty),
        }
    }
    fn get_index_val_for(parent: &Path, key: &str) -> Option<TomlValue> {
        // Find `metadata.toml`.
        let mut text = Vec::new();
        let mut file = File::open(path_buf![parent, "metadata.toml"])
            .map_err(|err| error!("Unable to open metadata from '{}': {}",
                parent.to_string_lossy(), err))
            .ok()?;
        file.read_to_end(&mut text)
            .map_err(|err| error!("Unable to read metadata from '{}': {}",
                parent.to_string_lossy(), err))
            .ok()?;
        let toml = ::toml::from_slice::<TomlValue>(&text)
            .map_err(|err| warn!("Unable to serialize content of '{}': {}",
                parent.to_string_lossy(), err))
            .ok()?;
        toml.get(key)
            .map(|x| x.to_owned())
    }

    info!("Indexing files with key '{}' of type '{}'.", key, ty);
    // Because Windows NT allow non-disk paths, so we have to canonicalize so
    // that the paths of subitems literally start with this prefix.
    let mut unordered = Vec::new();
    for entry in WalkDir::new(&published_dir)
        .into_iter()
        .filter_map(|x| x.ok()) {
        // Seek for `content.md`.
        if entry.file_type().is_file() &&
            entry.file_name() == "content.md" {
            if let Some(parent) = entry.path().parent() {
                info!("Indexing article '{}'...", &parent.to_string_lossy());
                if let Some(val) = get_index_val_for(parent, key) {
                    if val.type_str() != ty {
                        warn!("Article is not indexed: index key of type '{}' was found, but it should be of type '{}'.",
                            ty, val.type_str());
                        continue
                    }
                    unordered.push((val, parent.strip_prefix(&published_dir).unwrap().to_owned()));
                } else {
                    warn!("Article is not indexed: index key is inaccessible.");
                }
            } else {
                error!("Unexpected error accessing parent of: {}",
                    &entry.path().to_string_lossy());
            }
        }
    }
    unordered.sort_by(|&(ref a, _), &(ref b, _)| cast_lt(&a, &b, ty));
    Some(unordered)
}
impl PostApi {
    pub fn new(extra: &super::V1Extra) -> PostApi {
        PostApi {
            auth: extra.auth.clone(),
            cache: Cache::new(10, PostSource::new(&extra.published_dir)),
            index: Arc::new(RwLock::new(
                make_index(&extra.published_dir,
                    &extra.index_key, &extra.index_key_type)
                    .unwrap_or_default()
            )),
            entries_per_request: extra.entries_per_request,
        }
    }

    fn get_content(&self, req: &mut Request) -> ApiResult {
        fn get_digest(full_text: &str) -> String {
            let mut rv = String::new();
            let mut lines = full_text.lines();
            rv.push_str(lines.next().unwrap_or_default());
            rv.push_str("\n\n");
            lines.skip_while(|line| line.trim().len() == 0)
                .take_while(|line| line.trim().len() > 0)
                .for_each(|line| rv.push_str(line));
            rv
        }
        #[derive(Deserialize)]
        struct Param {
            /// Get raw markdown rather than parsed html.
            raw: Option<bool>,
            /// Get the title and the first paragraph.
            digest: Option<bool>,
        }

        let id = req.path_segs().join("/");
        let param = req.to_param::<Param>()?;
        let cache = self.cache.get(&id)?;
        let mut text = cache.read().unwrap().clone();
        // If raw markdown was requested, return right away.
        if let Some(true) = param.digest {
            text = get_digest(&text);
        }
        let res = if let Some(true) = param.raw {
            Response::new()
                .with_header(ContentType(
                    "text/markdown; charset=UTF-8".parse().unwrap()))
                .with_body(text.into_bytes())
        // By default we return the translated HTML.
        } else {
            let mut html = String::with_capacity(text.len());
            let mut opts = ParserOptions::empty();
            opts.insert(OPTION_ENABLE_TABLES);
            let parser = Parser::new_ext(&text, opts);
            ::pulldown_cmark::html::push_html(&mut html, parser);
            Response::new()
                .with_header(ContentType(
                    "text/html; charset=UTF-8".parse().unwrap()))
                .with_body(html.into_bytes())
        };
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
        let entries = guard.iter()
            .skip(from)
            .take(self.entries_per_request as usize)
            .map(|&(_, ref path)| if cfg!(windows) {
                    path.to_string_lossy().replace('\\', "/")
                } else {
                    path.to_string_lossy().to_string()
                }
            )
            .collect::<Vec<_>>();
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

struct PostSource {
    dir: String,
}
impl PostSource {
    fn new(dir: &str) -> PostSource {
        PostSource {
            dir: dir.to_string(),
        }
    }
    fn open_content(&self, id: &str, read: bool) -> ::std::io::Result<File> {
        use std::fs::create_dir_all;
        info!("Try openning file of ID: {}", id);
        let mut path = path_buf![&self.dir, id];
        if !read && !path.exists() {
            create_dir_all(&path)?;
        }
        path.push("content.md");
        OpenOptions::new()
            .read(read)
            .write(!read)
            .create(!read)
            .open(path)
    }
}
impl CacheSource for PostSource {
    type Value = String;
    fn load(&self, id: &str, create: bool) -> Result<String> {
        use std::io::{Read, BufReader};
        let r_content = self.open_content(id, true)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
            .and_then(|file| {
                let mut reader = BufReader::new(file);
                // Find the title line.
                let mut post = String::new();
                reader.read_to_string(&mut post)
                    .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
                // Convert Markdown to HTML only when it's needed. Allow new posts to be
                // published.
                Ok(post)
            });
        if r_content.is_err() && create {
            Ok(String::new())
        } else {
            r_content
        }
    }
    fn unload(&self, id: &str, val: &String) -> Result<()> {
        use std::io::{Write, BufWriter};
        trace!("Unloading {}", id);
        if let Ok(file) = self.open_content(id, false) {
            let mut writer = BufWriter::new(file);
            if writer.write_all(val.as_bytes()).is_ok() {
                // Convert Markdown to HTML only when it's needed. Allow new posts
                // to be published.
                trace!("Successed {}", id);
               Ok(())
            } else {
               Err(Error::internal(ERR_ACCESS))
            }
        } else {
            // No local file found, it's calling for creating a new resource.
           Err(Error::internal(ERR_ACCESS))
        }
    }
    fn remove(&self, id: &str) -> Result<()> {
        use std::fs::remove_file;
        if remove_file(path_buf![&self.dir, id, "content.md"]).is_err() {
           Err(Error::internal(ERR_ACCESS))
        } else {
           Ok(())
        }
    }
}
