use std::sync::Arc;
use writium::prelude::*;
use api::PostApi;
use self::header::{Authorization, Bearer, ContentType};
use auth::SimpleAuthority;
use api::test_common::*;

static CONTENT_MARKDOWN: &'static str = "# Title\n\nHello, Writus!\n\nBeep!";

static CONTENT_DIF: &'static str = "Wow!";

mod source;

fn api() -> PostApi {
    use writium_cache::Cache;
    let mut post = PostApi::new();
    post.set_auth(Arc::new(SimpleAuthority::new("PASSWORD")));
    post.set_cache(Cache::new(3, source::MockSource::new()));
    post
}
fn indexed_api() -> PostApi {
    use api::Index;
    use serde_json::Value as JsonValue;
    let index = Index::new("key", "integer", None);
    let mut api = api();
    api.set_entries_per_request(2);
    index.write().unwrap().insert("/foo".into(), &JsonValue::Number(0.into()));
    index.write().unwrap().insert("/bar".into(), &JsonValue::Number(1.into()));
    index.write().unwrap().insert("/baz".into(), &JsonValue::Number(2.into()));
    api.set_index(index);
    api
}

#[test]
fn fail_get_one() {
    let api = api();
    let req = Request::new(Method::Get)
        .with_path_segs(&["bar"]);
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::NotFound);
}
#[test]
fn test_get_one() {
    let api = api();
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let res = test_ok(&api, req);
    check_type(&res, "text", "markdown");
    check_content(&res, CONTENT_MARKDOWN);
}
#[test]
fn test_get_index() {
    let api = indexed_api();
    let req = Request::new(Method::Get);
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, r#"["/foo","/bar"]"#);
}
#[test]
fn test_get_index_from() {
    let api = indexed_api();
    let req = Request::new(Method::Get)
        .with_query("from=1");
    let res = test_ok(&api, req);
    check_type(&res, "application", "json");
    check_content(&res, r#"["/bar","/baz"]"#);
}

#[test]
fn fail_put_auth() {
    let api = api();
    let req = Request::new(Method::Put)
        .with_path_segs(&["bar"])
        .with_header(ContentType("text/markdown".parse().unwrap()));
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::Unauthorized);
}
#[test]
fn fail_put_type() {
    let api = api();
    let req = Request::new(Method::Put)
        .with_path_segs(&["bar"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::BadRequest);
}
#[test]
fn test_put() {
    let api = api();
    let req = Request::new(Method::Put)
        .with_path_segs(&["bar"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }))
        .with_header(ContentType("text/markdown".parse().unwrap()))
        .with_body(CONTENT_DIF);
    let _ = test_ok(&api, req);
    // Check content is correct.
    let req = Request::new(Method::Get)
        .with_path_segs(&["bar"])
        .with_query("raw=true");
    let res = test_ok(&api, req);
    check_type(&res, "text", "markdown");
    check_content(&res, CONTENT_DIF);
}

#[test]
fn fail_delete_auth() {
    let api = api();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"]);
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::Unauthorized);
}
#[test]
fn test_delete() {
    let api = api();
    let req = Request::new(Method::Delete)
        .with_path_segs(&["foo"])
        .with_header(Authorization(Bearer { token: "PASSWORD".to_owned() }));
    let _ = test_ok(&api, req);
    // Check article is actually removed.
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"]);
    let err = test_err(&api, req);
    assert_eq!(err.status(), StatusCode::NotFound);
}
