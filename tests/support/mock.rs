use agent_linker::core::symlink::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    File,
    Directory,
    Symlink { target: PathBuf, kind: LinkKind },
}
#[derive(Default)]
pub struct MockSymlinkProvider {
    pub entries: BTreeMap<PathBuf, Entry>,
    pub mutations: Vec<PathBuf>,
    fail_replace: bool,
    pub observe_manifest: Option<(PathBuf, Vec<u8>)>,
}
impl MockSymlinkProvider {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_file(&mut self, p: impl Into<PathBuf>) {
        self.entries.insert(p.into(), Entry::File);
    }
    pub fn add_dir(&mut self, p: impl Into<PathBuf>) {
        self.entries.insert(p.into(), Entry::Directory);
    }
    pub fn add_symlink(
        &mut self,
        p: impl Into<PathBuf>,
        target: impl Into<PathBuf>,
        kind: LinkKind,
    ) {
        self.entries.insert(
            p.into(),
            Entry::Symlink {
                target: target.into(),
                kind,
            },
        );
    }
    pub fn fail_next_replacement(&mut self) {
        self.fail_replace = true;
    }
    fn error(&self, kind: SymlinkErrorKind, link: &Path) -> SymlinkError {
        SymlinkError::new(kind, self.backend()).with_link(link)
    }
    fn observe(&self) {
        if let Some((path, expected)) = &self.observe_manifest {
            assert_eq!(
                std::fs::read(path).unwrap(),
                *expected,
                "manifest was saved before all provider mutations completed"
            );
        }
    }
}
impl SymlinkProvider for MockSymlinkProvider {
    fn backend(&self) -> SymlinkBackend {
        SymlinkBackend::Mock
    }
    fn validate_source(&self, source: &Path, kind: LinkKind) -> SymlinkResult<()> {
        match (self.entries.get(source), kind) {
            (Some(Entry::File), LinkKind::File) | (Some(Entry::Directory), LinkKind::Directory) => {
                Ok(())
            }
            _ => Err(
                SymlinkError::new(SymlinkErrorKind::SourceNotFound, self.backend())
                    .with_source(source),
            ),
        }
    }
    fn create_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        self.validate_source(source, kind)?;
        if self.entries.contains_key(link) {
            return Err(self.error(SymlinkErrorKind::LinkAlreadyExists, link));
        }
        self.observe();
        self.mutations.push(link.into());
        self.add_symlink(link, source, kind);
        Ok(())
    }
    fn replace_symlink(&mut self, source: &Path, link: &Path, kind: LinkKind) -> SymlinkResult<()> {
        self.validate_source(source, kind)?;
        if self.fail_replace {
            self.fail_replace = false;
            return Err(self.error(SymlinkErrorKind::PermissionDenied, link));
        }
        if !matches!(self.entries.get(link), Some(Entry::Symlink { .. })) {
            return Err(self.error(SymlinkErrorKind::ExistingRealFile, link));
        }
        self.observe();
        self.mutations.push(link.into());
        self.add_symlink(link, source, kind);
        Ok(())
    }
    fn remove_symlink(&mut self, link: &Path) -> SymlinkResult<RemoveSymlinkOutcome> {
        match self.entries.get(link) {
            None => return Ok(RemoveSymlinkOutcome::Missing),
            Some(Entry::File) => return Err(self.error(SymlinkErrorKind::ExistingRealFile, link)),
            Some(Entry::Directory) => {
                return Err(self.error(SymlinkErrorKind::ExistingRealDirectory, link))
            }
            _ => {}
        }
        self.observe();
        self.mutations.push(link.into());
        self.entries.remove(link);
        Ok(RemoveSymlinkOutcome::Removed)
    }
    fn read_link(&self, link: &Path) -> SymlinkResult<PathBuf> {
        match self.entries.get(link) {
            Some(Entry::Symlink { target, .. }) => Ok(target.clone()),
            _ => Err(self.error(SymlinkErrorKind::Io, link)),
        }
    }
    fn link_status(
        &self,
        source: &Path,
        link: &Path,
        _kind: LinkKind,
    ) -> SymlinkResult<LinkStatus> {
        Ok(match self.entries.get(link) {
            None => LinkStatus::Missing,
            Some(Entry::File) => LinkStatus::ExistingRealFile,
            Some(Entry::Directory) => LinkStatus::ExistingRealDirectory,
            Some(Entry::Symlink { target, .. }) if target != source => {
                LinkStatus::WrongSymlinkTarget {
                    actual: target.clone(),
                    expected: source.into(),
                }
            }
            Some(Entry::Symlink { target, .. }) if !self.entries.contains_key(target) => {
                LinkStatus::BrokenSymlink {
                    target: target.clone(),
                }
            }
            Some(Entry::Symlink { target, .. }) => LinkStatus::CorrectSymlink {
                target: target.clone(),
            },
        })
    }
}
