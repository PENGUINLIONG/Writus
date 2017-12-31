use std::collections::HashMap;
use std::sync::Arc;
use auth::SimpleAuthority;
use toml::Value as TomlValue;
use writium::hyper::mime::Mime;

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

fn raw_to_extra(extra: RawExtra) -> Extra {
    Extra {
        published_dir: extra.published_dir.unwrap_or("./published".to_string()),
        auth: if let Some(token) = extra.auth_token.as_ref() {
            Arc::new(SimpleAuthority::new(token))
        } else {
            Arc::new(SimpleAuthority::default())
        },
        index_key: extra.index_key.unwrap_or("published".to_string()),
        index_key_type: extra.index_key_type.unwrap_or("datetime".to_string()),
        entries_per_request: extra.entries_per_request.unwrap_or(5),
        allowed_exts: extra.allowed_exts.unwrap_or_default()
            .into_iter()
            .map(|(x, y)| (x, y.parse().expect("Unable to parse MIME in field `allowed_ext`.")))
            .collect(),
        template_dir: extra.template_dir.unwrap_or("./templates".to_owned()),
    }
}

pub fn convert_extra(extra: TomlValue) -> Extra {
    let extra: RawExtra = extra.try_into().expect("Unable to parse fields neccessary to Writium Blog API v1");
    raw_to_extra(extra)
}
