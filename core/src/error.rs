use std::error::Error as StdError;
use std::fmt::{self, Debug};
use std::io;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(#[from] io::Error),
    #[error("Error recording changes: {0}")]
    RepoError(Arc<dyn StdError + Send + Sync>),
    #[error("Database error: {0}")]
    DbError(#[from] sqlx::Error),
    // #[error("Repo Error: {0}")]
    // RepoError(#[from] libpijul::Error),
    #[error("Serialization Error: {0}")]
    SerError(Box<dyn StdError + 'static + Send + Sync>),
    #[error("Deserialization Error: {0}")]
    DeserError(Box<dyn StdError + 'static + Send + Sync>),
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader { expected: String, found: String },
    #[error("unknown error")]
    Unknown,
    #[error("{0}")]
    Msg(String),
}

macro_rules! error_convert {
    // ($($path:ident)::+<$($generic_param:ident $( : $bounds:tt )? ),*>, $dest:expr) => {
    //     impl<$($generic_param),*> From<$($path)::*<$($generic_param),*>> for Error
    //     where
    //         $($generic_param : $($bounds)*, )*
    //     {
    //         fn from(error: $($path)::*<$($generic_param),*>) -> Self {
    //             $dest(Box::new(error))
    //         }
    //     }
    // };

    // Non-generic case
    ($type:ty, $dest:expr) => {
        impl From<$type> for Error {
            fn from(error: $type) -> Self {
                $dest(Box::new(error))
            }
        }
    };
}

// Almost: missing 'static restriction for generic arg
// use libpijul::TreeTxnT;
// error_convert!(libpijul::fs::FsError<T: TreeTxnT>, Error::RepoError);

error_convert!(toml::de::Error, Error::DeserError);
error_convert!(serde_json::Error, Error::DeserError);
error_convert!(serde_yaml::Error, Error::DeserError);
// error_convert!(
//     libpijul::pristine::sanakirja::SanakirjaError,
//     Error::RepoError
// );

// impl<C, W, T> From<libpijul::record::RecordError<C, W, T>> for Error
// where
//     C: StdError + 'static,
//     W: StdError + 'static,
//     T: libpijul::GraphTxnT + libpijul::TreeTxnT + 'static,
// {
//     fn from(error: libpijul::record::RecordError<C, W, T>) -> Self {
//         Error::RepoError(Arc::new(Box::new(error)))
//     }
// }

// impl<T> From<libpijul::fs::FsError<T>> for Error
// where
//     T: libpijul::TreeTxnT + 'static,
// {
//     fn from(error: libpijul::fs::FsError<T>) -> Self {
//         Error::RepoError(Box::new(error))
//     }
// }

// impl From<libpijul::pristine::sanakirja::SanakirjaError> for Error {
//     fn from(error: libpijul::pristine::sanakirja::SanakirjaError) -> Self {
//         Error::RepoError(Box::new(error))
//     }
// }

// impl<E> From<libpijul::pristine::TxnErr<E>> for Error
// where
//     E: StdError + Debug + 'static,
// {
//     fn from(error: libpijul::pristine::TxnErr<E>) -> Self {
//         Error::RepoError(Box::new(error))
//     }
// }

// impl<C, T> From<libpijul::ApplyError<C, T>> for Error
// where
//     C: StdError,
//     T: libpijul::pristine::GraphTxnT + libpijul::pristine::TreeTxnT,
// {
//     fn from(error: libpijul::ApplyError<C, T>) -> Self {
//         Error::RepoError(Box::new(error))
//     }
// }

pub(crate) fn repo_error(error: impl StdError + 'static + Send + Sync) -> Error {
    Error::RepoError(Arc::new(error))
}

pub(crate) fn io_error<P: AsRef<Path>>(error_kind: io::ErrorKind, path: P) -> Error {
    io::Error::new(error_kind, path.as_ref().display().to_string()).into()
}

#[macro_export]
macro_rules! bail {
    // Usage: bail!("FORMAT", ARGS...)
    ($fmt:expr $(, $arg:expr)*) => {
        return Err(Error::Msg(format!($fmt $(, $arg)*)));
    };
}

pub fn msg(msg: impl fmt::Display) -> Error {
    Error::Msg(msg.to_string())
}
