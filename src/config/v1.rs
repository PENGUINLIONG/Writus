use std::collections::HashMap;
use std::sync::Arc;
use auth::SimpleAuthority;
use toml::Value as TomlValue;
use writium::hyper::mime::Mime;
use writium::prelude::*;
use writium_cache::Cache;
use api::*;
use model::*;
use view::*;

#[derive(Deserialize)]
struct RawExtra {
    pub published_dir: Option<String>,
    pub auth_token: Option<String>,
    pub index_key: Option<String>,
    pub index_key_type: Option<String>,
    pub entries_per_request: Option<u64>,
    pub allowed_exts: Option<HashMap<String, String>>,
    pub template_dir: Option<String>,
}
pub struct Extra {
    pub published_dir: String,
    pub auth: Arc<SimpleAuthority>,
    pub index_key: String,
    pub index_key_type: String,
    pub entries_per_request: u64,
    pub allowed_exts: HashMap<String, Mime>,
    pub template_dir: String,
}
impl From<Extra> for Namespace {
    /// Construct a Namespace containing all the v1 api and views.
    fn from(extra: Extra) -> Namespace {
        let index = Index::new(&extra.index_key, &extra.index_key_type, Some(&extra.published_dir));
        let post_cache = Arc::new(Cache::new(10, PostSource::new(&extra.published_dir)));
        let metadata_cache = Arc::new(Cache::new(10, MetadataSource::new(&extra.published_dir)));
        let comment_cache = Arc::new(Cache::new(10, CommentSource::new(&extra.published_dir)));

        let mut post_api = PostApi::new();
        post_api.set_auth(extra.auth.clone());
        post_api.set_cache(post_cache.clone());
        post_api.set_index(index.clone());

        let mut comment_api = CommentApi::new();
        comment_api.set_auth(extra.auth.clone());
        comment_api.set_cache(comment_cache.clone());

        let mut metadata_api = MetadataApi::new();
        metadata_api.set_auth(extra.auth.clone());
        metadata_api.set_cache(metadata_cache.clone());
        metadata_api.set_index(index.clone());

        let mut resource_api = ResourceApi::new();
        resource_api.set_auth(extra.auth.clone());
        resource_api.set_published_dir(&extra.published_dir);
        resource_api.set_allowed_exts(extra.allowed_exts.clone());

        let apis = Namespace::new(&["api", "v1"])
            .with_api(post_api)
            .with_api(comment_api)
            .with_api(metadata_api)
            .with_api(resource_api);

        let mut post_view = PostView::new();
        post_view.set_post_cache(post_cache.clone());
        post_view.set_metadata_cache(metadata_cache.clone());
        let post_template = Template::from_file(&extra.template_dir, "post.html")
            .unwrap_or_default();
        post_view.set_template(post_template);

        let mut root_view = RootView::new();
        root_view.set_post_cache(post_cache);
        root_view.set_metadata_cache(metadata_cache);
        let digest_template = Template::from_file(&extra.template_dir, "digest.html")
            .unwrap_or_default();
        root_view.set_digest_template(digest_template);
        let index_template = Template::from_file(&extra.template_dir, "index.html")
            .unwrap_or_default();
        root_view.set_index_template(index_template);
        root_view.set_index(index.clone());
        root_view.set_entries_per_request(extra.entries_per_request as usize);

        let views = Namespace::new(&[])
            .with_api(post_view)
            .with_api(root_view);

        Namespace::new(&[])
            .with_api(apis)
            .with_api(views)
    }
}
impl From<TomlValue> for Extra {
    fn from(extra: TomlValue) -> Extra {
        let extra: RawExtra = extra.try_into().expect("Unable to parse fields neccessary to Writium Blog API v1");
        raw_to_extra(extra)
    }
}

fn raw_to_extra(extra: RawExtra) -> Extra {
    Extra {
        published_dir: extra.published_dir.unwrap_or("./published".to_string()),
        auth: if let Some(token) = extra.auth_token.as_ref() {
            Arc::new(SimpleAuthority::new(token))
        } else {
            Arc::new(SimpleAuthority::default())
        },
        index_key: extra.index_key.unwrap_or("published".to_string()),
        index_key_type: extra.index_key_type.unwrap_or("-datetime".to_string()),
        entries_per_request: extra.entries_per_request.unwrap_or(5),
        allowed_exts: extra.allowed_exts.unwrap_or_default()
            .into_iter()
            .map(|(x, y)| (x, y.parse().expect("Unable to parse MIME in field `allowed_ext`.")))
            .collect(),
        template_dir: extra.template_dir.unwrap_or("./templates".to_owned()),
    }
}
