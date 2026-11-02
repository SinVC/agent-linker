use crate::core::symlink::SymlinkError;
use std::{
    error, fmt, io,
    path::{Path, PathBuf},
};
pub type Result<T> = std::result::Result<T, Error>;
type Cause = Box<dyn error::Error + Send + Sync>;

#[derive(Debug)]
pub enum Error {
    DatabaseMessage(String),
    Database {
        path: Option<PathBuf>,
        source: rusqlite::Error,
    },
    InvalidArguments(String),
    Manifest(String),
    NotFound(String),
    Conflict {
        message: String,
        path: Option<PathBuf>,
        source: Option<rusqlite::Error>,
    },
    DatabaseBusy {
        path: Option<PathBuf>,
        source: rusqlite::Error,
    },
    PermissionDenied {
        path: Option<PathBuf>,
        source: Cause,
    },
    Project(String),
    Symlink(SymlinkError),
    Io {
        path: Option<PathBuf>,
        source: io::Error,
    },
    Parse {
        path: Option<PathBuf>,
        source: Cause,
    },
}

impl Error {
    pub fn database(message: impl Into<String>) -> Self {
        Self::DatabaseMessage(message.into())
    }
    pub fn invalid_arguments(message: impl Into<String>) -> Self {
        Self::InvalidArguments(message.into())
    }
    pub fn manifest(message: impl Into<String>) -> Self {
        Self::Manifest(message.into())
    }
    pub fn project(message: impl Into<String>) -> Self {
        Self::Project(message.into())
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
            path: None,
            source: None,
        }
    }
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::from(source).at_path(path)
    }
    pub fn parse(source: impl error::Error + Send + Sync + 'static) -> Self {
        Self::Parse {
            path: None,
            source: Box::new(source),
        }
    }
    pub fn at_path(mut self, value: impl Into<PathBuf>) -> Self {
        match &mut self {
            Self::Database { path, .. }
            | Self::DatabaseBusy { path, .. }
            | Self::PermissionDenied { path, .. }
            | Self::Io { path, .. }
            | Self::Parse { path, .. }
            | Self::Conflict { path, .. } => *path = Some(value.into()),
            Self::DatabaseMessage(message) | Self::Manifest(message) => {
                *message = format!("{}: {message}", value.into().display())
            }
            _ => {}
        }
        self
    }
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Database { path, .. }
            | Self::DatabaseBusy { path, .. }
            | Self::PermissionDenied { path, .. }
            | Self::Io { path, .. }
            | Self::Parse { path, .. }
            | Self::Conflict { path, .. } => path.as_deref(),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseMessage(msg)
            | Self::InvalidArguments(msg)
            | Self::Manifest(msg)
            | Self::NotFound(msg)
            | Self::Project(msg) => write!(f, "{msg}"),
            Self::Database { source, .. } => write!(f, "database error: {source}"),
            Self::DatabaseBusy { source, .. } => write!(f, "database busy: {source}"),
            Self::PermissionDenied { source, .. } => write!(f, "permission denied: {source}"),
            Self::Conflict { message, .. } => write!(f, "{message}"),
            Self::Symlink(source) => write!(f, "{source}"),
            Self::Io { source, .. } => write!(f, "{source}"),
            Self::Parse { source, .. } => write!(f, "{source}"),
        }?;
        if let Some(path) = self.path() {
            write!(f, "; path={}", path.display())?;
        }
        Ok(())
    }
}
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Database { source, .. } | Self::DatabaseBusy { source, .. } => Some(source),
            Self::Conflict { source, .. } => source.as_ref().map(|s| s as &dyn error::Error),
            Self::PermissionDenied { source, .. } | Self::Parse { source, .. } => {
                Some(source.as_ref())
            }
            Self::Io { source, .. } => Some(source),
            Self::Symlink(source) => Some(source),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        if source.kind() == io::ErrorKind::PermissionDenied {
            Self::PermissionDenied {
                path: None,
                source: Box::new(source),
            }
        } else {
            Self::Io { path: None, source }
        }
    }
}
impl From<SymlinkError> for Error {
    fn from(source: SymlinkError) -> Self {
        Self::Symlink(source)
    }
}
impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self::database(message)
    }
}
impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Self::parse(source)
    }
}
