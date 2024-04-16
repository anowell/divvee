use crate::{bail, error, error::repo_error, error::Error, Result};
use canonical_path::CanonicalPathBuf;
use chrono::{DateTime, Utc};
use libpijul::change::{Author, ChangeHeader, LocalChange};
use libpijul::changestore::{self, filesystem, ChangeStore};
use libpijul::key::{PublicKey, SecretKey};
use libpijul::pristine::sanakirja::{Pristine, Txn};
use libpijul::pristine::{self, TreeTxnT, TxnT};
use libpijul::{
    working_copy, Base32, ChannelMutTxnT, ChannelTxnT, DepsTxnT, GraphTxnT, MutTxnTExt, RevLog,
    TxnTExt, DOT_DIR,
};
use log::{debug, warn};
use owning_ref::{BoxRef, OwningHandle};
use serde::Deserialize;
use std::env::{self, current_dir};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const IDENTITY_NAME: &str = "default";

#[derive(Clone)]
pub struct Repository {
    pub pristine: Arc<Pristine>,
    pub changes: changestore::filesystem::FileSystem,
    pub working_copy: working_copy::filesystem::FileSystem,
    // pub config: config::Config,
    pub path: PathBuf,
    pub changes_dir: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct ChangeInfo {
    pub hash: String,
    // TODO: use a struct with name and email fields
    pub authors: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Identity {
    pub display_name: String,
    pub email: String,
    pub public_key: PublicKey,
    secret_key: Option<SecretKey>,
}

// TODO: use global config dir
fn global_ident_path() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap()).join(".config/pijul/identities")
}

impl Identity {
    pub fn load_global(identity_name: &str) -> Result<Self> {
        let identity_path = global_ident_path().join(identity_name);
        debug!(
            "Identity {} from {}",
            identity_name,
            identity_path.display()
        );
        let text = fs::read_to_string(identity_path.join("identity.toml"))?;
        let mut identity: Identity = toml::from_str(&text)?;

        let identity_text = fs::read_to_string(identity_path.join("secret_key.json"))?;
        identity.secret_key = serde_json::from_str(&identity_text)?;

        Ok(identity)
    }

    pub fn load_by_key(repo_path: &Path, pub_key: &str) -> Result<Self> {
        let identities_path = repo_path.join(DOT_DIR).join("identities");
        std::fs::create_dir_all(&identities_path)?;

        let f = fs::File::open(identities_path.join(pub_key))?;
        let identity: Identity = serde_json::from_reader(&f)?;

        Ok(identity)
    }
}

pub const DEFAULT_CHANNEL: &str = "main";
pub const PRISTINE_DIR: &str = "pristine";
pub const CHANGES_DIR: &str = "changes";
pub const CONFIG_FILE: &str = "config";
// const DEFAULT_IGNORE: [&[u8]; 2] = [b".git", b".DS_Store"];

impl Repository {
    fn find_dot_dir(cur: Option<PathBuf>) -> Result<PathBuf> {
        let mut cur = if let Some(cur) = cur {
            cur
        } else {
            current_dir()?
        };
        cur.push(DOT_DIR);
        loop {
            debug!("{:?}", cur);
            if std::fs::metadata(&cur).is_err() {
                cur.pop();
                if cur.pop() {
                    cur.push(DOT_DIR);
                } else {
                    bail!("No Pijul repository found");
                }
            } else {
                break;
            }
        }
        Ok(cur)
    }

    pub fn find_root(cur: Option<PathBuf>) -> Result<Repository> {
        let cur = Self::find_dot_dir(cur)?;
        let mut pristine_dir = cur.clone();
        pristine_dir.push(PRISTINE_DIR);
        let mut changes_dir = cur.clone();
        changes_dir.push(CHANGES_DIR);
        let mut working_copy_dir = cur.clone();
        working_copy_dir.pop();

        Ok(Repository {
            pristine: Arc::new(Pristine::new(&pristine_dir.join("db")).map_err(repo_error)?),
            working_copy: working_copy::filesystem::FileSystem::from_root(&working_copy_dir),
            changes: changestore::filesystem::FileSystem::from_root(
                &working_copy_dir,
                1, // crate::repository::max_files(),
            ),
            // config,
            path: working_copy_dir,
            changes_dir,
        })
    }

    pub fn add_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        debug!("add_file {}", path.as_ref().display());
        let txn = self.pristine.arc_txn_begin().map_err(repo_error)?;
        let repo_path = CanonicalPathBuf::canonicalize(&self.path)?;
        let path = repo_path.join(path)?;
        debug!("{}", path.display());
        let meta = std::fs::metadata(&path)?;
        debug!("{:?}", meta);

        if !working_copy::filesystem::filter_ignore(
            repo_path.as_ref(),
            path.as_ref(),
            meta.is_dir(),
        ) {
            bail!("Invalid path");
        }

        {
            let mut txn = txn.write();
            let path = if let Ok(path) = path.as_path().strip_prefix(&repo_path.as_path()) {
                path
            } else {
                panic!("TODO: figure out why we got here");
            };

            // use path_slash::PathExt;
            let path_str = path.to_string_lossy(); //path.to_slash_lossy();

            if !txn.is_tracked(&path_str).map_err(repo_error)? {
                txn.add(&path_str, meta.is_dir(), 0).map_err(repo_error)?;
            }
        }

        txn.commit().map_err(repo_error)?;
        debug!("Tracked {}", path.display());

        Ok(())
    }

    pub fn record(&mut self, msg: &str) -> Result<()> {
        let txn = self.pristine.arc_txn_begin().map_err(repo_error)?;

        let channel = DEFAULT_CHANNEL.to_string();
        let mut channel =
            if let Some(channel) = txn.read().load_channel(&channel).map_err(repo_error)? {
                channel
            } else {
                bail!("Channel {:?} not found", channel);
            };

        // Create record headers
        let ident = Identity::load_global(IDENTITY_NAME)?;
        let author = Author([("key".to_string(), ident.public_key.key)].into());
        let header = ChangeHeader {
            message: msg.to_string(),
            authors: vec![author],
            description: None,
            timestamp: Utc::now(),
        };

        // let repo_path = CanonicalPathBuf::canonicalize(&self.path)?;
        let key = ident.secret_key.unwrap().load(None).map_err(repo_error)?;
        txn.write()
            .apply_root_change_if_needed(&self.changes, &channel, rand::thread_rng())
            .map_err(repo_error)?;

        // let result = record(
        //     txn,
        //     channel.clone(),
        //     &self.working_copy,
        //     &self.changes,
        //     // repo_path,
        //     header,
        // )?;

        let mut state = libpijul::RecordBuilder::new();
        state
            .record(
                txn.clone(),
                libpijul::Algorithm::default(),
                false,
                &libpijul::DEFAULT_SEPARATOR,
                channel.clone(),
                &self.working_copy,
                &self.changes,
                "",
                1, // num_cpus::get(),
            )
            .map_err(repo_error)?;

        let rec = state.finish();
        if rec.actions.is_empty() {
            txn.write().touch_channel(&mut *channel.write(), None);
            txn.commit().map_err(repo_error)?;
            warn!("Nothing to record");
            return Ok(());
        }

        let mut change = {
            debug!("TAKING LOCK {}", line!());
            let txn_ = txn.write();
            let actions = rec
                .actions
                .into_iter()
                .map(|rec| rec.globalize(&*txn_).unwrap())
                .collect();
            let contents = if let Ok(c) = Arc::try_unwrap(rec.contents) {
                c.into_inner()
            } else {
                unreachable!()
            };
            let change =
                LocalChange::make_change(&*txn_, &channel, actions, contents, header, Vec::new())
                    .map_err(repo_error)?;

            debug!("has_binary = {:?}", rec.has_binary_files);

            // let current: HashSet<_> = change.dependencies.iter().cloned().collect();

            if change.header.message.trim().is_empty() {
                bail!("No change message");
            }
            debug!("saving change");
            std::mem::drop(txn_);
            change
        };

        let (updates, oldest) = (rec.updatables, rec.oldest_change);

        let hash = self
            .changes
            .save_change(&mut change, |change, hash| {
                change.unhashed = Some(serde_json::json!({
                    "signature": key.sign_raw(&hash.to_bytes()).unwrap(),
                }));
                Ok::<_, filesystem::Error>(())
            })
            .map_err(repo_error)?;

        let mut txn_ = txn.write();
        txn_.apply_local_change(&mut channel, &change, &hash, &updates)
            .map_err(repo_error)?;

        // let mut path = self.path.join(libpijul::DOT_DIR);
        // path.push("identities");
        // std::fs::create_dir_all(&path)?;

        debug!("Hash: {}", hash.to_base32());
        debug!("oldest = {:?}", oldest);
        let mut oldest = oldest
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        if oldest == 0 {
            // If no diff was done at all, it means that no
            // existing file changed since last time (some
            // files may have been added, deleted or moved,
            // but `touch` isn't about those).
            oldest = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
        }
        txn_.touch_channel(&mut *channel.write(), Some((oldest / 1000) * 1000));
        std::mem::drop(txn_);
        txn.commit().map_err(repo_error)?;
        Ok(())
    }

    pub fn change(&self, hash: &libpijul::Hash) -> Result<ChangeInfo> {
        let header = self.changes.get_header(&hash).map_err(repo_error)?;
        let authors = header
            .authors
            .into_iter()
            .filter_map(|auth| auth.0.get("key").cloned())
            .map(|key| {
                let mut author = None;
                if let Ok(ident) = Identity::load_global(IDENTITY_NAME) {
                    if ident.public_key.key == key {
                        author = Some(ident.email.clone());
                    }
                }
                if author.is_none() {
                    author = Identity::load_by_key(&self.path, &key)
                        .map(|i| i.email.clone())
                        .ok();
                }
                author.unwrap()
            })
            .collect();
        Ok(ChangeInfo {
            hash: hash.to_base32(),
            authors,
            timestamp: header.timestamp,
            message: header.message.clone(),
        })
    }

    pub fn changes<'txn>(&self, path: &Path) -> Result<LogIter<'txn>> {
        let txn = self.pristine.txn_begin().map_err(repo_error)?;
        let channel_name = txn.current_channel().unwrap_or(DEFAULT_CHANNEL);
        let channel_ref = txn.load_channel(channel_name).map_err(repo_error)?.unwrap();
        let inode = libpijul::fs::find_inode(&txn, path.to_str().unwrap()).map_err(repo_error)?;
        let inode_position = txn
            .get_inodes(&inode, None)
            .map_err(repo_error)?
            .unwrap()
            .clone();

        let ref_txn = Box::new(txn);
        let revlog = OwningHandle::new_with_fn(BoxRef::new(ref_txn), |txn| {
            Box::new(
                unsafe { txn.as_ref() }
                    .unwrap()
                    .reverse_log(&*channel_ref.read(), None)
                    .unwrap(),
            )
        });

        Ok(LogIter {
            revlog,
            inode_position,
        })
    }

    pub fn first_and_last_changes(&self, path: &Path) -> Result<(libpijul::Hash, libpijul::Hash)> {
        let txn = self.pristine.txn_begin().map_err(repo_error)?;
        let channel_name = txn.current_channel().unwrap_or(DEFAULT_CHANNEL);
        let channel_ref = txn.load_channel(channel_name).map_err(repo_error)?.unwrap();
        let inode = libpijul::fs::find_inode(&txn, path.to_str().unwrap()).map_err(repo_error)?;
        let inode_position = txn
            .get_inodes(&inode, None)
            .map_err(repo_error)?
            .unwrap()
            .clone();

        let channel = &*channel_ref.read();
        let last_modified = txn.last_modified(&channel);

        let mut log = txn.log_for_path(channel, inode_position, 0).unwrap();
        let first = log.next().unwrap().map_err(repo_error)?;
        let mut revlog = txn
            .rev_log_for_path(channel, inode_position, last_modified)
            .unwrap();
        let last = revlog.next().unwrap().map_err(repo_error)?;
        Ok((first, last))
    }

    // pub fn txn(&self) -> Result<Txn> {
    //     let txn = self.pristine.txn_begin().map_err(repo_error)?;
    //     Ok(txn)
    // }
}

// pub struct Transaction {
//     txn: Txn,
// }

// impl Transaction {
//     pub fn changes<'txn>(&'txn self, path: &Path) -> Result<ChangeIter<'txn>> {
//         let channel_name = self.txn.current_channel().unwrap_or(DEFAULT_CHANNEL);
//         let channel_ref = self
//             .txn
//             .load_channel(channel_name)
//             .map_err(repo_error)?
//             .unwrap();
//         let inode =
//             libpijul::fs::find_inode(&self.txn, path.to_str().unwrap()).map_err(repo_error)?;
//         let inode_position = self
//             .txn
//             .get_inodes(&inode, None)
//             .map_err(repo_error)?
//             .unwrap()
//             .clone();

//         let revlog = self.txn.reverse_log(&*channel_ref.read(), None).unwrap();
//         Ok(ChangeIter {
//             revlog,
//             // inode,
//             inode_position,
//             txn: &self.txn,
//         })
//     }
// }

// pub struct ChangeIter<'txn> {
//     txn: &'txn Txn,
//     // channel_ref: &'txn libpijul::ChannelRef<Txn>,
//     revlog: RevLog<'txn, Txn>,
//     // inode: libpijul::Inode,
//     inode_position: pristine::Position<libpijul::ChangeId>,
// }

// impl<'txn> ChangeIter<'txn> {
//     pub(crate) fn new(txn: &'txn Txn, path: &Path) -> Result<ChangeIter<'txn>> {
//         let channel_name = txn.current_channel().unwrap_or(DEFAULT_CHANNEL);
//         let channel_ref = txn.load_channel(channel_name).map_err(repo_error)?.unwrap();
//         let inode = libpijul::fs::find_inode(txn, path.to_str().unwrap()).map_err(repo_error)?;
//         let inode_position = txn
//             .get_inodes(&inode, None)
//             .map_err(repo_error)?
//             .unwrap()
//             .clone();

//         let revlog = txn.reverse_log(&*channel_ref.read(), None).unwrap();
//         Ok(ChangeIter {
//             revlog,
//             // inode,
//             inode_position,
//             txn,
//         })
//     }
// }

// impl<'txn> Iterator for ChangeIter<'txn> {
//     type Item = Result<libpijul::Hash>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let txn = self.txn;

//         while let Some(pr) = self.revlog.next() {
//             let (_, (hash, _mrk)) = match pr {
//                 Ok(pr) => pr,
//                 Err(err) => return Some(Err(repo_error(err))),
//             };
//             let change_id = match txn.get_internal(hash) {
//                 Ok(Some(cid)) => cid,
//                 Ok(None) => return Some(Err(error::msg("No change found for hash"))),
//                 Err(err) => return Some(Err(repo_error(err))),
//             };
//             let touches_file = txn
//                 .get_touched_files(&self.inode_position, Some(change_id))
//                 .unwrap_or_default()
//                 == Some(change_id);

//             if touches_file {
//                 return Some(Ok(hash.into()));
//             }
//         }
//         None
//     }
// }

pub struct LogIter<'txn> {
    revlog: OwningHandle<BoxRef<Txn>, Box<RevLog<'txn, Txn>>>,
    inode_position: pristine::Position<libpijul::ChangeId>,
}

impl<'txn> Iterator for LogIter<'txn> {
    type Item = Result<libpijul::Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(pr) = self.revlog.next() {
            let (_, (hash, _mrk)) = match pr {
                Ok(pr) => pr,
                Err(err) => return Some(Err(repo_error(err))),
            };

            let txn = self.revlog.as_owner();
            let change_id = match txn.get_internal(hash) {
                Ok(Some(cid)) => cid,
                Ok(None) => return Some(Err(error::msg("No change found for hash"))),
                Err(err) => return Some(Err(repo_error(err))),
            };
            let touches_file = txn
                .get_touched_files(&self.inode_position, Some(change_id))
                .unwrap_or_default()
                == Some(change_id);

            if touches_file {
                return Some(Ok(hash.into()));
            }
        }
        None
    }
}
