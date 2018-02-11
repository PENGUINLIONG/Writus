use writium_cache::CacheSource;
use writium::prelude::*;
use super::FileAccessor;

const ERR_IO: &str = "Resource accessed but error occured during IO.";

pub struct ResourceSource {
    accessor: FileAccessor,
}
impl ResourceSource {
    pub fn new(dir: &str) -> ResourceSource {
        ResourceSource {
            accessor: FileAccessor::new(dir),
        }
    }
}
impl CacheSource for ResourceSource {
    type Value = Vec<u8>;
    fn load(&self, id: &str, create: bool) -> Result<Vec<u8>> {
        use std::io::Read;
        let mut reader = match self.accessor.read(id) {
            Ok(rd) => rd,
            Err(err) => return if create {
                Ok(Vec::new())
            } else {
                Err(err)
            }
        };
        let mut buf = Vec::new();
        match reader.read_to_end(&mut buf) {
            Ok(_) => Ok(buf),
            Err(err) => if create {
                Ok(Vec::new())
            } else {
                Err(Error::internal(ERR_IO).with_cause(err))
            },
        }
    }
    fn unload(&self, id: &str, data: &Vec<u8>) -> Result<()> {
        use std::io::Write;
        let mut writer = self.accessor.write(id)?;
        writer.write_all(&data)
            .map_err(|err| Error::internal(ERR_IO).with_cause(err))
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.accessor.remove(id)
    }
}
