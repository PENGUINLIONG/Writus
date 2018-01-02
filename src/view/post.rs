use serde_json::Value as JsonValue;
use writium::prelude::*;
use writium_cache::{Cache, DumbCacheSource};
use super::template::*;

pub struct PostView {
    template: Template,
    post_cache: Cache<String>,
    metadata_cache: Cache<JsonValue>,
}
impl PostView {
    pub fn new() -> PostView {
        PostView {
            template: Template::default(),
            post_cache: Cache::new(0, DumbCacheSource::new()),
            metadata_cache: Cache::new(0, DumbCacheSource::new()),
        }
    }
    pub fn set_post_cache(&mut self, cache: Cache<String>) {
        self.post_cache = cache;
    }
    pub fn set_metadata_cache(&mut self, cache: Cache<JsonValue>) {
        self.metadata_cache = cache;
    }
    pub fn set_template(&mut self, template: Template) {
        self.template = template;
    }
    pub fn render(&self, req: &mut Request) -> ApiResult {
        use self::header::ContentType;
        let id = req.path_segs().join("/");
        let post_cache = self.post_cache.get(&id)?;
        let content_guard = post_cache.read().unwrap();
        let content: &str = content_guard.as_ref();
        let metadata_cache = self.metadata_cache.get(&id)?;
        let metadata_guard = metadata_cache.read().unwrap();
        let metadata: &JsonValue = &metadata_guard;
        let res = Response::new()
            .with_header(ContentType("text/html; charset=UTF-8".parse().unwrap()))
            .with_body(self.template.render(&content, &metadata));
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
