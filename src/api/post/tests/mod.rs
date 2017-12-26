use std::sync::Arc;
use writium_framework::prelude::*;
use toml::Value as TomlValue;
use api::PostApi;
use self::header::{Authorization, Bearer, ContentType};
use auth::SimpleAuthority;

static CONTENT_MARKDOWN: &'static str = "# Title\n\nHello, Writus!\n\nBeep!";
static CONTENT_HTML: &'static str = "<h1>Title</h1>\n<p>Hello, Writus!</p>\n<p>Beep!</p>\n";
static CONTENT_MARKDOWN_DIGEST: &'static str = "# Title\n\nHello, Writus!";
static CONTENT_HTML_DIGEST: &'static str = "<h1>Title</h1>\n<p>Hello, Writus!</p>\n";

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
    let index = Index::default();
    let mut api = api();
    api.set_entries_per_request(2);
    index.write().unwrap().push((TomlValue::Integer(0), "/foo".into()));
    index.write().unwrap().push((TomlValue::Integer(1), "/bar".into()));
    index.write().unwrap().push((TomlValue::Integer(2), "/baz".into()));
    api.set_index(index);
    api
}

fn test_ok(api: &PostApi, mut req: Request) -> Response {
    let result = api.route(&mut req);
    assert!(result.is_ok());
    result.unwrap()
}
fn test_err(api: &PostApi, mut req: Request) -> Error {
    let result = api.route(&mut req);
    assert!(result.is_err());
    result.unwrap_err()
}
fn check_type(res: &Response, ty: &str, sub: &str) {
    let ctype = res.header::<ContentType>();
    assert!(ctype.is_some());
    let ctype = ctype.unwrap();
    assert_eq!(ctype.0.type_(), ty);
    assert_eq!(ctype.0.subtype(), sub);
}
fn check_content(res: &Response, content: &str) {
    let res_content = res.to_str();
    assert!(res_content.is_ok());
    assert_eq!(res_content.unwrap(), content);
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
    check_type(&res, "text", "html");
    check_content(&res, CONTENT_HTML);
}
#[test]
fn test_get_one_raw() {
    let api = api();
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("raw=true");
    let res = test_ok(&api, req);
    check_type(&res, "text", "markdown");
    check_content(&res, CONTENT_MARKDOWN);
}
#[test]
fn test_get_one_digest() {
    let api = api();
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("digest=true");
    let res = test_ok(&api, req);
    check_type(&res, "text", "html");
    check_content(&res, CONTENT_HTML_DIGEST);
}
#[test]
fn test_get_one_raw_digest() {
    let api = api();
    let req = Request::new(Method::Get)
        .with_path_segs(&["foo"])
        .with_query("digest=true&raw=true");
    let res = test_ok(&api, req);
    check_type(&res, "text", "markdown");
    check_content(&res, CONTENT_MARKDOWN_DIGEST);
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
