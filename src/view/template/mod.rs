use std::io::{Read, BufReader};
use std::fs::File;
use std::path::{Path, PathBuf};
use serde_json::Value as JsonValue;

mod section;
use self::section::*;

fn concat_subfragments(base: &Path, mut template: String) -> Result<String, String> {
    let mut rv = String::with_capacity(template.len());
    loop {
        if let Some(beg) = template.find("<?") {
            rv.extend(template.drain(..beg))
        } else {
            // No more processing instructions, get out of the loop.
            return Ok(rv + &template)
        }
        let end = template.find("?>")
            .ok_or("Unclosed processing instruction.".to_owned())?;
        let mut extend = false;
        if end < 2 {
            return Err("Tag beginning and ending overlaps.".to_owned())
        } else {
            let parts: Vec<&str> = template[2..end]
                .splitn(2, ' ')
                .collect();
            if parts.len() == 2 {
                if parts[0] == "frag" {
                    // Insert fragment.
                    let frag_path = parts[1].trim();
                    let subfrag_path = path_buf![&base, &frag_path];
                    rv += &load_fragement(base, &subfrag_path)?;
                } else if parts[0] == "var" {
                    // Keep variables for the next stage (compilation).
                    extend = true;
                }
            }
            // Ignore unknown processing instructions.
        }
        if extend {
            rv.extend(template.drain(..(end + 2)));
        } else {
            template.drain(..(end + 2));
        }
    }
}
fn load_fragement(base: &Path, file_path: &Path) -> Result<String, String> {
    let file = File::open(path_buf![&base, &file_path])
        .map_err(|err| format!("Unable to open template file: {}", err))?;
    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    reader.read_to_string(&mut buf)
        .map_err(|err| format!("Unable to read from template file: {}", err))?;
    concat_subfragments(base, buf)
}
fn compile(mut concated: String) -> Vec<Box<TemplateSection>> {
    let mut rv: Vec<Box<TemplateSection>> = Vec::new();
    loop {
        if let Some(beg) = concated.find("<?") {
            let string = concated.drain(..beg).collect();
            rv.push(Box::new(StringSection::new(string)));
        } else {
            // No more processing instructions, get out of the loop.
            rv.push(Box::new(StringSection::new(concated)));
            return rv
        }
        // There should be no invalid syntax present (after
        // concat_subfragments).
        let end = concated.find("?>").unwrap();
        {
            let parts: Vec<&str> = concated[2..end]
                .splitn(2, ' ')
                .collect();
            // `parts[0]` can be nothing other than 'var'.
            rv.push(Box::new(MetadataSection::new(parts[1].to_owned())));
            // Ignore unknown processing instructions.
        }
        concated.drain(..(end + 2));
    }
}

pub struct Template {
    sections: Vec<Box<TemplateSection>>,
}
impl Template {
    pub fn from_file(base: &str, path: &str) -> Option<Template> {
        info!("Loading template from file: {}", [base, path].join("/"));
        let concated = match load_fragement(&PathBuf::from(base), &PathBuf::from(path)) {
            Ok(concated) => concated,
            Err(err) => {
                error!("Cannot compile template: {}", err);
                return None
            },
        };
        Some(Template { sections: compile(concated) })
    }
    pub fn render(&self, meta: &JsonValue, extra: &[(&str, &str)]) -> String {
        let mut rv = String::new();
        for sec in self.sections.iter() {
            sec.get_section(meta, extra, &mut rv);
        }
        rv
    }
}
impl Default for Template {
    fn default() -> Template {
        Template {
            sections: Vec::new(),
        }
    }
}
