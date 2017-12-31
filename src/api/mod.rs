use std::sync::Arc;
use toml::Value as TomlValue;
use writium::prelude::*;

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

pub fn api_v1(extra: TomlValue) -> Namespace {
    let extra = ::config::v1::convert_extra(extra);
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
    metadata.set_index(index.clone());

    Namespace::new(&["api", "v1"])
        .with_api(post)
        .with_api(comment)
        .with_api(metadata)
        .with_api(ResourceApi::new(&extra))
}
