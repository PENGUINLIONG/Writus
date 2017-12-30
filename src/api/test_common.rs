use writium::prelude::*;
use self::header::ContentType;

pub fn test_ok(api: &Api, mut req: Request) -> Response {
    let result = api.route(&mut req);
    result.unwrap()
}
pub fn test_err(api: &Api, mut req: Request) -> Error {
    let result = api.route(&mut req);
    result.unwrap_err()
}
pub fn check_type(res: &Response, ty: &str, sub: &str) {
    let ctype = res.header::<ContentType>();
    let ctype = ctype.unwrap();
    assert_eq!(ctype.0.type_(), ty);
    assert_eq!(ctype.0.subtype(), sub);
}
pub fn check_content(res: &Response, content: &str) {
    let res_content = res.to_str();
    assert_eq!(res_content.unwrap(), content);
}
