use std::collections::HashMap;
use std::sync::Arc;
use toml::Value as TomlValue;
use auth::SimpleAuthority;
use writium::prelude::*;
use writium::hyper::mime::Mime;

pub mod index;

pub use self::index::Index;

#[cfg(test)]
mod test_common;

pub mod comment;
pub mod metadata;
pub mod post;
pub mod resource;

// Reexport APIs.
pub use self::comment::CommentApi;
pub use self::metadata::MetadataApi;
pub use self::post::PostApi;
pub use self::resource::ResourceApi;

#[derive(Deserialize)]
struct V1RawExtra {
    pub published_dir: Option<String>,
    pub auth_token: Option<String>,
    pub index_key: Option<String>,
    pub index_key_type: Option<String>,
    pub entries_per_request: Option<u64>,
    pub allowed_exts: Option<HashMap<String, String>>,
}
pub struct V1Extra {
    pub published_dir: String,
    pub auth: Arc<SimpleAuthority>,
    pub index_key: String,
    pub index_key_type: String,
    pub entries_per_request: u64,
    pub allowed_exts: Arc<HashMap<String, Mime>>,
}

fn convert_v1_extra(extra: V1RawExtra) -> V1Extra {
    V1Extra {
        published_dir: extra.published_dir.unwrap_or("./published".to_string()),
        auth: if let Some(token) = extra.auth_token.as_ref() {
                Arc::new(SimpleAuthority::new(token))
            } else {
                Arc::new(SimpleAuthority::default())
            },
        index_key: extra.index_key.unwrap_or("published".to_string()),
        index_key_type: extra.index_key_type.unwrap_or("datetime".to_string()),
        entries_per_request: extra.entries_per_request.unwrap_or(5),
        allowed_exts: Arc::new(
                extra.allowed_exts.unwrap_or_default()
                    .into_iter()
                    .map(|(x, y)| (x, y.parse().expect("Unable to parse MIME in field `allowed_ext`.")))
                    .collect()
            ),
    }
}

pub fn api_v1(extra: TomlValue) -> Namespace {
    let extra: V1RawExtra = extra.try_into().expect("Unable to parse fields neccessary to Writium Blog API v1");
    let extra = convert_v1_extra(extra);
    
    let index = Index::new(&extra.index_key, &extra.index_key_type, Some(&extra.published_dir));

    let mut post = PostApi::new();
    post.set_auth(extra.auth.clone());
    post.set_cache_default(&extra.published_dir);
    post.set_index(index.clone());

    let mut comment = CommentApi::new();
    comment.set_auth(extra.auth.clone());
    comment.set_cache_default(&extra.published_dir);

    let mut metadata = MetadataApi::new();
    metadata.set_auth(extra.auth.clone());
    metadata.set_cache_default(&extra.published_dir);

    Namespace::new(&["api", "v1"])
        .with_api(post)
        .with_api(comment)
        .with_api(metadata)
        .with_api(ResourceApi::new(&extra))
}
