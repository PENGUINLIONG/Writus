use std::collections::BTreeMap;
use std::io::{BufReader, BufWriter};
use std::fs::{File, OpenOptions};
use serde_json as json;
use writium_cache::CacheSource;
use writium_framework::prelude::*;
use super::Comment;

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
    fn open_comment(&self, id: &str, read: bool, create: bool) -> ::std::io::Result<File> {
        info!("Try openning file of ID: {}", id);
        OpenOptions::new()
            .create(create)
            .read(read)
            .write(!read)
            .open(path_buf![&self.dir, id, "comments.json"])
    }
}
impl CacheSource for DefaultSource {
    type Value = BTreeMap<usize, Comment>;
    fn load(&self, id: &str, create: bool) -> Result<Self::Value> {
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
    fn unload(&self, id: &str, obj: &Self::Value) -> Result<()> {
        let file = self.open_comment(id, false, true)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))?;
        let writer = BufWriter::new(file);
        json::to_writer_pretty(writer, obj)
            .map_err(|err| Error::internal(ERR_ACCESS).with_cause(err))
    }
}
