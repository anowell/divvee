pub use error::Error;
// use itertools::Itertools;
use db::{Db, DbRecord};
use libpijul::Base32;
use log::debug;
use std::fs::{self, File};
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};

pub mod db;
pub mod error;
pub mod repo;
pub mod task;

pub type Result<T> = std::result::Result<T, error::Error>;
use repo::{ChangeInfo, Repository};

pub struct System {
    repo: Repository,
    db: Db,
}

pub trait RepoDoc: Sized {
    fn to_doc_string(&self) -> String;
    fn parse_doc(s: &str, path: Option<PathBuf>) -> Result<Self>;
}

pub trait Doc {
    type RepoDoc;
    // fn augment_doc(doc: RepoDoc, authorship: TBD)
}

pub struct MetaDoc<T> {
    pub created: ChangeInfo,
    pub updated: Option<ChangeInfo>,
    pub doc: T,
}

impl<T> Deref for MetaDoc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.doc
    }
}

impl RepoDoc for String {
    fn to_doc_string(&self) -> String {
        self.to_owned()
    }
    fn parse_doc(s: &str, _path: Option<PathBuf>) -> Result<Self> {
        Ok(s.to_owned())
    }
}

impl System {
    pub async fn init<P: AsRef<Path>>(repo_dir: P) -> Result<System> {
        let p = repo_dir.as_ref();
        let repo = Repository::find_root(Some(p.into()))?;
        let db = Db::connect(&p.join(".db.sqlite").to_string_lossy()).await?;
        Ok(System { repo, db })
    }

    pub fn next_id<P: AsRef<Path>>(&self, dir: &P) -> Result<u32> {
        let path = self.repo.path.join(dir);
        debug!("Looking up next_id in {}", path.display());
        let last = fs::read_dir(path)?
            .filter_map(|res| res.map(|e| e.path()).ok())
            .filter_map(|fpath| {
                fpath
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .split('-')
                    .nth(1)
                    .and_then(|s_num| s_num.parse::<u32>().ok())
            })
            .max();
        Ok(last.unwrap_or(0) + 1)
    }

    pub async fn create_doc<D: RepoDoc + DbRecord, P: AsRef<Path>>(
        &self,
        path: P,
        new_doc: D,
    ) -> Result<Document> {
        let path = path.as_ref();
        if self.repo.path.join(path).exists() {
            return Err(error::io_error(io::ErrorKind::AlreadyExists, path));
        }

        let mut doc = Document::new(self.repo.clone(), path)?;
        doc.write(&new_doc, true)?;

        // TODO: change this to read the full type for upsert
        let record = doc.read_doc::<D>()?;
        self.db.upsert_record(&record).await?;

        Ok(doc)
    }

    pub async fn update_doc<D: RepoDoc + DbRecord, P: AsRef<Path>>(
        &self,
        path: P,
        new_doc: D,
    ) -> Result<Document> {
        let path = path.as_ref();
        if !self.repo.path.join(path).exists() {
            return Err(error::io_error(io::ErrorKind::NotFound, path));
        }

        let mut doc = Document::new(self.repo.clone(), path)?;
        doc.write(&new_doc, true)?;

        // TODO: change this to read the full type for upsert
        let record = doc.read_doc::<D>()?;
        self.db.upsert_record(&record).await?;

        Ok(doc)
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Document> {
        Document::new(self.repo.clone(), path)
    }

    /// Reads a doc. Shorthand for load && parse when you don't need the Document for changes.
    pub fn read_doc<D: RepoDoc, P: AsRef<Path>>(&self, path: P) -> Result<D> {
        self.load(path)?.read_doc()
    }

    /// Reads a doc. Shorthand for load && parse when you don't need the Document for changes.
    pub fn read_doc_with_meta<D: RepoDoc, P: AsRef<Path>>(&self, path: P) -> Result<MetaDoc<D>> {
        self.load(path)?.read_doc_with_meta()
    }

    pub fn read_dir<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Document>> {
        let read_dir = fs::read_dir(self.repo.path.join(path)).map_err(Error::from)?;
        let docs: Vec<Document> = read_dir
            .map(|e| {
                let path = e
                    .unwrap()
                    .path()
                    .strip_prefix(&self.repo.path)
                    .unwrap()
                    .to_owned();
                Document::new(self.repo.clone(), path)
            })
            .collect::<Result<_>>()?;
        Ok(docs)
    }

    pub async fn reindex<D: RepoDoc + DbRecord>(&self, path: &Path) -> Result<()> {
        let doc = Document::new(self.repo.clone(), path)?;
        let record = doc.read_doc::<D>()?;
        self.db.upsert_record(&record).await?;
        Ok(())
    }

    pub async fn query<D: DbRecord>(&self, where_clause: &str) -> Result<Vec<D>> {
        self.db.query_dangerous::<D>(where_clause).await
    }

    // pub fn sync(&mut self, retry: u8) -> Result<()> {
    //     self.repo.fetch();
    //     if !self.repo.file_exist_conflicts.is_empty() {
    //         for f in self.repo.file_exist_conflicts() {
    //             let dest = self.repo.next_issue(proj);
    //             self.repo.move_file(f, dest);
    //         }
    //         self.repo.merge();
    //     }

    //     match self.repo.push {
    //         Err(ErrorKind::Conflict) if retry < 5 => {
    //             util::sleep(1_000);
    //             self.sync()?;
    //         }
    //         Err(err) => Err(err)
    //         Ok(_) => Ok(()),
    //     }
    // }
}

// pub struct Versioned<T> {
//     pub updated: DateTime<Utc>,
//     pub editor: String,
//     pub value: T,
// }

// impl<T> Deref for Versioned<T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         &self.value
//     }
// }

/// Handle to a single document in the repository
pub struct Document {
    // handle to the repo
    repo: Repository,
    // path to document file relative to repo root
    path: PathBuf,
}

impl Document {
    pub fn new<P: AsRef<Path>>(repo: Repository, path: P) -> Result<Document> {
        let file_path = repo.path.join(&path);
        let path = file_path.strip_prefix(&repo.path).unwrap().to_owned();

        Ok(Document { repo, path })
    }

    /// Returns the full path of this document
    pub fn canonical_path(&self) -> PathBuf {
        self.repo.path.join(&self.path)
    }

    /// Returns the path of this document relative to the repo root
    pub fn repo_path(&self) -> &Path {
        &self.path
    }

    /// Writes record to disk (truncates if existing)
    ///
    /// Optionally records the change in the repository
    pub fn write<D>(&mut self, doc: &D, record: bool) -> Result<()>
    where
        D: RepoDoc,
    {
        let file_path = self.canonical_path();
        debug!("Saving {}", file_path.display());

        let mut file = File::create(file_path)?;
        let md = doc.to_doc_string();
        file.write_all(md.as_bytes())?;

        if record {
            // TODO: only need to add if not already tracked
            self.repo.add_file(&self.path)?;

            let msg = format!(
                "Updated {}",
                self.path.file_name().unwrap().to_string_lossy()
            );
            self.repo.record(&msg)?;
        }

        Ok(())
    }

    // fn lines_intersecting(&self, span: Span) -> Option<RangeInclusive<usize>> {
    //     // Line numbers start counting at 1
    //     span.intersecting_positions(&self.line_spans)
    //         .map(|range| range.start() + 1..=range.end() + 1)
    // }

    pub fn read_to_string(&self) -> Result<String> {
        let path = self.canonical_path();
        fs::read_to_string(path).map_err(Error::from)
    }

    pub fn read_doc<D: RepoDoc>(&self) -> Result<D> {
        let doc_str = self.read_to_string()?;
        D::parse_doc(&doc_str, Some(self.path.clone()))
    }

    pub fn read_doc_with_meta<D: RepoDoc>(&self) -> Result<MetaDoc<D>> {
        let doc = self.read_doc()?;
        let (first, last) = self.repo.first_and_last_changes(&self.path)?;
        let created = self.repo.change(&first)?;
        let updated = match first == last {
            true => None,
            false => Some(self.repo.change(&last)?),
        };
        Ok(MetaDoc {
            created,
            updated,
            doc,
        })
    }

    // pub fn print_first_last(&self) -> Result<()> {
    //     let (first, last) = self.repo.first_and_last_changes(&self.path)?;
    //     // println!("FIRST: {}\nLAST : {}", first.to_base32(), last.to_base32());
    //     println!(
    //         "FIRST: {}\n\n{:?}\n",
    //         first.to_base32(),
    //         self.repo.change(&first)?
    //     );
    //     println!(
    //         "LAST : {}\n\n{:?}\n",
    //         last.to_base32(),
    //         self.repo.change(&last)?
    //     );
    //     Ok(())
    // }

    pub fn changes(&self) -> Result<impl Iterator<Item = Result<ChangeInfo>> + '_> {
        // let txn = self.repo.txn()?;
        let changes = self
            .repo
            .changes(&self.path)?
            .map(|h| h.and_then(|h| self.repo.change(&h)));

        // TODO add the diff as well because iterationg over changes is mostly useful with diff
        Ok(changes)
    }

    // pub fn credit<C>(&self) -> Result<C>
    // where
    //     C: CreditView,
    // {
    // }

    // pub fn history_iter(&self) -> impl Iterator<Item=Document> {

    // }

    // pub fn change_iter(&self) -> impl Iterator<Item=Change> {

    // }

    // fn get_versioned(&self, field: &str) -> Versioned<String> {
    //     let blame = self.repo.blame_file(&self.path);
    //     let issue_doc = self.doc.get("issue").unwrap();
    //     let node = issue_doc.children().unwrap().get(field).unwrap();
    //     let span = Span {
    //         offset: node.span().offset(),
    //         len: node.span().len(),
    //     };
    //     let lines = self.lines(span).unwrap();
    //     let hunk = blame
    //         .iter()
    //         // .inspect(|h| {
    //         //     eprintln!(
    //         //         "{}: start={} lines={}",
    //         //         h.final_commit_id(),
    //         //         h.final_start_line(),
    //         //         h.lines_in_hunk()
    //         //     );
    //         // })
    //         .find(|hunk| {
    //             let hunk_end_line = hunk.final_start_line() + hunk.lines_in_hunk() as usize;

    //             *lines.start() >= hunk.final_start_line() && *lines.start() < hunk_end_line
    //         })
    //         .unwrap();

    //     let sig = hunk.final_signature();
    //     Versioned {
    //         updated: DateTime::from_timestamp(sig.when().seconds(), 0).unwrap(),
    //         editor: sig.email().unwrap().to_owned(),
    //         value: node.get(0).unwrap().value().as_string().unwrap().to_owned(),
    //     }
    // }
}

// trait CreditView {
//     fn lines<T>(field: &str) -> impl Iterator<Item = (T, Credit)>;
// }

// struct Credit {
//     author: String,
//     updated: DateTime<Utc>,
// }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Span {
    offset: usize,
    len: usize,
}

// impl Span {
//     // assume `set` is an ordered, contiguous set of spans (panics otherwise)
//     // returns the range of indices where self overlaps spans in `set`
//     fn intersecting_positions(&self, set: &[Span]) -> Option<RangeInclusive<usize>> {
//         for (s1, s2) in set.iter().tuple_windows() {
//             assert_eq!(s1.offset + s1.len, s2.offset);
//         }

//         let mut iter = set.iter();
//         match iter
//             .position(|item| self.offset >= item.offset && self.offset < item.offset + item.len)
//         {
//             None => None,
//             Some(start) => {
//                 let end = iter
//                     .position(|&item| self.offset + self.len < item.offset)
//                     .map(|len| start + len)
//                     .unwrap_or_else(|| set.len() - 1);
//                 // git line numbers start at 1
//                 Some(start..=end)
//             }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::task::Task;
    use super::*;
    use tokio;

    #[tokio::test]
    async fn parse_issue() {
        let store = System::init("../repo").await.unwrap();
        let doc = store.load("DIST-1.md").unwrap().read_doc::<Task>().unwrap();
        assert_eq!(doc.title, "Define layout");
        assert_eq!(doc.status.unwrap(), "In-Progress");
        assert_eq!(doc.assignee.as_ref().unwrap(), "anowell");
        // assert_eq!(issue.title.editor, "anowell@gmail.com");
        // assert_eq!(issue.title.updated.to_string(), "2024-01-16 09:08:15 UTC");
        // assert_eq!(issue.status.updated.to_string(), "2024-01-16 23:21:45 UTC");
    }

    // #[test]
    // fn test_span_intersecting() {
    //     let span = |o, l| Span { offset: o, len: l };
    //     let set = vec![span(0, 12), span(12, 9), span(21, 5)];

    //     let intersecting = |o, l| span(o, l).intersecting_positions(&set).unwrap();
    //     assert_eq!(intersecting(0, 1), 0..=0);
    //     assert_eq!(intersecting(1, 10), 0..=0);
    //     assert_eq!(intersecting(10, 5), 0..=1);
    //     assert_eq!(intersecting(10, 12), 0..=2);
    //     assert_eq!(intersecting(12, 5), 1..=1);
    //     assert_eq!(intersecting(21, 5), 2..=2);
    // }
}
