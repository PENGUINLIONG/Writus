use std::collections::{BTreeMap, HashMap};
use writium::prelude::*;
use writium_cache::CacheSource;
use super::FileAccessor;

const ERR_IO: &str = "Resource accessed but error occured during IO.";
const ERR_BROKEN_JSON: &str = "Local JSON file is broken. Try replacing the invalid data before other operations.";

#[derive(Clone, Deserialize, Serialize)]
pub struct Comment {
    pub metadata: HashMap<String, String>,
    pub content: String,
}
pub type Comments = BTreeMap<usize, Comment>;
pub struct CommentSource {
    accessor: FileAccessor,
}
impl CommentSource {
    pub fn new(dir: &str) -> CommentSource {
        CommentSource {
            accessor: FileAccessor::with_fixed_file_name(dir, "comments.json"),
        }
    }
}
impl CacheSource for CommentSource {
    type Value = Comments;
    fn load(&self, id: &str, create: bool) -> Result<Comments> {
        use std::io::Read;
        let mut reader = self.accessor.read(id)?;
        let mut json_vec = Vec::new();
        reader.read_to_end(&mut json_vec)
            .map_err(|err| Error::internal(ERR_BROKEN_JSON).with_cause(err))?;
        match ::serde_json::from_slice(&json_vec) {
            Ok(json) => Ok(json),
            Err(err) => if create {
                Ok(Comments::new())
            } else {
                Err(Error::internal(ERR_IO).with_cause(err))
            },
        }
    }
    fn unload(&self, id: &str, obj: &Comments) -> Result<()> {
        let writer = self.accessor.write(id)?;
        ::serde_json::to_writer_pretty(writer, obj)
            .map_err(|err| Error::internal(ERR_IO).with_cause(err))
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.accessor.remove(id)
    }
}
