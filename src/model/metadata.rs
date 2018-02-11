use writium::prelude::*;
use writium_cache::CacheSource;
use serde_json::Value as JsonValue;
use super::FileAccessor;

const ERR_IO: &str = "Resource accessed but error occured during IO.";
const ERR_BROKEN_JSON: &str = "Local JSON file is broken. Try replacing the\
    invalid data before other operations.";

pub struct MetadataSource {
    accessor: FileAccessor,
}
impl MetadataSource {
    pub fn new(dir: &str) -> MetadataSource {
        MetadataSource {
            accessor: FileAccessor::with_fixed_file_name(dir, "metadata.json"),
        }
    }
}
impl CacheSource for MetadataSource {
    type Value = JsonValue;
    fn load(&self, id:&str, create: bool) -> Result<Self::Value> {
        use std::io::Read;
        let mut reader = match self.accessor.read(id) {
            Ok(rd) => rd,
            Err(err) => return if create {
                Ok(json!({}))
            } else {
                Err(err)
            }
        };
        let mut json_vec = Vec::new();
        reader.read_to_end(&mut json_vec)
            .map_err(|err| Error::internal(ERR_IO).with_cause(err))?;
        match ::serde_json::from_slice::<JsonValue>(&json_vec) {
            Ok(json) => if json.is_object() {
                Ok(json)
            } else if create{
                warn!("JSON in '{}' is valid but is not an object. `create` \
                    flag is on, so a new value will replace the invalid data.",
                    id);
                Ok(json!({}))
            } else {
                Err(Error::internal(ERR_BROKEN_JSON))
            },
            Err(err) => if create {
                warn!("JSON in '{}' is broken and `create` flag is on. A new \
                    value will replace the invalid data.", id);
                Ok(json!({}))
            } else {
                Err(Error::internal(ERR_BROKEN_JSON).with_cause(err))
            },
        }
    }
    fn unload(&self, id: &str, val: &Self::Value) -> Result<()> {
        let writer = self.accessor.write(id)?;
        ::serde_json::to_writer_pretty(writer, val)
            .map_err(|err| Error::internal(ERR_IO).with_cause(err))
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.accessor.remove(id)
    }
}
