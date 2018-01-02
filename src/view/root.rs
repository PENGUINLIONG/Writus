use serde_json::Value as JsonValue;
use pulldown_cmark::Parser;
use pulldown_cmark::{Options as ParserOptions, OPTION_ENABLE_TABLES};
use writium::prelude::*;
use writium_cache::{Cache, DumbCacheSource};
use api::index::Index;
use super::template::*;

pub struct RootView {
    index_template: Template,
    digest_template: Template,
    post_cache: Cache<String>,
    metadata_cache: Cache<JsonValue>,
    index: Index,
    entries_per_request: usize,
}
impl RootView {
    pub fn new() -> RootView {
        RootView {
            index_template: Template::default(),
            digest_template: Template::default(),
            post_cache: Cache::new(0, DumbCacheSource::new()),
            metadata_cache: Cache::new(0, DumbCacheSource::new()),
            index: Index::default(),
            entries_per_request: 5,
        }
    }
    pub fn set_post_cache(&mut self, cache: Cache<String>) {
        self.post_cache = cache;
    }
    pub fn set_metadata_cache(&mut self, cache: Cache<JsonValue>) {
        self.metadata_cache = cache;
    }
    pub fn set_digest_template(&mut self, template: Template) {
        self.digest_template = template;
    }
    pub fn set_index_template(&mut self, template: Template) {
        self.index_template = template;
    }
    pub fn set_index(&mut self, index: Index) {
        self.index = index;
    }
    pub fn set_entries_per_request(&mut self, epr: usize) {
        self.entries_per_request = epr;
    }

    fn render_digest(&self, post: &str, meta: &JsonValue) -> String {
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
        let text = get_digest(&post);
        let mut html = String::with_capacity(text.len());
        let mut opts = ParserOptions::empty();
        opts.insert(OPTION_ENABLE_TABLES);
        let parser = Parser::new_ext(&text, opts);
        ::pulldown_cmark::html::push_html(&mut html, parser);
        self.digest_template.render(&html, meta)
    }
    fn render_index(&self, req: &mut Request) -> ApiResult {
        use self::header::ContentType;
        use serde_json::value::Map;
        #[derive(Deserialize)]
        struct Param {
            /// The current page number.
            page: Option<usize>,
        }
        let param = req.to_param::<Param>()?;

        let guard = self.index.read().unwrap();
        let max_page = {
            let len = guard.len();
            if len % self.entries_per_request > 0 {
                len / self.entries_per_request + 1
            } else {
                len / self.entries_per_request
            }
        };
        let page = param.page.unwrap_or_default()
            .max(1)
            .min(max_page);
        let skip = (page - 1) * self.entries_per_request;
        let take = self.entries_per_request;

        let ids = guard.get_range(skip, take);
        let mut digests = String::new();
        for id in ids {
            let post_cache = self.post_cache.get(&id)?;
            let post_guard = post_cache.read().unwrap();
            let post: &str = post_guard.as_ref();
            let metadata_cache = self.metadata_cache.get(&id)?;
            let metadata_guard = metadata_cache.read().unwrap();
            let metadata: &JsonValue = &metadata_guard;
            digests.push_str(&self.render_digest(post, metadata));
        }
        let mut meta = Map::with_capacity(3);
        if page - 1 != 0 {
            let query = format!("?page={}", page - 1);
            meta.insert("prev".to_string(), JsonValue::String(query));
        }
        if page + 1 != max_page {
            let query = format!("?page={}", page + 1);
            meta.insert("next".to_string(), JsonValue::String(query));
        }
        let meta = JsonValue::Object(meta);
        let res = Response::new()
            .with_header(ContentType("text/html; charset=UTF-8".parse().unwrap()))
            .with_body(self.index_template.render(&digests, &meta));
        Ok(res)
    }
}
impl Api for RootView {
    fn name(&self) -> &[&str] {
        &[]
    }
    fn route(&self, req: &mut Request) -> ApiResult {
        use self::header::{Allow, Location};
        match req.method() {
            Method::Get => {
                if req.path_segs().len() == 0 ||
                    req.path_segs()[0] == "" {
                    self.render_index(req)
                } else {
                    let mut loc = "/api/v1/resources".to_owned();
                    for seg in req.path_segs() {
                        loc.push('/');
                        loc.push_str(seg);
                    }
                    let res = Response::new()
                        .with_status(StatusCode::Found)
                        .with_header(Location::new(loc));
                    Ok(res)
                }
            },
            Method::Options => {
                let res = Response::new()
                    .with_header(Allow(vec![Method::Options, Method::Get]));
                Ok(res)
            },
            _ => Err(Error::method_not_allowed())
        }
    }
}
