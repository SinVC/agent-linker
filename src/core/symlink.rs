//! Symlink provider and link status entry point.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::BTreeMap;

use std::{
    error, fmt, fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

#[cfg(windows)]
use win_symlinks_client::{
    create_symlink_via_broker, CreateSymlinkOptions as BrokerCreateSymlinkOptions, ErrorCode,
    TargetKind, WinSymlinksError,
};

pub type SymlinkResult<T> = std::result::Result<T, SymlinkError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    File,
    #[serde(alias = "dir")]
    Directory,
}

impl fmt::Display for LinkKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkKind::File => write!(formatter, "file"),
            LinkKind::Directory => write!(formatter, "directory"),
        }
    }
}

impl FromStr for LinkKind {
    type Err = ParseLinkKindError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "file" => Ok(LinkKind::File),
            "directory" | "dir" => Ok(LinkKind::Directory),
            other => Err(ParseLinkKindError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLinkKindError(String);

impl fmt::Display for ParseLinkKindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown link kind `{}`", self.0)
    }
}

impl error::Error for ParseLinkKindError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkBackend {
    Std,
    WindowsBroker,
    WindowsStdFallback,
    Mock,
}

impl fmt::Display for SymlinkBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymlinkBackend::Std => write!(formatter, "std"),
            SymlinkBackend::WindowsBroker => write!(formatter, "windows-broker"),
            SymlinkBackend::WindowsStdFallback => write!(formatter, "windows-std-fallback"),
            SymlinkBackend::Mock => write!(formatter, "mock"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkStatus {
    Missing,
    CorrectSymlink { target: PathBuf },
    WrongSymlinkTarget { actual: PathBuf, expected: PathBuf },
    BrokenSymlink { target: PathBuf },
    ExistingRealFile,
    ExistingRealDirectory,
    UnsupportedFileType { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkErrorKind {
    SourceNotFound,
    LinkParentNotFound,
    LinkAlreadyExists,
    WrongSymlinkTarget,
    ExistingRealFile,
    ExistingRealDirectory,
    PermissionDenied,
    BrokerUnavailable,
    BrokerProtocolError,
    UnsupportedPlatform,
    UnsupportedLinkKind,
    Io,
}

impl fmt::Display for SymlinkErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymlinkErrorKind::SourceNotFound => write!(formatter, "source_not_found"),
            SymlinkErrorKind::LinkParentNotFound => write!(formatter, "link_parent_not_found"),
            SymlinkErrorKind::LinkAlreadyExists => write!(formatter, "link_already_exists"),
            SymlinkErrorKind::WrongSymlinkTarget => write!(formatter, "wrong_symlink_target"),
            SymlinkErrorKind::ExistingRealFile => write!(formatter, "existing_real_file"),
            SymlinkErrorKind::ExistingRealDirectory => write!(formatter, "existing_real_directory"),
            SymlinkErrorKind::PermissionDenied => write!(formatter, "permission_denied"),
            SymlinkErrorKind::BrokerUnavailable => write!(formatter, "broker_unavailable"),
            SymlinkErrorKind::BrokerProtocolError => write!(formatter, "broker_protocol_error"),
            SymlinkErrorKind::UnsupportedPlatform => write!(formatter, "unsupported_platform"),
            SymlinkErrorKind::UnsupportedLinkKind => write!(formatter, "unsupported_link_kind"),
            SymlinkErrorKind::Io => write!(formatter, "io"),
        }
    }
}

#[derive(Debug)]
pub struct SymlinkError {
    kind: SymlinkErrorKind,
    backend: SymlinkBackend,
    source: Option<PathBuf>,
    link: Option<PathBuf>,
    detail: Option<String>,
    system_code: Option<i32>,
    broker_code: Option<String>,
    cause: Option<Box<dyn error::Error + Send + Sync>>,
}

impl SymlinkError {
    pub fn new(kind: SymlinkErrorKind, backend: SymlinkBackend) -> Self {
        Self {
            kind,
            backend,
            source: None,
            link: None,
            detail: None,
            system_code: None,
            broker_code: None,
            cause: None,
        }
    }

    pub fn with_cause(mut self, source: impl error::Error + Send + Sync + 'static) -> Self {
        self.cause = Some(Box::new(source));
        self
    }

    pub fn kind(&self) -> SymlinkErrorKind {
        self.kind
    }

    pub fn backend(&self) -> SymlinkBackend {
        self.backend
    }

    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    pub fn link(&self) -> Option<&Path> {
        self.link.as_deref()
    }

    pub fn system_code(&self) -> Option<i32> {
        self.system_code
    }

    pub fn broker_code(&self) -> Option<&str> {
        self.broker_code.as_deref()
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub fn with_source(mut self, source: impl Into<PathBuf>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_link(mut self, link: impl Into<PathBuf>) -> Self {
        self.link = Some(link.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_system_code(mut self, system_code: Option<i32>) -> Self {
        self.system_code = system_code;
        self
    }

    pub fn with_broker_code(mut self, broker_code: impl Into<String>) -> Self {
        self.broker_code = Some(broker_code.into());
        self
    }
}

impl fmt::Display for SymlinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} via {}", self.kind, self.backend)?;

        if let Some(source) = &self.source {
            write!(formatter, "; source={}", source.display())?;
        }

        if let Some(link) = &self.link {
            write!(formatter, "; link={}", link.display())?;
        }

        if let Some(detail) = &self.detail {
            write!(formatter, "; {detail}")?;
        }

        if let Some(system_code) = self.system_code {
            write!(formatter, "; system_code={system_code}")?;
        }

        if let Some(broker_code) = &self.broker_code {
            write!(formatter, "; broker_code={broker_code}")?;
        }

        Ok(())
    }
}

impl error::Error for SymlinkError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.cause
            .as_ref()
            .map(|source| source.as_ref() as &dyn error::Error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreateSymlinkOptions {
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateSymlinkOutcome {
    Created,
    AlreadyCorrect,
    ReplacedWrongSymlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveSymlinkOutcome {
    Removed,
    Missing,
}

pub trait SymlinkProvider {
    fn validate_source(&self, source: &Path, kind: LinkKind) -> SymlinkResult<()> {
        validate_source(source, kind, self.backend())
    }

    fn backend(&self) -> SymlinkBackend;

    fn create_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()>;

    /// Replace an existing symlink without removing it before its replacement is ready.
    /// Reject real paths and validate the new source before changing the old link.
    fn replace_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()>;

    fn remove_symlink(&mut self, link: &Path) -> SymlinkResult<RemoveSymlinkOutcome>;

    fn read_link(&self, link: &Path) -> SymlinkResult<PathBuf>;

    fn link_status(&self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<LinkStatus>;
}

pub fn ensure_symlink(
    provider: &mut dyn SymlinkProvider,
    source: &Path,
    link: &Path,
    kind: LinkKind,
    options: CreateSymlinkOptions,
) -> SymlinkResult<CreateSymlinkOutcome> {
    let status = provider.link_status(source, link, kind)?;
    provider.validate_source(source, kind)?;
    let outcome = evaluate_create(&status, true, source, link, provider.backend(), options)?;
    match outcome {
        CreateSymlinkOutcome::Created => provider.create_symlink(source, link, kind)?,
        CreateSymlinkOutcome::ReplacedWrongSymlink => {
            provider.replace_symlink(source, link, kind)?
        }
        CreateSymlinkOutcome::AlreadyCorrect => {}
    }
    Ok(outcome)
}

pub fn evaluate_create(
    status: &LinkStatus,
    source_valid: bool,
    source: &Path,
    link: &Path,
    backend: SymlinkBackend,
    options: CreateSymlinkOptions,
) -> SymlinkResult<CreateSymlinkOutcome> {
    if !source_valid {
        return Err(SymlinkError::new(SymlinkErrorKind::SourceNotFound, backend)
            .with_source(source)
            .with_link(link));
    }
    match status {
        LinkStatus::Missing => Ok(CreateSymlinkOutcome::Created),
        LinkStatus::CorrectSymlink { .. } => Ok(CreateSymlinkOutcome::AlreadyCorrect),
        LinkStatus::WrongSymlinkTarget { .. } | LinkStatus::BrokenSymlink { .. }
            if options.force =>
        {
            Ok(CreateSymlinkOutcome::ReplacedWrongSymlink)
        }
        LinkStatus::WrongSymlinkTarget { actual, expected } => Err(SymlinkError::new(
            SymlinkErrorKind::WrongSymlinkTarget,
            backend,
        )
        .with_source(expected)
        .with_link(link)
        .with_detail(format!("existing link points to {}", actual.display()))),
        LinkStatus::BrokenSymlink { target } => {
            Err(SymlinkError::new(SymlinkErrorKind::SourceNotFound, backend)
                .with_source(source)
                .with_link(link)
                .with_detail(format!(
                    "existing symlink points to missing source {}",
                    target.display()
                )))
        }
        _ => evaluate_remove(status, link, backend)
            .map(|_| CreateSymlinkOutcome::Created)
            .map_err(|error| error.with_source(source)),
    }
}

pub fn evaluate_remove(
    status: &LinkStatus,
    link: &Path,
    backend: SymlinkBackend,
) -> SymlinkResult<RemoveSymlinkOutcome> {
    let kind = match status {
        LinkStatus::Missing => return Ok(RemoveSymlinkOutcome::Missing),
        LinkStatus::CorrectSymlink { .. }
        | LinkStatus::WrongSymlinkTarget { .. }
        | LinkStatus::BrokenSymlink { .. } => return Ok(RemoveSymlinkOutcome::Removed),
        LinkStatus::ExistingRealFile => SymlinkErrorKind::ExistingRealFile,
        LinkStatus::ExistingRealDirectory => SymlinkErrorKind::ExistingRealDirectory,
        LinkStatus::UnsupportedFileType { .. } => SymlinkErrorKind::UnsupportedLinkKind,
    };
    Err(SymlinkError::new(kind, backend).with_link(link))
}

pub fn default_provider() -> Box<dyn SymlinkProvider> {
    #[cfg(windows)]
    {
        Box::new(WindowsBrokerSymlinkProvider::new())
    }

    #[cfg(not(windows))]
    {
        Box::new(StdSymlinkProvider::new())
    }
}

#[derive(Debug, Clone)]
pub struct StdSymlinkProvider {
    backend: SymlinkBackend,
}

impl StdSymlinkProvider {
    #[cfg(not(windows))]
    pub const fn new() -> Self {
        Self {
            backend: SymlinkBackend::Std,
        }
    }

    #[cfg(windows)]
    pub const fn explicit_windows_fallback() -> Self {
        Self {
            backend: SymlinkBackend::WindowsStdFallback,
        }
    }
}

impl SymlinkProvider for StdSymlinkProvider {
    fn backend(&self) -> SymlinkBackend {
        self.backend
    }

    fn create_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        validate_source(source, kind, self.backend)?;
        validate_link_parent(link, self.backend)?;
        reject_existing_link_path(link, self.backend)?;
        create_std_symlink(source, link, kind, self.backend)
    }

    fn replace_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        validate_source(source, kind, self.backend)?;
        validate_link_parent(link, self.backend)?;
        require_fs_symlink(link, self.backend)?;
        replace_std_symlink(source, link, kind, self.backend)
    }

    fn remove_symlink(&mut self, link: &Path) -> SymlinkResult<RemoveSymlinkOutcome> {
        remove_fs_symlink(link, self.backend)
    }

    fn read_link(&self, link: &Path) -> SymlinkResult<PathBuf> {
        read_fs_link(link, self.backend)
    }

    fn link_status(&self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<LinkStatus> {
        fs_link_status(source, link, kind, self.backend)
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, Default)]
pub struct WindowsBrokerSymlinkProvider;

#[cfg(windows)]
impl WindowsBrokerSymlinkProvider {
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(windows)]
impl SymlinkProvider for WindowsBrokerSymlinkProvider {
    fn backend(&self) -> SymlinkBackend {
        SymlinkBackend::WindowsBroker
    }

    fn create_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        self.write_symlink(source, link, kind, false)
    }

    fn replace_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        self.write_symlink(source, link, kind, true)
    }

    fn remove_symlink(&mut self, link: &Path) -> SymlinkResult<RemoveSymlinkOutcome> {
        remove_fs_symlink(link, SymlinkBackend::WindowsBroker)
    }

    fn read_link(&self, link: &Path) -> SymlinkResult<PathBuf> {
        read_fs_link(link, SymlinkBackend::WindowsBroker)
    }

    fn link_status(&self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<LinkStatus> {
        fs_link_status(source, link, kind, SymlinkBackend::WindowsBroker)
    }
}

#[cfg(windows)]
impl WindowsBrokerSymlinkProvider {
    fn write_symlink(
        &mut self,
        source: &Path,
        link: &Path,
        kind: LinkKind,
        replace: bool,
    ) -> SymlinkResult<()> {
        validate_source(source, kind, SymlinkBackend::WindowsBroker)?;
        validate_link_parent(link, SymlinkBackend::WindowsBroker)?;
        if replace {
            require_fs_symlink(link, SymlinkBackend::WindowsBroker)?;
        } else {
            reject_existing_link_path(link, SymlinkBackend::WindowsBroker)?;
        }

        let options = BrokerCreateSymlinkOptions::new(source, link)
            .target_kind(broker_target_kind(kind))
            .replace_existing_symlink(replace);

        create_symlink_via_broker(options).map_err(|error| {
            broker_error_to_symlink_error(error)
                .with_source(source)
                .with_link(link)
        })
    }
}

#[cfg(windows)]
fn broker_target_kind(kind: LinkKind) -> TargetKind {
    match kind {
        LinkKind::File => TargetKind::File,
        LinkKind::Directory => TargetKind::Dir,
    }
}

#[cfg(windows)]
fn broker_error_to_symlink_error(error: WinSymlinksError) -> SymlinkError {
    let kind = match error.code() {
        ErrorCode::ServiceNotInstalled | ErrorCode::ServiceUnavailable => {
            SymlinkErrorKind::BrokerUnavailable
        }
        ErrorCode::UnsupportedMode
        | ErrorCode::ServiceIdentityMismatch
        | ErrorCode::RemoteClientRejected => SymlinkErrorKind::BrokerProtocolError,
        ErrorCode::PrivilegeRequired | ErrorCode::CallerParentWriteDenied => {
            SymlinkErrorKind::PermissionDenied
        }
        ErrorCode::LinkAlreadyExists => SymlinkErrorKind::LinkAlreadyExists,
        ErrorCode::LinkPathIsNotSymlink
        | ErrorCode::UnsafeReparsePoint
        | ErrorCode::ReplacementPartiallyCompleted => SymlinkErrorKind::WrongSymlinkTarget,
        ErrorCode::TargetKindRequired | ErrorCode::TargetKindConflict => {
            SymlinkErrorKind::UnsupportedLinkKind
        }
        ErrorCode::SourceBlacklisted
        | ErrorCode::CreateSymlinkFailed
        | ErrorCode::PathNormalizationFailed => SymlinkErrorKind::Io,
    };

    SymlinkError::new(kind, SymlinkBackend::WindowsBroker)
        .with_detail(error.message().to_string())
        .with_broker_code(error.code().to_string())
        .with_cause(error)
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockEntry {
    File,
    Directory,
    Symlink { target: PathBuf, kind: LinkKind },
    Unsupported,
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct MockSymlinkProvider {
    entries: BTreeMap<PathBuf, MockEntry>,
    fail_next_replacement: bool,
}

#[cfg(test)]
impl MockSymlinkProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: impl Into<PathBuf>) {
        self.entries.insert(path.into(), MockEntry::File);
    }

    pub fn add_dir(&mut self, path: impl Into<PathBuf>) {
        self.entries.insert(path.into(), MockEntry::Directory);
    }

    pub fn add_symlink(
        &mut self,
        link: impl Into<PathBuf>,
        target: impl Into<PathBuf>,
        kind: LinkKind,
    ) {
        self.entries.insert(
            link.into(),
            MockEntry::Symlink {
                target: target.into(),
                kind,
            },
        );
    }

    pub fn add_unsupported(&mut self, path: impl Into<PathBuf>) {
        self.entries.insert(path.into(), MockEntry::Unsupported);
    }

    pub fn entry(&self, path: &Path) -> Option<&MockEntry> {
        self.entries.get(path)
    }

    pub fn fail_next_replacement(&mut self) {
        self.fail_next_replacement = true;
    }

    fn validate_source(&self, source: &Path, kind: LinkKind) -> SymlinkResult<()> {
        if !self.entries.contains_key(source) {
            return Err(
                SymlinkError::new(SymlinkErrorKind::SourceNotFound, self.backend())
                    .with_source(source),
            );
        }
        if !self.source_matches_kind(source, kind) {
            return Err(
                SymlinkError::new(SymlinkErrorKind::UnsupportedLinkKind, self.backend())
                    .with_source(source)
                    .with_detail(format!("source is not a {kind}")),
            );
        }
        Ok(())
    }

    fn source_matches_kind(&self, source: &Path, kind: LinkKind) -> bool {
        matches!(
            (self.entries.get(source), kind),
            (Some(MockEntry::File), LinkKind::File)
                | (Some(MockEntry::Directory), LinkKind::Directory)
        )
    }

    fn parent_exists(&self, link: &Path) -> bool {
        let Some(parent) = link
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        else {
            return true;
        };

        matches!(self.entries.get(parent), Some(MockEntry::Directory))
    }
}

#[cfg(test)]
impl SymlinkProvider for MockSymlinkProvider {
    fn validate_source(&self, source: &Path, kind: LinkKind) -> SymlinkResult<()> {
        MockSymlinkProvider::validate_source(self, source, kind)
    }
    fn backend(&self) -> SymlinkBackend {
        SymlinkBackend::Mock
    }

    fn create_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        self.validate_source(source, kind)
            .map_err(|error| error.with_link(link))?;

        if !self.parent_exists(link) {
            return Err(SymlinkError::new(
                SymlinkErrorKind::LinkParentNotFound,
                SymlinkBackend::Mock,
            )
            .with_source(source)
            .with_link(link));
        }

        if self.entries.contains_key(link) {
            return Err(SymlinkError::new(
                SymlinkErrorKind::LinkAlreadyExists,
                SymlinkBackend::Mock,
            )
            .with_source(source)
            .with_link(link));
        }

        self.add_symlink(link.to_path_buf(), source.to_path_buf(), kind);
        Ok(())
    }

    fn replace_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        self.validate_source(source, kind)
            .map_err(|error| error.with_link(link))?;
        // read_link rejects real files/directories and absent paths without changing them.
        self.read_link(link)?;
        if std::mem::take(&mut self.fail_next_replacement) {
            return Err(SymlinkError::new(SymlinkErrorKind::Io, self.backend())
                .with_source(source)
                .with_link(link)
                .with_detail("injected replacement failure"));
        }
        self.add_symlink(link.to_path_buf(), source.to_path_buf(), kind);
        Ok(())
    }

    fn remove_symlink(&mut self, link: &Path) -> SymlinkResult<RemoveSymlinkOutcome> {
        let state = self.link_status(Path::new(""), link, LinkKind::File)?;
        let outcome = evaluate_remove(&state, link, self.backend())?;
        if outcome == RemoveSymlinkOutcome::Removed {
            self.entries.remove(link);
        }
        Ok(outcome)
    }

    fn read_link(&self, link: &Path) -> SymlinkResult<PathBuf> {
        match self.entries.get(link) {
            Some(MockEntry::Symlink { target, .. }) => Ok(target.clone()),
            Some(MockEntry::File) => Err(SymlinkError::new(
                SymlinkErrorKind::ExistingRealFile,
                SymlinkBackend::Mock,
            )
            .with_link(link)),
            Some(MockEntry::Directory) => Err(SymlinkError::new(
                SymlinkErrorKind::ExistingRealDirectory,
                SymlinkBackend::Mock,
            )
            .with_link(link)),
            Some(MockEntry::Unsupported) => Err(SymlinkError::new(
                SymlinkErrorKind::UnsupportedLinkKind,
                SymlinkBackend::Mock,
            )
            .with_link(link)),
            None => Err(
                SymlinkError::new(SymlinkErrorKind::Io, SymlinkBackend::Mock)
                    .with_link(link)
                    .with_detail("link does not exist"),
            ),
        }
    }

    fn link_status(&self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<LinkStatus> {
        match self.entries.get(link) {
            None => Ok(LinkStatus::Missing),
            Some(MockEntry::File) => Ok(LinkStatus::ExistingRealFile),
            Some(MockEntry::Directory) => Ok(LinkStatus::ExistingRealDirectory),
            Some(MockEntry::Unsupported) => Ok(LinkStatus::UnsupportedFileType {
                path: link.to_path_buf(),
            }),
            Some(MockEntry::Symlink { target, .. }) => {
                if !mock_targets_match(link, target, source) {
                    return Ok(LinkStatus::WrongSymlinkTarget {
                        actual: target.clone(),
                        expected: source.to_path_buf(),
                    });
                }

                match self.entries.get(source) {
                    None => Ok(LinkStatus::BrokenSymlink {
                        target: target.clone(),
                    }),
                    Some(MockEntry::File) if kind == LinkKind::File => {
                        Ok(LinkStatus::CorrectSymlink {
                            target: target.clone(),
                        })
                    }
                    Some(MockEntry::Directory) if kind == LinkKind::Directory => {
                        Ok(LinkStatus::CorrectSymlink {
                            target: target.clone(),
                        })
                    }
                    Some(_) => Ok(LinkStatus::UnsupportedFileType {
                        path: source.to_path_buf(),
                    }),
                }
            }
        }
    }
}

fn validate_source(source: &Path, kind: LinkKind, backend: SymlinkBackend) -> SymlinkResult<()> {
    let metadata = match fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(SymlinkError::new(SymlinkErrorKind::SourceNotFound, backend)
                .with_source(source)
                .with_system_code(error.raw_os_error())
                .with_cause(error));
        }
        Err(error) => return Err(io_to_symlink_error(error, backend).with_source(source)),
    };

    if metadata_matches_kind(&metadata, kind) {
        Ok(())
    } else {
        Err(
            SymlinkError::new(SymlinkErrorKind::UnsupportedLinkKind, backend)
                .with_source(source)
                .with_detail(format!("source is not a {kind}")),
        )
    }
}

fn validate_link_parent(link: &Path, backend: SymlinkBackend) -> SymlinkResult<()> {
    let Some(parent) = link
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };

    match fs::metadata(parent) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(
            SymlinkError::new(SymlinkErrorKind::LinkParentNotFound, backend)
                .with_link(link)
                .with_detail(format!("parent is not a directory: {}", parent.display())),
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(SymlinkError::new(
            SymlinkErrorKind::LinkParentNotFound,
            backend,
        )
        .with_link(link)
        .with_system_code(error.raw_os_error())
        .with_cause(error)),
        Err(error) => Err(io_to_symlink_error(error, backend).with_link(link)),
    }
}

fn reject_existing_link_path(link: &Path, backend: SymlinkBackend) -> SymlinkResult<()> {
    match fs::symlink_metadata(link) {
        Ok(_) => {
            Err(SymlinkError::new(SymlinkErrorKind::LinkAlreadyExists, backend).with_link(link))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_to_symlink_error(error, backend).with_link(link)),
    }
}

fn require_fs_symlink(link: &Path, backend: SymlinkBackend) -> SymlinkResult<()> {
    let metadata = fs::symlink_metadata(link)
        .map_err(|error| io_to_symlink_error(error, backend).with_link(link))?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let kind = if metadata.is_file() {
        SymlinkErrorKind::ExistingRealFile
    } else if metadata.is_dir() {
        SymlinkErrorKind::ExistingRealDirectory
    } else {
        SymlinkErrorKind::UnsupportedLinkKind
    };
    Err(SymlinkError::new(kind, backend).with_link(link))
}

fn replace_std_symlink(
    source: &Path,
    link: &Path,
    kind: LinkKind,
    backend: SymlinkBackend,
) -> SymlinkResult<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    let parent = link.parent().unwrap_or_else(|| Path::new("."));
    loop {
        let temporary = parent.join(format!(
            ".aglink-symlink-{}-{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        // Creating the temporary symlink itself reserves its name without clobbering.
        match create_std_symlink(source, &temporary, kind, backend) {
            Err(_) if fs::symlink_metadata(&temporary).is_ok() => continue,
            Err(error) => return Err(error.with_link(link)),
            Ok(()) => {}
        }
        let result = require_fs_symlink(link, backend).and_then(|()| {
            fs::rename(&temporary, link).map_err(|error| {
                io_to_symlink_error(error, backend)
                    .with_source(source)
                    .with_link(link)
            })
        });
        if result.is_err() {
            // Only the staged symlink belongs to us; never remove the destination here.
            let _ = remove_fs_symlink(&temporary, backend);
        }
        return result;
    }
}

fn metadata_matches_kind(metadata: &fs::Metadata, kind: LinkKind) -> bool {
    match kind {
        LinkKind::File => metadata.is_file(),
        LinkKind::Directory => metadata.is_dir(),
    }
}

fn create_std_symlink(
    source: &Path,
    link: &Path,
    kind: LinkKind,
    backend: SymlinkBackend,
) -> SymlinkResult<()> {
    #[cfg(unix)]
    {
        let _ = kind;
        std::os::unix::fs::symlink(source, link).map_err(|error| {
            io_to_symlink_error(error, backend)
                .with_source(source)
                .with_link(link)
        })
    }

    #[cfg(windows)]
    {
        let result = match kind {
            LinkKind::File => std::os::windows::fs::symlink_file(source, link),
            LinkKind::Directory => std::os::windows::fs::symlink_dir(source, link),
        };

        result.map_err(|error| {
            io_to_symlink_error(error, backend)
                .with_source(source)
                .with_link(link)
        })
    }

    #[cfg(all(not(unix), not(windows)))]
    {
        let _ = (source, link, kind);
        Err(SymlinkError::new(
            SymlinkErrorKind::UnsupportedPlatform,
            backend,
        ))
    }
}

fn fs_link_status(
    source: &Path,
    link: &Path,
    kind: LinkKind,
    backend: SymlinkBackend,
) -> SymlinkResult<LinkStatus> {
    let link_metadata = match fs::symlink_metadata(link) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(LinkStatus::Missing),
        Err(error) => return Err(io_to_symlink_error(error, backend).with_link(link)),
    };

    let file_type = link_metadata.file_type();

    if file_type.is_symlink() {
        let target = read_fs_link(link, backend)?;

        if !fs_targets_match(link, &target, source) {
            return Ok(LinkStatus::WrongSymlinkTarget {
                actual: target,
                expected: source.to_path_buf(),
            });
        }

        return match fs::metadata(source) {
            Ok(source_metadata) if metadata_matches_kind(&source_metadata, kind) => {
                Ok(LinkStatus::CorrectSymlink { target })
            }
            Ok(_) => Ok(LinkStatus::UnsupportedFileType {
                path: source.to_path_buf(),
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(LinkStatus::BrokenSymlink { target })
            }
            Err(error) => Err(io_to_symlink_error(error, backend).with_source(source)),
        };
    }

    if link_metadata.is_file() {
        Ok(LinkStatus::ExistingRealFile)
    } else if link_metadata.is_dir() {
        Ok(LinkStatus::ExistingRealDirectory)
    } else {
        Ok(LinkStatus::UnsupportedFileType {
            path: link.to_path_buf(),
        })
    }
}

fn read_fs_link(link: &Path, backend: SymlinkBackend) -> SymlinkResult<PathBuf> {
    fs::read_link(link).map_err(|error| io_to_symlink_error(error, backend).with_link(link))
}

fn remove_fs_symlink(link: &Path, backend: SymlinkBackend) -> SymlinkResult<RemoveSymlinkOutcome> {
    let link_metadata = match fs::symlink_metadata(link) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(RemoveSymlinkOutcome::Missing);
        }
        Err(error) => return Err(io_to_symlink_error(error, backend).with_link(link)),
    };

    if !link_metadata.file_type().is_symlink() {
        let status = if link_metadata.is_file() {
            LinkStatus::ExistingRealFile
        } else if link_metadata.is_dir() {
            LinkStatus::ExistingRealDirectory
        } else {
            LinkStatus::UnsupportedFileType {
                path: link.to_path_buf(),
            }
        };
        return evaluate_remove(&status, link, backend);
    }

    remove_symlink_path(link, &link_metadata, backend)?;
    Ok(RemoveSymlinkOutcome::Removed)
}

#[cfg(unix)]
fn remove_symlink_path(
    link: &Path,
    _metadata: &fs::Metadata,
    backend: SymlinkBackend,
) -> SymlinkResult<()> {
    fs::remove_file(link).map_err(|error| io_to_symlink_error(error, backend).with_link(link))
}

#[cfg(windows)]
fn remove_symlink_path(
    link: &Path,
    metadata: &fs::Metadata,
    backend: SymlinkBackend,
) -> SymlinkResult<()> {
    use std::os::windows::fs::FileTypeExt;

    let result = if metadata.file_type().is_symlink_dir() {
        fs::remove_dir(link)
    } else {
        fs::remove_file(link)
    };

    result.map_err(|error| io_to_symlink_error(error, backend).with_link(link))
}

#[cfg(all(not(unix), not(windows)))]
fn remove_symlink_path(
    link: &Path,
    _metadata: &fs::Metadata,
    backend: SymlinkBackend,
) -> SymlinkResult<()> {
    Err(SymlinkError::new(SymlinkErrorKind::UnsupportedPlatform, backend).with_link(link))
}

fn fs_targets_match(link: &Path, actual: &Path, expected: &Path) -> bool {
    if actual == expected || resolve_relative_target(link, actual) == expected {
        return true;
    }

    let actual = resolve_relative_target(link, actual);
    match (actual.canonicalize(), expected.canonicalize()) {
        (Ok(actual), Ok(expected)) => {
            crate::core::paths::normalize_windows_path(&actual)
                == crate::core::paths::normalize_windows_path(&expected)
        }
        (Err(actual_err), Err(expected_err))
            if actual_err.kind() == io::ErrorKind::NotFound
                && expected_err.kind() == io::ErrorKind::NotFound =>
        {
            ancestor_targets_match(&actual, expected)
        }
        _ => false,
    }
}

fn ancestor_targets_match(actual: &Path, expected: &Path) -> bool {
    let Some((actual_ancestor, actual_tail)) = split_longest_existing_ancestor(actual) else {
        return false;
    };
    let Some((expected_ancestor, expected_tail)) = split_longest_existing_ancestor(expected) else {
        return false;
    };

    if actual_tail != expected_tail {
        return false;
    }

    match (actual_ancestor.canonicalize(), expected_ancestor.canonicalize()) {
        (Ok(a), Ok(e)) => {
            crate::core::paths::normalize_windows_path(&a)
                == crate::core::paths::normalize_windows_path(&e)
        }
        _ => false,
    }
}

fn split_longest_existing_ancestor(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut current = path.to_path_buf();
    let mut tail_components = Vec::new();

    loop {
        match current.canonicalize() {
            Ok(_) => {
                let mut tail = PathBuf::new();
                for comp in tail_components.into_iter().rev() {
                    tail.push(comp);
                }
                return Some((current, tail));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let file_name = current.file_name()?.to_os_string();
                tail_components.push(file_name);
                if !current.pop() {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
fn mock_targets_match(link: &Path, actual: &Path, expected: &Path) -> bool {
    actual == expected || resolve_relative_target(link, actual) == expected
}

fn resolve_relative_target(link: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        return target.to_path_buf();
    }

    link.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| target.to_path_buf(), |parent| parent.join(target))
}

fn io_to_symlink_error(error: io::Error, backend: SymlinkBackend) -> SymlinkError {
    let kind = match error.kind() {
        io::ErrorKind::PermissionDenied => SymlinkErrorKind::PermissionDenied,
        _ => SymlinkErrorKind::Io,
    };

    SymlinkError::new(kind, backend)
        .with_detail(error.to_string())
        .with_system_code(error.raw_os_error())
        .with_cause(error)
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::{broker_error_to_symlink_error, ErrorCode, WinSymlinksError};
    use super::{
        default_provider, ensure_symlink, fs_targets_match, CreateSymlinkOptions,
        CreateSymlinkOutcome, LinkKind, LinkStatus, MockEntry, MockSymlinkProvider,
        RemoveSymlinkOutcome, StdSymlinkProvider, SymlinkBackend, SymlinkErrorKind,
        SymlinkProvider,
    };
    use std::{fs, path::PathBuf};
    #[cfg(unix)]
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn path(value: &str) -> PathBuf {
        PathBuf::from(value)
    }

    fn base_mock() -> MockSymlinkProvider {
        let mut provider = MockSymlinkProvider::new();
        provider.add_dir(path("project"));
        provider
    }

    #[test]
    fn mock_link_status_covers_all_statuses() {
        let mut provider = base_mock();
        provider.add_file(path("project/source.md"));
        provider.add_file(path("project/other.md"));
        provider.add_symlink(
            path("project/correct.md"),
            path("project/source.md"),
            LinkKind::File,
        );
        provider.add_symlink(
            path("project/wrong.md"),
            path("project/other.md"),
            LinkKind::File,
        );
        provider.add_symlink(
            path("project/broken.md"),
            path("project/missing-source.md"),
            LinkKind::File,
        );
        provider.add_file(path("project/real-file.md"));
        provider.add_dir(path("project/real-dir"));
        provider.add_unsupported(path("project/socket"));

        assert_eq!(
            provider
                .link_status(
                    &path("project/source.md"),
                    &path("project/missing-link.md"),
                    LinkKind::File,
                )
                .unwrap(),
            LinkStatus::Missing
        );
        assert_eq!(
            provider
                .link_status(
                    &path("project/source.md"),
                    &path("project/correct.md"),
                    LinkKind::File,
                )
                .unwrap(),
            LinkStatus::CorrectSymlink {
                target: path("project/source.md")
            }
        );
        assert_eq!(
            provider
                .link_status(
                    &path("project/source.md"),
                    &path("project/wrong.md"),
                    LinkKind::File,
                )
                .unwrap(),
            LinkStatus::WrongSymlinkTarget {
                actual: path("project/other.md"),
                expected: path("project/source.md")
            }
        );
        assert_eq!(
            provider
                .link_status(
                    &path("project/missing-source.md"),
                    &path("project/broken.md"),
                    LinkKind::File,
                )
                .unwrap(),
            LinkStatus::BrokenSymlink {
                target: path("project/missing-source.md")
            }
        );
        assert_eq!(
            provider
                .link_status(
                    &path("project/source.md"),
                    &path("project/real-file.md"),
                    LinkKind::File,
                )
                .unwrap(),
            LinkStatus::ExistingRealFile
        );
        assert_eq!(
            provider
                .link_status(
                    &path("project/source.md"),
                    &path("project/real-dir"),
                    LinkKind::File,
                )
                .unwrap(),
            LinkStatus::ExistingRealDirectory
        );
        assert_eq!(
            provider
                .link_status(
                    &path("project/source.md"),
                    &path("project/socket"),
                    LinkKind::File,
                )
                .unwrap(),
            LinkStatus::UnsupportedFileType {
                path: path("project/socket")
            }
        );
    }

    #[test]
    fn mock_create_read_and_remove_are_safe() {
        let mut provider = base_mock();
        provider.add_file(path("project/source.md"));

        provider
            .create_symlink(
                &path("project/source.md"),
                &path("project/link.md"),
                LinkKind::File,
            )
            .unwrap();

        assert_eq!(
            provider.read_link(&path("project/link.md")).unwrap(),
            path("project/source.md")
        );

        let duplicate = provider
            .create_symlink(
                &path("project/source.md"),
                &path("project/link.md"),
                LinkKind::File,
            )
            .unwrap_err();
        assert_eq!(duplicate.kind(), SymlinkErrorKind::LinkAlreadyExists);

        assert_eq!(
            provider.remove_symlink(&path("project/link.md")).unwrap(),
            RemoveSymlinkOutcome::Removed
        );
        assert_eq!(
            provider.remove_symlink(&path("project/link.md")).unwrap(),
            RemoveSymlinkOutcome::Missing
        );
    }

    #[test]
    fn mock_create_rejects_missing_source_and_parent() {
        let mut provider = base_mock();

        let missing_source = provider
            .create_symlink(
                &path("project/source.md"),
                &path("project/link.md"),
                LinkKind::File,
            )
            .unwrap_err();
        assert_eq!(missing_source.kind(), SymlinkErrorKind::SourceNotFound);

        provider.add_file(path("project/source.md"));
        let missing_parent = provider
            .create_symlink(
                &path("project/source.md"),
                &path("missing-parent/link.md"),
                LinkKind::File,
            )
            .unwrap_err();
        assert_eq!(missing_parent.kind(), SymlinkErrorKind::LinkParentNotFound);
    }

    #[test]
    fn ensure_symlink_creates_then_skips_correct_link() {
        let mut provider = base_mock();
        provider.add_file(path("project/source.md"));

        let created = ensure_symlink(
            &mut provider,
            &path("project/source.md"),
            &path("project/link.md"),
            LinkKind::File,
            CreateSymlinkOptions { force: false },
        )
        .unwrap();
        assert_eq!(created, CreateSymlinkOutcome::Created);

        let skipped = ensure_symlink(
            &mut provider,
            &path("project/source.md"),
            &path("project/link.md"),
            LinkKind::File,
            CreateSymlinkOptions { force: false },
        )
        .unwrap();
        assert_eq!(skipped, CreateSymlinkOutcome::AlreadyCorrect);
    }

    #[test]
    fn ensure_symlink_rejects_wrong_target_without_force() {
        let mut provider = base_mock();
        provider.add_file(path("project/source.md"));
        provider.add_file(path("project/other.md"));
        provider.add_symlink(
            path("project/link.md"),
            path("project/other.md"),
            LinkKind::File,
        );

        let error = ensure_symlink(
            &mut provider,
            &path("project/source.md"),
            &path("project/link.md"),
            LinkKind::File,
            CreateSymlinkOptions { force: false },
        )
        .unwrap_err();

        assert_eq!(error.kind(), SymlinkErrorKind::WrongSymlinkTarget);
        assert_eq!(
            provider.read_link(&path("project/link.md")).unwrap(),
            path("project/other.md")
        );
    }

    #[test]
    fn force_replaces_wrong_symlink_only() {
        let mut provider = base_mock();
        provider.add_file(path("project/source.md"));
        provider.add_file(path("project/other.md"));
        provider.add_symlink(
            path("project/link.md"),
            path("project/other.md"),
            LinkKind::File,
        );

        let outcome = ensure_symlink(
            &mut provider,
            &path("project/source.md"),
            &path("project/link.md"),
            LinkKind::File,
            CreateSymlinkOptions { force: true },
        )
        .unwrap();

        assert_eq!(outcome, CreateSymlinkOutcome::ReplacedWrongSymlink);
        assert_eq!(
            provider.read_link(&path("project/link.md")).unwrap(),
            path("project/source.md")
        );
    }

    #[test]
    fn force_replaces_live_and_dangling_wrong_targets_and_failure_keeps_old_link() {
        for old_exists in [true, false] {
            for fail in [true, false] {
                let mut provider = base_mock();
                let source = path("project/new.md");
                let old = path("project/old.md");
                let link = path("project/link.md");
                provider.add_file(&source);
                if old_exists {
                    provider.add_file(&old);
                }
                provider.add_symlink(&link, &old, LinkKind::File);
                assert_eq!(
                    ensure_symlink(
                        &mut provider,
                        &source,
                        &link,
                        LinkKind::File,
                        CreateSymlinkOptions { force: false }
                    )
                    .unwrap_err()
                    .kind(),
                    SymlinkErrorKind::WrongSymlinkTarget
                );
                assert_eq!(provider.read_link(&link).unwrap(), old);
                if fail {
                    provider.fail_next_replacement();
                }
                let result = ensure_symlink(
                    &mut provider,
                    &source,
                    &link,
                    LinkKind::File,
                    CreateSymlinkOptions { force: true },
                );
                if fail {
                    assert_eq!(result.unwrap_err().kind(), SymlinkErrorKind::Io);
                    assert_eq!(provider.read_link(&link).unwrap(), old);
                } else {
                    assert_eq!(result.unwrap(), CreateSymlinkOutcome::ReplacedWrongSymlink);
                    assert_eq!(provider.read_link(&link).unwrap(), source);
                }
            }
        }
    }

    #[test]
    fn replacement_validates_new_source_and_preserves_old_link() {
        let mut provider = base_mock();
        let source = path("project/new.md");
        let old = path("project/old.md");
        let link = path("project/link.md");
        provider.add_file(&old);
        provider.add_symlink(&link, &old, LinkKind::File);
        assert_eq!(
            ensure_symlink(
                &mut provider,
                &source,
                &link,
                LinkKind::File,
                CreateSymlinkOptions { force: true },
            )
            .unwrap_err()
            .kind(),
            SymlinkErrorKind::SourceNotFound
        );
        assert_eq!(provider.read_link(&link).unwrap(), old);
        provider.add_dir(&source);
        assert_eq!(
            provider
                .replace_symlink(&source, &link, LinkKind::File)
                .unwrap_err()
                .kind(),
            SymlinkErrorKind::UnsupportedLinkKind
        );
        assert_eq!(provider.read_link(&link).unwrap(), old);
        provider.add_file(&source);
        for directory in [false, true] {
            if directory {
                provider.add_dir(&link);
            } else {
                provider.add_file(&link);
            }
            let before = provider.entry(&link).cloned();
            assert_eq!(
                provider
                    .replace_symlink(&source, &link, LinkKind::File)
                    .unwrap_err()
                    .kind(),
                if directory {
                    SymlinkErrorKind::ExistingRealDirectory
                } else {
                    SymlinkErrorKind::ExistingRealFile
                }
            );
            assert_eq!(provider.entry(&link), before.as_ref());
        }
    }

    #[test]
    fn force_never_deletes_real_file_or_directory() {
        let mut file_provider = base_mock();
        file_provider.add_file(path("project/source.md"));
        file_provider.add_file(path("project/link.md"));

        let file_error = ensure_symlink(
            &mut file_provider,
            &path("project/source.md"),
            &path("project/link.md"),
            LinkKind::File,
            CreateSymlinkOptions { force: true },
        )
        .unwrap_err();
        assert_eq!(file_error.kind(), SymlinkErrorKind::ExistingRealFile);
        assert_eq!(
            file_provider.entry(&path("project/link.md")),
            Some(&MockEntry::File)
        );

        let mut dir_provider = base_mock();
        dir_provider.add_file(path("project/source.md"));
        dir_provider.add_dir(path("project/link.md"));

        let dir_error = ensure_symlink(
            &mut dir_provider,
            &path("project/source.md"),
            &path("project/link.md"),
            LinkKind::File,
            CreateSymlinkOptions { force: true },
        )
        .unwrap_err();
        assert_eq!(dir_error.kind(), SymlinkErrorKind::ExistingRealDirectory);
        assert_eq!(
            dir_provider.entry(&path("project/link.md")),
            Some(&MockEntry::Directory)
        );
    }

    #[test]
    fn broken_symlink_to_expected_source_reports_source_not_found() {
        let mut provider = base_mock();
        provider.add_symlink(
            path("project/link.md"),
            path("project/source.md"),
            LinkKind::File,
        );

        let error = ensure_symlink(
            &mut provider,
            &path("project/source.md"),
            &path("project/link.md"),
            LinkKind::File,
            CreateSymlinkOptions { force: true },
        )
        .unwrap_err();

        assert_eq!(error.kind(), SymlinkErrorKind::SourceNotFound);
        assert_eq!(
            provider.read_link(&path("project/link.md")).unwrap(),
            path("project/source.md")
        );
    }

    #[test]
    fn missing_wrong_target_is_wrong_symlink_not_source_not_found() {
        let mut provider = base_mock();
        provider.add_file(path("project/source.md"));
        provider.add_symlink(
            path("project/link.md"),
            path("project/missing-other.md"),
            LinkKind::File,
        );

        let status = provider
            .link_status(
                &path("project/source.md"),
                &path("project/link.md"),
                LinkKind::File,
            )
            .unwrap();

        assert_eq!(
            status,
            LinkStatus::WrongSymlinkTarget {
                actual: path("project/missing-other.md"),
                expected: path("project/source.md")
            }
        );
    }

    #[test]
    fn mock_remove_refuses_real_paths() {
        let mut provider = base_mock();
        provider.add_file(path("project/real-file.md"));
        provider.add_dir(path("project/real-dir"));

        let file_error = provider
            .remove_symlink(&path("project/real-file.md"))
            .unwrap_err();
        assert_eq!(file_error.kind(), SymlinkErrorKind::ExistingRealFile);

        let dir_error = provider
            .remove_symlink(&path("project/real-dir"))
            .unwrap_err();
        assert_eq!(dir_error.kind(), SymlinkErrorKind::ExistingRealDirectory);
    }

    #[cfg(windows)]
    #[test]
    fn default_provider_uses_windows_broker() {
        assert_eq!(default_provider().backend(), SymlinkBackend::WindowsBroker);
        assert_eq!(
            StdSymlinkProvider::explicit_windows_fallback().backend(),
            SymlinkBackend::WindowsStdFallback
        );
    }

    #[cfg(windows)]
    #[test]
    fn broker_error_mapping_preserves_stable_codes() {
        let cases = [
            (
                ErrorCode::ServiceNotInstalled,
                SymlinkErrorKind::BrokerUnavailable,
            ),
            (
                ErrorCode::ServiceUnavailable,
                SymlinkErrorKind::BrokerUnavailable,
            ),
            (
                ErrorCode::ServiceIdentityMismatch,
                SymlinkErrorKind::BrokerProtocolError,
            ),
            (
                ErrorCode::PrivilegeRequired,
                SymlinkErrorKind::PermissionDenied,
            ),
            (
                ErrorCode::CallerParentWriteDenied,
                SymlinkErrorKind::PermissionDenied,
            ),
            (
                ErrorCode::LinkAlreadyExists,
                SymlinkErrorKind::LinkAlreadyExists,
            ),
            (
                ErrorCode::LinkPathIsNotSymlink,
                SymlinkErrorKind::WrongSymlinkTarget,
            ),
            (
                ErrorCode::UnsafeReparsePoint,
                SymlinkErrorKind::WrongSymlinkTarget,
            ),
            (
                ErrorCode::ReplacementPartiallyCompleted,
                SymlinkErrorKind::WrongSymlinkTarget,
            ),
            (
                ErrorCode::TargetKindConflict,
                SymlinkErrorKind::UnsupportedLinkKind,
            ),
            (ErrorCode::CreateSymlinkFailed, SymlinkErrorKind::Io),
        ];

        for (broker_code, expected_kind) in cases {
            let error =
                broker_error_to_symlink_error(WinSymlinksError::new(broker_code, "broker failure"));

            assert_eq!(error.kind(), expected_kind);
            assert_eq!(error.backend(), SymlinkBackend::WindowsBroker);
            assert_eq!(error.broker_code(), Some(broker_code.to_string().as_str()));
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn default_provider_uses_std_backend_off_windows() {
        assert_eq!(default_provider().backend(), SymlinkBackend::Std);
    }

    #[cfg(unix)]
    #[test]
    fn std_provider_uses_unix_symlinks() {
        let temp_dir = TestDir::new();
        let source = temp_dir.path().join("source.txt");
        let link = temp_dir.path().join("link.txt");
        fs::write(&source, "source").unwrap();

        let mut provider = StdSymlinkProvider::new();
        provider
            .create_symlink(&source, &link, LinkKind::File)
            .unwrap();

        assert_eq!(provider.read_link(&link).unwrap(), source);
        assert!(matches!(
            provider
                .link_status(&source, &link, LinkKind::File)
                .unwrap(),
            LinkStatus::CorrectSymlink { .. }
        ));
        assert_eq!(
            provider.remove_symlink(&link).unwrap(),
            RemoveSymlinkOutcome::Removed
        );
        assert_eq!(
            provider
                .link_status(&source, &link, LinkKind::File)
                .unwrap(),
            LinkStatus::Missing
        );
    }

    #[cfg(unix)]
    #[test]
    fn std_replacement_renames_same_directory_symlink_and_preserves_real_paths() {
        for kind in [LinkKind::File, LinkKind::Directory] {
            let temp = TestDir::new();
            let source = temp.path().join("source");
            let old = temp.path().join("old");
            let link = temp.path().join("link");
            match kind {
                LinkKind::File => fs::write(&source, "new source").unwrap(),
                LinkKind::Directory => fs::create_dir(&source).unwrap(),
            }
            let mut provider = StdSymlinkProvider::new();
            // A dangling old target can be replaced without deleting the link first.
            std::os::unix::fs::symlink(&old, &link).unwrap();
            assert_eq!(
                ensure_symlink(
                    &mut provider,
                    &source,
                    &link,
                    kind,
                    CreateSymlinkOptions { force: false }
                )
                .unwrap_err()
                .kind(),
                SymlinkErrorKind::WrongSymlinkTarget
            );
            assert_eq!(
                ensure_symlink(
                    &mut provider,
                    &source,
                    &link,
                    kind,
                    CreateSymlinkOptions { force: true }
                )
                .unwrap(),
                CreateSymlinkOutcome::ReplacedWrongSymlink
            );
            assert_eq!(fs::read_link(&link).unwrap(), source);
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
            assert_eq!(
                provider
                    .replace_symlink(&old, &link, kind)
                    .unwrap_err()
                    .kind(),
                SymlinkErrorKind::SourceNotFound
            );
            assert_eq!(fs::read_link(&link).unwrap(), source);
            provider.remove_symlink(&link).unwrap();
            fs::write(&link, "user data").unwrap();
            assert_eq!(
                provider
                    .replace_symlink(&source, &link, kind)
                    .unwrap_err()
                    .kind(),
                SymlinkErrorKind::ExistingRealFile
            );
            assert_eq!(fs::read_to_string(&link).unwrap(), "user data");
            fs::remove_file(&link).unwrap();
            fs::create_dir(&link).unwrap();
            fs::write(link.join("keep"), "user data").unwrap();
            assert_eq!(
                provider
                    .replace_symlink(&source, &link, kind)
                    .unwrap_err()
                    .kind(),
                SymlinkErrorKind::ExistingRealDirectory
            );
            assert_eq!(fs::read_to_string(link.join("keep")).unwrap(), "user data");
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires running WinSymlinksBroker; run serially with --ignored --test-threads=1"]
    fn broker_native_replacement_and_error_codes() {
        use super::{
            broker_target_kind, create_symlink_via_broker, BrokerCreateSymlinkOptions,
            WindowsBrokerSymlinkProvider,
        };
        use crate::core::test_support::TestDir;
        use std::fs;

        for kind in [LinkKind::File, LinkKind::Directory] {
            for dangling in [false, true] {
                let temp = TestDir::new("broker-replace");
                let old = temp.path().join("old");
                let source = temp.path().join("source");
                let link = temp.path().join("link");
                match kind {
                    LinkKind::File => {
                        fs::write(&old, "old").unwrap();
                        fs::write(&source, "new").unwrap();
                    }
                    LinkKind::Directory => {
                        fs::create_dir(&old).unwrap();
                        fs::create_dir(&source).unwrap();
                    }
                }
                let mut provider = WindowsBrokerSymlinkProvider::new();
                provider.create_symlink(&old, &link, kind).unwrap();
                let error = create_symlink_via_broker(
                    BrokerCreateSymlinkOptions::new(&source, &link)
                        .target_kind(broker_target_kind(kind))
                        .replace_existing_symlink(false),
                )
                .unwrap_err();
                assert_eq!(error.code(), ErrorCode::LinkAlreadyExists);
                assert_eq!(
                    broker_error_to_symlink_error(error).broker_code(),
                    Some("LINK_ALREADY_EXISTS")
                );
                if dangling {
                    match kind {
                        LinkKind::File => fs::remove_file(&old).unwrap(),
                        LinkKind::Directory => fs::remove_dir(&old).unwrap(),
                    }
                }
                assert_eq!(
                    ensure_symlink(
                        &mut provider,
                        &source,
                        &link,
                        kind,
                        CreateSymlinkOptions { force: false }
                    )
                    .unwrap_err()
                    .kind(),
                    SymlinkErrorKind::WrongSymlinkTarget
                );
                assert_eq!(
                    ensure_symlink(
                        &mut provider,
                        &source,
                        &link,
                        kind,
                        CreateSymlinkOptions { force: true }
                    )
                    .unwrap(),
                    CreateSymlinkOutcome::ReplacedWrongSymlink
                );
                assert!(matches!(
                    provider.link_status(&source, &link, kind).unwrap(),
                    LinkStatus::CorrectSymlink { .. }
                ));
                let before = provider.read_link(&link).unwrap();
                assert_eq!(
                    provider
                        .replace_symlink(&old.with_file_name("missing"), &link, kind)
                        .unwrap_err()
                        .kind(),
                    SymlinkErrorKind::SourceNotFound
                );
                assert_eq!(provider.read_link(&link).unwrap(), before);
                provider.remove_symlink(&link).unwrap();
                for directory in [false, true] {
                    if directory {
                        fs::create_dir(&link).unwrap();
                    } else {
                        fs::write(&link, "user data").unwrap();
                    }
                    let error = create_symlink_via_broker(
                        BrokerCreateSymlinkOptions::new(&source, &link)
                            .target_kind(broker_target_kind(kind))
                            .replace_existing_symlink(true),
                    )
                    .unwrap_err();
                    assert_eq!(error.code(), ErrorCode::LinkPathIsNotSymlink);
                    let mapped = broker_error_to_symlink_error(error);
                    assert_eq!(mapped.kind(), SymlinkErrorKind::WrongSymlinkTarget);
                    assert_eq!(mapped.broker_code(), Some("LINK_PATH_IS_NOT_SYMLINK"));
                    if directory {
                        fs::remove_dir(&link).unwrap();
                    } else {
                        assert_eq!(fs::read_to_string(&link).unwrap(), "user data");
                        fs::remove_file(&link).unwrap();
                    }
                }
            }
        }
    }

    #[cfg(unix)]
    struct TestDir {
        path: PathBuf,
    }

    #[cfg(unix)]
    impl TestDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "agent-linker-symlink-test-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    #[cfg(unix)]
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn fs_targets_match_broken_link_with_case_variant_existing_ancestor() {
        let temp = crate::core::test_support::TestDir::new("targets-match-broken");
        let parent = temp.path().join("ParentDir").join("SubDir");
        fs::create_dir_all(&parent).unwrap();

        let link = temp.path().join("link.txt");
        let actual = parent.join("missing_leaf.txt");

        #[cfg(windows)]
        let expected = {
            let path_str = actual.to_string_lossy().to_ascii_lowercase();
            PathBuf::from(path_str)
        };
        #[cfg(not(windows))]
        let expected = actual.clone();

        assert!(fs_targets_match(&link, &actual, &expected));
    }

    #[test]
    fn fs_targets_match_broken_link_rejects_different_ancestors() {
        let temp = crate::core::test_support::TestDir::new("targets-diff-ancestors");
        let p1 = temp.path().join("ParentOne");
        let p2 = temp.path().join("ParentTwo");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        let link = temp.path().join("link.txt");
        let actual = p1.join("missing.txt");
        let expected = p2.join("missing.txt");

        assert!(!fs_targets_match(&link, &actual, &expected));
    }

    #[test]
    fn fs_targets_match_broken_link_rejects_different_missing_depths() {
        let temp = crate::core::test_support::TestDir::new("targets-diff-depths");
        let parent = temp.path().join("Parent");
        fs::create_dir_all(&parent).unwrap();

        let link = temp.path().join("link.txt");
        let actual = parent.join("deep").join("missing.txt");
        let expected = parent.join("missing.txt");

        assert!(!fs_targets_match(&link, &actual, &expected));
    }

    #[test]
    fn fs_targets_match_broken_link_rejects_case_different_tail() {
        let temp = crate::core::test_support::TestDir::new("targets-tail-case");
        let parent = temp.path().join("Parent");
        fs::create_dir_all(&parent).unwrap();

        let link = temp.path().join("link.txt");
        let actual = parent.join("MissingLeaf.txt");
        let expected = parent.join("missingleaf.txt");

        // Strict tail comparison preserves correctness for case-sensitive directories
        assert!(!fs_targets_match(&link, &actual, &expected));
    }
}
