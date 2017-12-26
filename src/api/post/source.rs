use std::fs::{File, OpenOptions};
use writium_cache::CacheSource;
use writium_framework::prelude::*;

const ERR_ACCESS: &'static str = "Cannot access to requested resource.";

pub struct DefaultSource {
    dir: String,
}
impl DefaultSource {
    pub fn new(dir: &str) -> DefaultSource {
        DefaultSource {
            dir: dir.to_string(),
        }
    }
    fn open_content(&self, id: &str, read: bool) -> ::std::io::Result<File> {
        use std::fs::create_dir_all;
        info!("Try openning file of ID: {}", id);
        let mut path = path_buf![&self.dir, id];
        if !read && !path.exists() {
            create_dir_all(&path)?;
        }
        path.push("content.md");
        OpenOptions::new()
            .read(read)
            .write(!read)
            .create(!read)
            .open(path)
    }
}
impl CacheSource for DefaultSource {
    type Value = String;
    fn load(&self, id: &str, create: bool) -> Result<String> {
        use std::io::{Read, BufReader};
        let r_content = self.open_content(id, true)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
            .and_then(|file| {
                let mut reader = BufReader::new(file);
                // Find the title line.
                let mut post = String::new();
                reader.read_to_string(&mut post)
                    .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
                // Convert Markdown to HTML only when it's needed. Allow new posts to be
                // published.
                Ok(post)
            });
        if r_content.is_err() && create {
            Ok(String::new())
        } else {
            r_content
        }
    }
    fn unload(&self, id: &str, val: &String) -> Result<()> {
        use std::io::{Write, BufWriter};
        trace!("Unloading {}", id);
        if let Ok(file) = self.open_content(id, false) {
            let mut writer = BufWriter::new(file);
            if writer.write_all(val.as_bytes()).is_ok() {
                // Convert Markdown to HTML only when it's needed. Allow new posts
                // to be published.
                trace!("Successed {}", id);
               Ok(())
            } else {
               Err(Error::internal(ERR_ACCESS))
            }
        } else {
            // No local file found, it's calling for creating a new resource.
           Err(Error::internal(ERR_ACCESS))
        }
    }
    fn remove(&self, id: &str) -> Result<()> {
        use std::fs::remove_file;
        if remove_file(path_buf![&self.dir, id, "content.md"]).is_err() {
           Err(Error::internal(ERR_ACCESS))
        } else {
           Ok(())
        }
    }
}
