use serde_json::Value as JsonValue;

pub trait TemplateSection: Send + Sync {
    fn get_section(&self, meta: &JsonValue, extra: &[(&str, &str)], out: &mut String);
}
pub struct StringSection {
    string: String
}
impl StringSection {
    pub fn new(string: String) -> StringSection {
        StringSection {
            string: string,
        }
    }
}
impl TemplateSection for StringSection {
    fn get_section(&self, _meta: &JsonValue, _extra: &[(&str, &str)], out: &mut String) {
        out.push_str(&self.string)
    }
}
pub struct MetadataSection {
    key: String,
}
impl MetadataSection {
    pub fn new(key: String) -> MetadataSection {
        MetadataSection {
            key: key,
        }
    }
}
impl TemplateSection for MetadataSection {
    fn get_section(&self, meta: &JsonValue, extra: &[(&str, &str)], out: &mut String) {
        if let Some(meta) = meta.get(&self.key) {
            if let Some(string) = meta.as_str() {
                out.push_str(string);
            } else if let Ok(string) = ::serde_json::to_string(meta) {
                out.push_str(&string);
            }
        } else if let Some(&(_, extra)) = extra.into_iter()
            .find(|&&(key, _)| key == &self.key) {
            out.push_str(extra);
        } else {
            // Do nothing when there is no such value.
        }
    }
}
