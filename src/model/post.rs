use writium_cache::CacheSource;
use writium::prelude::*;
use super::FileAccessor;

const ERR_IO: &str = "Resource accessed but error occured during IO.";
const ERR_PARENT: &str = "Parent of requested post cannot be created. Maybe \
    there is a file occupying a segment of name in the path.";

pub struct PostSource {
    accessor: FileAccessor,
}
impl PostSource {
    pub fn new(dir: &str) -> PostSource {
        PostSource {
            accessor: FileAccessor::with_fixed_file_name(dir, "content.md"),
        }
    }
}
impl CacheSource for PostSource {
    type Value = String;
    fn load(&self, id: &str, create: bool) -> Result<String> {
        use std::io::Read;
        match self.accessor.read(id) {
            Ok(mut reader) => {
                let mut post = String::new();
                reader.read_to_string(&mut post)
                    .map(|_| post)
                    .map_err(|err| Error::internal(ERR_IO).with_cause(err))
                // Convert Markdown to HTML only when it's needed. So new posts
                // can be published.
            },
            Err(err) => if create {
                // Parent might not exist.
                if let Some(parent) = self.accessor.make_path(id).parent() {
                    // Create all directory so that all subsequent uploading of
                    // resources can be realized.
                    ::std::fs::create_dir_all(parent)
                        .map_err(|err| {
                            Error::internal(ERR_PARENT).with_cause(err)
                        })?;
                }
                Ok(String::new())
            } else {
                Err(err)
            },
        }
    }
    fn unload(&self, id: &str, val: &String) -> Result<()> {
        use std::io::Write;
        let mut writer = self.accessor.write(id)?;
        writer.write_all(val.as_bytes())
            .map_err(|err| Error::internal(ERR_IO).with_cause(err))
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.accessor.remove(id)
    }
}
