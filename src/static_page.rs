use std::io::{Read, BufReader};
use std::fs::File;
use writium::prelude::*;

pub struct StaticPage {
    #[allow(dead_code)]
    name: Vec<String>,
    name_ref: Vec<&'static str>,
    html: String,
    accept_tailing_segs: bool,
}
impl StaticPage {
    pub fn new(name: &str, html: String) -> StaticPage {
        fn make_ref(name: &str) -> &'static str {
            unsafe {
                let name_ref: &str = name;
                let name_ref = name_ref as *const str;
                &*name_ref
            }
        }
        if !name.starts_with('/') {
            panic!("Invalid static page path: {}", name);
        }
        let mut segs: Vec<String> = name[1..]
            .split('/')
            .map(|x| x.to_owned())
            .collect();
        // Automatically enable trailing segs.
        let acpt_trail = if let Some((last, _)) = segs.split_last() {
            if last == "..." { true } else { false }
        } else {
            false
        };
        if acpt_trail {
            segs.pop();
        }
        let mut name_ref = Vec::with_capacity(name.len());
        for seg in segs.iter() {
            name_ref.push(make_ref(seg));
        }
        StaticPage {
            name: segs,
            name_ref: name_ref,
            html: html,
            accept_tailing_segs: acpt_trail,
        }
    }
    pub fn from_file(name: &str, path: &str) -> ::std::io::Result<StaticPage> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        let rv = StaticPage::new(name, buf);
        Ok(rv)
    }
    /// Allow requests to be accepted if there are trailing path segments
    /// present in requested URI, this could be useful when those path segments
    /// are processed by the client browsers through JavaScript. By default,
    /// it's enabled when you provide a `name` followed by "/..." creating a new
    /// `StaticPage`, through `new()` or `from_file()`.
    pub fn accept_tailing_segs(&mut self) {
        self.accept_tailing_segs = true;
    }
}
impl Api for StaticPage {
    fn name(&self) -> &[&str] {
        &self.name_ref
    }
    fn route(&self, req: &mut Request) -> ApiResult {
        use self::header::{Allow, ContentType};
        match req.method() {
            Method::Get => {
                if req.path_segs().len() != 0 &&
                    self.accept_tailing_segs {
                    return Err(Error::not_found("Unexpected trailing error."))
                }
                let res = Response::new()
                    .with_header(ContentType("text/html; charset=UTF-8".parse().unwrap()))
                    .with_body(self.html.as_bytes());
                Ok(res)
            },
            Method::Options => {
                let res = Response::new()
                    .with_header(Allow(vec![Method::Options, Method::Get]));
                Ok(res)
            },
            _ => Err(Error::method_not_allowed())
        }
    }
}
