use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use writium_cache::CacheSource;
use writium::prelude::*;
use api::comment::Comment;

fn make_comment(author: &str, content: &str) -> Comment {
    Comment {
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("author".to_owned(), author.to_owned());
            meta
        },
        content: content.to_owned(),
    }
}
fn make_article(author_content: &[(&str, &str)]) -> BTreeMap<usize, Comment> {
    let mut article = BTreeMap::new();
    let mut index = 0;
    for &(ref author, ref content) in author_content {
        article.insert(index, make_comment(author, content));
        index += 2;
    }
    article
}
pub struct MockSource(Mutex<HashMap<String, BTreeMap<usize, Comment>>>);
impl MockSource {
    /// Make a source with article `foo` loaded, the article contains 1 comment
    /// with metadata `author` equals `PENGUINLIONG` and content `Wow!`.
    pub fn new() -> MockSource {
        let mut map = HashMap::new();
        map.insert("foo".to_owned(), make_article(&[("PENGUINLIONG", "Wow!")]));

        MockSource(Mutex::new(map))
    }
    pub fn new_privilege() -> MockSource {
        let mut map = HashMap::new();
        let mut article = make_article(&[("PENGUINLIONG", "Wow!")]);
        article.get_mut(&0).unwrap()
            .metadata
            .insert("privilege".to_owned(), "POWER!".to_owned());
        map.insert("foo".to_owned(), article);

        MockSource(Mutex::new(map))
    }
    pub fn many_comments() -> MockSource {
        let mut map = HashMap::new();
        map.insert("foo".to_owned(), make_article(&[
            ("PENGUINLIONG", "Wow!"),
            ("NOTLIONG", "Well."),
            ("LIONG", ":/"),
        ]));

        MockSource(Mutex::new(map))
    }
}
impl CacheSource for MockSource {
    type Value = BTreeMap<usize, Comment>;
    fn load(&self, id: &str, create: bool) -> Result<Self::Value> {
        if let Some(item) = self.0.lock().unwrap().get(id) {
            Ok(item.clone())
        } else if create {
            Ok(BTreeMap::new())
        } else {
            Err(Error::not_found("x"))
        }
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.0.lock().unwrap().remove(id);
        Ok(())
    }
}
