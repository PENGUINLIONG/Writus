use std::sync::Arc;
use serde_json::Value as JsonValue;
use writium::prelude::*;
use writium_cache::Cache;
use api::index::Index;
use auth::SimpleAuthority;
use super::MetadataApi;
use self::header::{Authorization, Bearer};
use api::test_common::*;

mod source;
use self::source::MockSource;

const FOO_META_ALL: &'static str = r#"{"key":"Boom!","neko":3}"#;
const FOO_META: &'static str = r#"{"neko":3}"#;
const DELETED_FOO_META: &'static str = r#"{"key":"Boom!"}"#;
const NULL_NEKO_ALL: &'static str = r#"{"key":"Boom!","neko":null}"#;
const NULL_NEKO: &'static str = r#"{"neko":null}"#;

fn api() -> MetadataApi {
    let mut api = MetadataApi::new();
    api.set_cache(Arc::new(Cache::new(2, MockSource::new())));
    api.set_auth(Arc::new(SimpleAuthority::new("PASSWORD")));
    let index = Index::new("key", "string", None);
    api.set_index(index);
    api
}
fn gen_null_neko() -> JsonValue {
    let mut map = ::serde_json::value::Map::new();
    map.insert("neko".to_string(), JsonValue::Null);
    JsonValue::Object(map)
}

#[test]
fn test_get_all() {
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api(), req);
    check_type(&res, "application", "json");
    check_content(&res, FOO_META_ALL);
}
#[test]
fn test_get_some() {
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("keys[]=neko");
    let res = test_ok(&api(), req);
    check_type(&res, "application", "json");
    check_content(&res, FOO_META);
}

#[test]
fn test_put() {
    let api = api();
    let req = Request::new(Method::Put)
        .with_path_segs(&["bar"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }))
        .with_json(&gen_null_neko())
        .unwrap();
    let _ = test_ok(&api, req);
    let req = Request::new(Method::Get)
        .with_path_segs(&["bar"])
        .with_query("keys[]=neko");
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, NULL_NEKO);
}
#[test]
fn fail_put() {
    let req = Request::new(Method::Put)
        .with_path_segs(&["bar"])
        .with_json(&gen_null_neko())
        .unwrap();
    let err = test_err(&api(), req);
    assert_eq!(err.status(), StatusCode::Unauthorized);
}

#[test]
fn test_patch() {
    let api = api();
    let req = Request::new(Method::Patch)
        .with_path_segs(&["foo"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }))
        .with_json(&gen_null_neko())
        .unwrap();
    let _ = test_ok(&api, req);
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, NULL_NEKO_ALL);
}
#[test]
fn fail_patch() {
    let req = Request::new(Method::Patch)
        .with_path_segs(&["foo"])
        .with_json(&gen_null_neko())
        .unwrap();
    let err = test_err(&api(), req);
    assert_eq!(err.status(), StatusCode::Unauthorized);
}

#[test]
fn test_delete_all() {
    let req = Request::new(Method::Delete)
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }))
        .with_path_segs(&["foo"]);
    let _ = test_ok(&api(), req);
    let req = Request::new(Method::Get);
    let err = test_err(&api(), req);
    assert_eq!(err.status(), StatusCode::NotFound);
}
#[test]
fn fail_delete_all() {
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"]);
    let err = test_err(&api(), req);
    assert_eq!(err.status(), StatusCode::Unauthorized);
}
#[test]
fn test_delete_some() {
    let api = api();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }))
        .with_query("keys[]=neko");
    let _ = test_ok(&api, req);
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, DELETED_FOO_META);
}
#[test]
fn fail_delete_some() {
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("keys[]=neko");
    let err = test_err(&api(), req);
    assert_eq!(err.status(), StatusCode::Unauthorized);
}



fn api_with_many_articles() -> (MetadataApi, Index) {
    let mut api = MetadataApi::new();
    api.set_cache(Arc::new(Cache::new(2, MockSource::new())));
    api.set_auth(Arc::new(SimpleAuthority::new("PASSWORD")));
    let index = Index::new("key", "string", None);
    {
        let mut idx = index.write().unwrap();
        idx.insert("foo", &JsonValue::String("111".to_owned()));
        idx.insert("bar", &JsonValue::String("222".to_owned()));
        idx.insert("baz", &JsonValue::String("333".to_owned()));
    }
    api.set_index(index.clone());
    (api, index)
}

#[test]
fn test_index_order() {
    fn gen_999() -> JsonValue {
        let mut map = ::serde_json::value::Map::new();
        map.insert("key".to_string(), JsonValue::String("999".to_owned()));
        JsonValue::Object(map)
    }
    let (api, index) = api_with_many_articles();
    let req = Request::new(Method::Put)
        .with_path_segs(&["foo"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }))
        .with_json(&gen_999())
        .unwrap();
    let _ = test_ok(&api, req);
    assert_eq!(index.read().unwrap().get_range(0, 3), vec!["bar", "baz", "foo"]);
}
#[test]
fn test_index_removal() {
    let (api, index) = api_with_many_articles();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("keys[]=key")
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let _ = test_ok(&api, req);
    assert_eq!(index.read().unwrap().get_range(0, 2), vec!["bar", "baz"]);
}
