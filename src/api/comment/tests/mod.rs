use std::collections::HashMap;
use std::sync::Arc;
use auth::SimpleAuthority;
use super::{Comment, CommentApi};
use writium_cache::Cache;
use writium::prelude::*;
use api::test_common::*;
use self::header::{Authorization, Bearer};

mod source;
use self::source::MockSource;

const DEFAULT_ONE_JSON: &'static str = r#"{"metadata":{"author":"PENGUINLIONG"},"content":"Wow!"}"#;
const DEFAULT_JSON: &'static str = r#"{"0":{"metadata":{"author":"PENGUINLIONG"},"content":"Wow!"}}"#;
const MANY_JSON: &'static str = r#"{"0":{"metadata":{"author":"PENGUINLIONG"},"content":"Wow!"},"2":{"metadata":{"author":"NOTLIONG"},"content":"Well."}}"#;
const MANY_FROM_JSON: &'static str = r#"{"2":{"metadata":{"author":"NOTLIONG"},"content":"Well."},"4":{"metadata":{"author":"LIONG"},"content":":/"}}"#;
const CENTER_CUT_JSON: &'static str = r#"{"0":{"metadata":{"author":"PENGUINLIONG"},"content":"Wow!"},"4":{"metadata":{"author":"LIONG"},"content":":/"}}"#;
const POST_JSON: &'static str = r#"{"0":{"metadata":{"author":"PENGUINLIONG"},"content":"Wow!"},"1":{"metadata":{},"content":"Panda!"}}"#;

fn api() -> CommentApi {
    let mut api = CommentApi::new();
    api.set_cache(Arc::new(Cache::new(3, MockSource::new())));
    api.set_auth(Arc::new(SimpleAuthority::new("PASSWORD")));
    api
}
fn api_privilege() -> CommentApi {
    let mut api = CommentApi::new();
    api.set_cache(Arc::new(Cache::new(3, MockSource::new_privilege())));
    api.set_auth(Arc::new(SimpleAuthority::new("PASSWORD")));
    api
}
fn api_with_many_comments() -> CommentApi {
    let mut api = CommentApi::new();
    api.set_cache(Arc::new(Cache::new(3, MockSource::many_comments())));
    api.set_auth(Arc::new(SimpleAuthority::new("PASSWORD")));
    api.set_entries_per_request(2);
    api
}

#[test]
fn test_get() {
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let comments = test_ok(&api(), req);
    check_type(&comments, "application", "json");
    check_content(&comments, DEFAULT_JSON);
}
#[test]
fn test_get_many() {
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let comments = test_ok(&api_with_many_comments(), req);
    check_type(&comments, "application", "json");
    check_content(&comments, MANY_JSON);
}
#[test]
fn test_get_many_from() {
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("from=1");
    let comments = test_ok(&api_with_many_comments(), req);
    check_type(&comments, "application", "json");
    check_content(&comments, MANY_FROM_JSON);
}
#[test]
fn fail_get_one() {
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("index=1");
    let err = test_err(&api(), req);
    assert_eq!(err.status(), StatusCode::NotFound);
}
#[test]
fn test_get_one() {
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("index=0");
    let comments = test_ok(&api(), req);
    check_type(&comments, "application", "json");
    check_content(&comments, DEFAULT_ONE_JSON);
}

#[test]
fn fail_delete_one_auth() {
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("index=0");
    let err = test_err(&api(), req);
    assert_eq!(err.status(), StatusCode::Unauthorized);
}
#[test]
fn test_delete_one_auth() {
    let api = api();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("index=0")
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let _ = test_ok(&api, req);
    // The comment should be removed.
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("index=0");
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::NotFound);
}
#[test]
fn test_delete_one_token() {
    let api = api_privilege();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("index=0")
        .with_header(Authorization(Bearer { token: "POWER!".to_owned() }));
    let _ = test_ok(&api, req);
    // The comment should be removed.
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("index=0");
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::NotFound);
}
#[test]
fn test_delete_range_from() {
    let api = api_with_many_comments();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("from=2")
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let _ = test_ok(&api, req);
    // The comment should be removed.
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, DEFAULT_JSON);
}
#[test]
fn test_delete_range_to() {
    let api = api_with_many_comments();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("to=2")
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let _ = test_ok(&api, req);
    // The comment should be removed.
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, MANY_FROM_JSON);
}
#[test]
fn test_delete_range_from_to() {
    let api = api_with_many_comments();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("from=2&to=3")
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let _ = test_ok(&api, req);
    // The comment should be removed.
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, CENTER_CUT_JSON);
}
#[test]
fn fail_delete_range_from_to() {
    let api = api_with_many_comments();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_query("from=2&to=2")
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::BadRequest);
}
#[test]
fn test_delete_all() {
    let api = api_with_many_comments();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let _ = test_ok(&api, req);
    // The comment should be removed.
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, "{}");
}

#[test]
fn test_post() {
    let api = api();
    let req = Request::new(Method::Post)
        .with_path_segs(&["foo"])
        .with_json(&Comment { metadata: HashMap::new(), content: "Panda!".to_owned() })
        .unwrap();
    let _ = test_ok(&api, req);
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, POST_JSON);
}
