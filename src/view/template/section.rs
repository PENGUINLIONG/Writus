use serde_json::Value as JsonValue;

pub trait TemplateSection: Send + Sync {
    fn get_section(&self, content: &str, vars: &JsonValue, out: &mut String);
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
    fn get_section(&self, _: &str, _: &JsonValue, out: &mut String) {
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
    fn get_section(&self, _: &str, vars: &JsonValue, out: &mut String) {
        let var = match vars.get(&self.key) {
            Some(var) => var,
            None => return,
        };
        if let Some(string) = var.as_str() {
            out.push_str(string)
        } else {
            match ::serde_json::to_string(var) {
                Ok(s) => out.push_str(&s),
                Err(err) => out.push_str(&format!("!!{}!!", err)),
            }
        }
    }
}
pub struct ContentSection();
impl ContentSection {
    pub fn new() -> ContentSection {
        ContentSection()
    }
}
impl TemplateSection for ContentSection {
    fn get_section(&self, content: &str, _: &JsonValue, out: &mut String) {
        out.push_str(&content);
    }
}
