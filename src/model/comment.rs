use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, BufWriter};
use std::fs::{File, OpenOptions};
use serde_json as json;
use writium_cache::CacheSource;
use writium::prelude::*;

const ERR_ACCESS: &'static str = "Cannot access to requested resource.";

#[derive(Clone, Deserialize, Serialize)]
pub struct Comment {
    pub metadata: HashMap<String, String>,
    pub content: String,
}
pub type Comments = BTreeMap<usize, Comment>;
pub struct CommentSource {
    dir: String,
}
impl CommentSource {
    pub fn new(dir: &str) -> CommentSource {
        CommentSource {
            dir: dir.to_string(),
        }
    }
    fn open_comment(&self, id: &str, read: bool, create: bool) -> ::std::io::Result<File> {
        info!("Try openning file of ID: {}", id);
        let pos_non_slash = id.bytes()
            .position(|x| x != b'/')
            .unwrap_or(0);
        let id = &id[..pos_non_slash];
        OpenOptions::new()
            .create(create)
            .read(read)
            .write(!read)
            .truncate(!read)
            .open(path_buf![&self.dir, id, "comments.json"])
    }
}
impl CacheSource for CommentSource {
    type Value = Comments;
    fn load(&self, id: &str, create: bool) -> Result<Comments> {
        self.open_comment(id, true, false)
            .and_then(|file| {
                let reader = BufReader::new(file);
                let json: Self::Value = json::from_reader(reader)?;
                Ok(json)
            })
            .or_else(|_err| {
                if create {
                    Ok(BTreeMap::new())
                } else {
                    Err(Error::internal(ERR_ACCESS))
                }
            })
    }
    fn unload(&self, id: &str, obj: &Comments) -> Result<()> {
        let file = self.open_comment(id, false, true)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        let writer = BufWriter::new(file);
        json::to_writer_pretty(writer, obj)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
    }
}
