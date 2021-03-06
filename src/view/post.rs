use std::sync::Arc;
use serde_json::Value as JsonValue;
use pulldown_cmark::Parser;
use pulldown_cmark::{Options as ParserOptions, OPTION_ENABLE_TABLES};
use writium::prelude::*;
use writium_cache::{Cache, DumbCacheSource};
use super::template::*;

pub struct PostView {
    template: Template,
    post_cache: Arc<Cache<String>>,
    metadata_cache: Arc<Cache<JsonValue>>,
}
impl PostView {
    pub fn new() -> PostView {
        PostView {
            template: Template::default(),
            post_cache: Arc::new(Cache::new(0, DumbCacheSource::new())),
            metadata_cache: Arc::new(Cache::new(0, DumbCacheSource::new())),
        }
    }
    pub fn set_post_cache(&mut self, cache: Arc<Cache<String>>) {
        self.post_cache = cache;
    }
    pub fn set_metadata_cache(&mut self, cache: Arc<Cache<JsonValue>>) {
        self.metadata_cache = cache;
    }
    pub fn set_template(&mut self, template: Template) {
        self.template = template;
    }
    pub fn render(&self, req: &mut Request) -> ApiResult {
        fn get_post(full_text: &str) -> (String, String) {
            let mut lines = full_text.lines();
            let title: String = lines
                .next()
                .unwrap_or_default()
                .chars()
                .skip_while(|ch| ch == &'#')
                .skip_while(|ch| ch == &' ')
                .collect();
            let mut content = String::new();
            lines.for_each(|x| {
                content += "\n";
                content += x;
            });
            (title, content)
        }
        fn md_to_html(md: &str) -> String {
            let mut buf = String::with_capacity(md.len());
            let mut opts = ParserOptions::empty();
            opts.insert(OPTION_ENABLE_TABLES);
            let parser = Parser::new_ext(&md, opts);
            ::pulldown_cmark::html::push_html(&mut buf, parser);
            buf
        }
        use self::header::ContentType;
        let id = req.path_segs().join("/");
        let post_cache = self.post_cache.get(&id)?;
        let content_guard = post_cache.read().unwrap();
        let (title, content) = get_post(content_guard.as_ref());
        let metadata_cache = self.metadata_cache.get(&id)?;
        let metadata_guard = metadata_cache.read().unwrap();
        let metadata: &JsonValue = &metadata_guard;
        let path = format!("/posts/{}", id);
        let res = Response::new()
            .with_header(ContentType(
                "text/html; charset=UTF-8".parse().unwrap()
            ))
            .with_body(self.template.render(&metadata, &[
                ("link", &path),
                ("id", &id),
                ("title", &title),
                ("content", &md_to_html(&content)),
            ]));
        Ok(res)
    }
}
impl Api for PostView {
    fn name(&self) -> &[&str] {
        &["posts"]
    }
    fn route(&self, req: &mut Request) -> ApiResult {
        use self::header::Allow;
        match req.method() {
            Method::Get => self.render(req),
            Method::Options => {
                let res = Response::new()
                    .with_header(Allow(vec![Method::Options, Method::Get]));
                Ok(res)
            },
            _ => Err(Error::method_not_allowed())
        }
    }
}
