use crate::Result;
use git2::{self, Blame, BlameOptions, Repository};
use std::path::Path;

pub struct Repo {
    repo: Repository,
}

impl Repo {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Repo> {
        let repo = Repository::open(path)?;
        Ok(Repo { repo })
    }

    pub fn blame_file<P: AsRef<Path>>(&self, path: P) -> Blame {
        // Create BlameOptions
        // let mut blame_opts = BlameOptions::new();
        // blame_opts.track_copies(true);

        // Open the file for blame for the entire document
        let blame = self.repo.blame_file(path.as_ref(), None).unwrap();

        blame
    }

    pub fn fetch(&mut self) -> Result<()> {
        todo!("impl fetch");
    }

    pub fn merge(&mut self) -> Result<()> {
        todo!("impl merge");
    }

    pub fn push(&mut self) -> Result<()> {
        todo!("impl push");
    }

    pub fn move_file<P: AsRef<Path>>(&mut self, src: P, dest: P) -> Result<()> {
        todo!("impl move");
    }

    pub fn add_file<P: AsRef<Path>>(&mut self, src: P) -> Result<()> {
        todo!("impl add_file");
    }

    pub fn remove_file<P: AsRef<Path>>(&mut self, src: P) -> Result<()> {
        todo!("impl remove_file");
    }
}
