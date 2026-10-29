//! Project and global path resolution entry point.

use crate::core::error::{Error, Result};
use std::{
    env, fs,
    path::{Component, Path, PathBuf},
};

const DB_FILE_NAME: &str = "agent-linker.db";
const DATA_DIR_NAME: &str = "agent-linker";

/// Shared lexical rules for project paths, independent of filesystem state.
pub fn validate_managed_relative_path(path: &Path) -> Result<()> {
    let text = path.as_os_str().to_string_lossy();
    if path.as_os_str().is_empty()
        || text.starts_with(['/', '\\'])
        || text
            .split(['/', '\\'])
            .any(|part| part == ".." || part.as_bytes().get(1) == Some(&b':'))
        || path.components().any(|part| {
            matches!(
                part,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
        || !path
            .components()
            .any(|part| matches!(part, Component::Normal(_)))
    {
        return Err(Error::invalid_arguments(format!(
            "path must be a nonempty project-relative path without a drive prefix or `..`: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn managed_relative_path_key(path: &Path) -> PathBuf {
    let path: PathBuf = path
        .components()
        .filter(|part| *part != Component::CurDir)
        .collect();
    #[cfg(windows)]
    let path = PathBuf::from(path.to_string_lossy().to_lowercase());
    path
}

/// Resolve a managed leaf without following symlink parents or creating directories.
/// The leaf itself may be the symlink that the caller intends to manage.
pub fn resolve_managed_project_path(project_root: &Path, rel_path: &Path) -> Result<PathBuf> {
    validate_managed_relative_path(rel_path)?;
    require_real_directory(project_root)?;
    require_real_directory(&project_root.join(".agents"))?;
    let mut current = project_root.to_path_buf();
    let mut parts = rel_path
        .components()
        .filter(|part| *part != Component::CurDir)
        .peekable();
    while let Some(part) = parts.next() {
        current.push(part.as_os_str());
        if parts.peek().is_some() {
            require_real_directory(&current)?;
        }
    }
    Ok(normalize_windows_path(&current))
}

fn require_real_directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(Error::project(format!(
                "managed parent must be a real directory: {}",
                path.display()
            )))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn current_project_root() -> Result<PathBuf> {
    find_project_root(&env::current_dir()?)
}

/// Find the nearest manifest, stopping before traversing a symlink directory.
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut directory = std::path::absolute(start)?;
    loop {
        require_real_directory(&directory)?;
        let manifest = resolve_managed_project_path(&directory, Path::new(".agents/links.toml"))?;
        match fs::symlink_metadata(&manifest) {
            Ok(_) => return Ok(normalize_windows_path(&directory)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if !directory.pop() {
            return Err(Error::project(format!(
                "no Agent Linker project found from {}; run aglink init",
                start.display()
            )));
        }
    }
}

/// Only ordinary short drive paths can safely lose the extended-length prefix.
pub fn normalize_windows_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        if let Some(text) = path.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
            let bytes = text.as_bytes();
            if bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && bytes[2] == b'\\'
                && std::ffi::OsStr::new(text).encode_wide().count() < 260
                && text[3..]
                    .split('\\')
                    .all(|part| !part.ends_with(['.', ' ']))
            {
                return PathBuf::from(text);
            }
        }
    }
    path.to_path_buf()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbPathReason {
    ExplicitDataDirEnv,
    WindowsInstall,
    PortableDatabase,
    PlatformDefault,
}

impl DbPathReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            DbPathReason::ExplicitDataDirEnv => "AGLINK_DATA_DIR",
            DbPathReason::WindowsInstall => "Windows installation receipt (install root/data)",
            DbPathReason::PortableDatabase => "portable release agent-linker.db next to executable",
            DbPathReason::PlatformDefault => "platform default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbPathResolution {
    pub path: PathBuf,
    pub reason: DbPathReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPlatform {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone)]
pub struct DbPathContext {
    pub aglink_data_dir: Option<PathBuf>,
    pub executable_path: Option<PathBuf>,
    pub local_appdata: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
    pub platform: TargetPlatform,
}

impl DbPathContext {
    pub fn from_environment() -> Self {
        Self {
            aglink_data_dir: env::var_os("AGLINK_DATA_DIR")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            executable_path: env::current_exe().ok(),
            local_appdata: env::var_os("LOCALAPPDATA")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            home: home_from_environment(),
            xdg_data_home: env::var_os("XDG_DATA_HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            platform: current_platform(),
        }
    }
}

pub fn resolve_database_path() -> Result<DbPathResolution> {
    resolve_database_path_with(&DbPathContext::from_environment())
}

pub fn resolve_database_path_with(context: &DbPathContext) -> Result<DbPathResolution> {
    let mut resolution = resolve_database_path_raw(context)?;
    resolution.path = normalize_windows_path(&resolution.path);
    Ok(resolution)
}

fn resolve_database_path_raw(context: &DbPathContext) -> Result<DbPathResolution> {
    if let Some(path) = &context.aglink_data_dir {
        return Ok(DbPathResolution {
            path: path.join(DB_FILE_NAME),
            reason: DbPathReason::ExplicitDataDirEnv,
        });
    }

    if let Some(executable_path) = &context.executable_path {
        if let Some(parent) = executable_path.parent() {
            if context.platform == TargetPlatform::Windows
                && executable_path
                    .file_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case("aglink.exe"))
                && parent
                    .file_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
            {
                if let Some(root) = parent.parent() {
                    let receipt_path = root.join("install-receipt.json");
                    if path_metadata(&receipt_path)?.is_some() {
                        let receipt = fs::read_to_string(&receipt_path)
                            .map_err(|error| path_error(&receipt_path, error))?;
                        let receipt: serde_json::Value =
                            serde_json::from_str(receipt.trim_start_matches('\u{feff}'))
                                .map_err(|error| path_error(&receipt_path, error))?;
                        if receipt["schema_version"] != 1
                            || receipt["product"] != "agent-linker"
                            || receipt["portable"] != false
                        {
                            return Err(path_error(
                                &receipt_path,
                                "invalid Windows installation receipt",
                            ));
                        }
                        return Ok(DbPathResolution {
                            path: root.join("data").join(DB_FILE_NAME),
                            reason: DbPathReason::WindowsInstall,
                        });
                    }
                }
            }
            let portable_path = parent.join(DB_FILE_NAME);
            if path_metadata(&portable_path)?.is_some_and(|metadata| metadata.is_file()) {
                return Ok(DbPathResolution {
                    path: portable_path,
                    reason: DbPathReason::PortableDatabase,
                });
            }
        }
    }

    Ok(DbPathResolution {
        path: platform_default_path(context)?,
        reason: DbPathReason::PlatformDefault,
    })
}

fn platform_default_path(context: &DbPathContext) -> Result<PathBuf> {
    match context.platform {
        TargetPlatform::Windows => {
            let Some(local_appdata) = &context.local_appdata else {
                return Err(Error::database(
                    "LOCALAPPDATA is required to resolve the Windows default database path",
                ));
            };
            Ok(local_appdata.join(DATA_DIR_NAME).join(DB_FILE_NAME))
        }
        TargetPlatform::Macos => {
            let Some(home) = &context.home else {
                return Err(Error::database(
                    "HOME is required to resolve the macOS default database path",
                ));
            };
            Ok(home
                .join("Library")
                .join("Application Support")
                .join(DATA_DIR_NAME)
                .join(DB_FILE_NAME))
        }
        TargetPlatform::Linux => {
            if let Some(xdg_data_home) = &context.xdg_data_home {
                return Ok(xdg_data_home.join(DATA_DIR_NAME).join(DB_FILE_NAME));
            }

            let Some(home) = &context.home else {
                return Err(Error::database(
                    "HOME is required to resolve the Linux default database path",
                ));
            };
            Ok(home
                .join(".local")
                .join("share")
                .join(DATA_DIR_NAME)
                .join(DB_FILE_NAME))
        }
    }
}

#[cfg(windows)]
fn current_platform() -> TargetPlatform {
    TargetPlatform::Windows
}

#[cfg(target_os = "macos")]
fn current_platform() -> TargetPlatform {
    TargetPlatform::Macos
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn current_platform() -> TargetPlatform {
    TargetPlatform::Linux
}

fn home_from_environment() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }

    #[cfg(not(windows))]
    {
        env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }
}

fn path_error(path: &Path, error: impl std::fmt::Display) -> Error {
    Error::database(format!("{}: {error}", path.display()))
}

fn path_metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(path_error(path, error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::TestDir;

    #[test]
    fn managed_paths_reject_unsafe_lexical_forms_without_creating_directories() {
        let project = TestDir::new("managed-paths");
        for path in [
            "",
            ".",
            "..",
            "../outside",
            "safe/../outside",
            "/absolute",
            "C:relative",
            "./C:relative",
            "safe/C:outside",
            "C:/absolute",
            r"C:\absolute",
            r"safe\..\outside",
            r"\rooted",
        ] {
            assert!(
                resolve_managed_project_path(project.path(), Path::new(path)).is_err(),
                "{path}"
            );
        }
        assert_eq!(
            resolve_managed_project_path(project.path(), Path::new("missing/leaf")).unwrap(),
            project.path().join("missing/leaf")
        );
        assert!(!project.path().join("missing").exists());
        assert!(!project.path().join(".agents").exists());
    }

    #[test]
    #[cfg_attr(
        windows,
        ignore = "requires running WinSymlinksBroker; run cargo test --locked -- --ignored --test-threads=1"
    )]
    fn managed_paths_reject_real_symlink_parents_but_allow_the_leaf() {
        let project = TestDir::new("symlink-parents");
        let outside = TestDir::new("symlink-outside");
        crate::core::test_support::directory_symlink(outside.path(), &project.path().join("alias"));
        assert!(resolve_managed_project_path(project.path(), Path::new("alias/leaf")).is_err());
        assert!(resolve_managed_project_path(project.path(), Path::new("alias")).is_ok());
        crate::core::test_support::directory_symlink(
            outside.path(),
            &project.path().join(".agents"),
        );
        assert!(resolve_managed_project_path(project.path(), Path::new("CLAUDE.md")).is_err());
    }

    #[test]
    fn project_discovery_finds_nearest_manifest_from_nested_directories() {
        let project = TestDir::new("discover-project");
        let nested = project.path().join("child/deeper");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir(project.path().join(".agents")).unwrap();
        fs::write(
            project.path().join(".agents/links.toml"),
            "schema_version = 1\n",
        )
        .unwrap();
        assert_eq!(find_project_root(&nested).unwrap(), project.path());
        fs::create_dir(project.path().join("child/.agents")).unwrap();
        fs::write(
            project.path().join("child/.agents/links.toml"),
            "schema_version = 1\n",
        )
        .unwrap();
        assert_eq!(
            find_project_root(&nested).unwrap(),
            project.path().join("child")
        );
    }

    #[test]
    #[cfg_attr(
        windows,
        ignore = "requires running WinSymlinksBroker; run cargo test --locked -- --ignored --test-threads=1"
    )]
    fn project_discovery_stops_at_a_symlink_directory() {
        let project = TestDir::new("discover-boundary");
        let outside = TestDir::new("discover-target");
        fs::create_dir(project.path().join(".agents")).unwrap();
        fs::write(
            project.path().join(".agents/links.toml"),
            "schema_version = 1\n",
        )
        .unwrap();
        fs::create_dir(outside.path().join("child")).unwrap();
        crate::core::test_support::directory_symlink(outside.path(), &project.path().join("alias"));
        assert!(find_project_root(&project.path().join("alias")).is_err());
        assert!(find_project_root(&project.path().join("alias/child")).is_err());
    }

    #[test]
    fn windows_receipt_accepts_minimal_identity_and_rejects_invalid_or_unreadable_data() {
        let project = TestDir::new("minimal-receipt");
        fs::create_dir(project.path().join("bin")).unwrap();
        let receipt = project.path().join("install-receipt.json");
        let mut context = base_context(project.path());
        context.platform = TargetPlatform::Windows;
        for contents in [
            r#"{"schema_version":1,"product":"agent-linker","portable":false}"#,
            r#"{"schema_version":1,"product":"agent-linker","portable":false,"command":"different","installed_by":"different"}"#,
        ] {
            fs::write(&receipt, contents).unwrap();
            assert_eq!(
                resolve_database_path_with(&context).unwrap().reason,
                DbPathReason::WindowsInstall
            );
        }
        for contents in [
            "broken",
            "{}",
            r#"{"schema_version":2,"product":"agent-linker","portable":false}"#,
            r#"{"schema_version":1,"product":"other","portable":false}"#,
            r#"{"schema_version":1,"product":"agent-linker","portable":true}"#,
        ] {
            fs::write(&receipt, contents).unwrap();
            assert!(resolve_database_path_with(&context).is_err(), "{contents}");
        }
        fs::remove_file(&receipt).unwrap();
        fs::create_dir(&receipt).unwrap();
        assert!(resolve_database_path_with(&context).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_normalization_preserves_long_and_non_drive_prefixes() {
        assert_eq!(
            normalize_windows_path(Path::new(r"\\?\C:\ordinary\file")),
            Path::new(r"C:\ordinary\file")
        );
        for path in [
            r"\\?\UNC\server\share\file".to_string(),
            r"\\?\Volume{abc}\file".to_string(),
            format!(r"\\?\C:\{}", "a".repeat(257)),
            r"\\?\C:\trailing.\file".to_string(),
        ] {
            assert_eq!(normalize_windows_path(Path::new(&path)), Path::new(&path));
        }
        // MAX_PATH counts UTF-16 code units, not UTF-8 bytes or Unicode characters.
        let long_unicode = format!(r"\\?\C:\{}", "😀".repeat(129));
        assert_eq!(
            normalize_windows_path(Path::new(&long_unicode)),
            Path::new(&long_unicode)
        );
        let context = DbPathContext {
            aglink_data_dir: Some(PathBuf::from(r"\\?\C:\data")),
            ..base_context(Path::new("C:/unused"))
        };
        assert_eq!(
            resolve_database_path_with(&context).unwrap().path,
            Path::new(r"C:\data\agent-linker.db")
        );
    }
    fn base_context(temp_dir: &Path) -> DbPathContext {
        DbPathContext {
            aglink_data_dir: None,
            executable_path: Some(temp_dir.join("bin").join("aglink.exe")),
            local_appdata: Some(temp_dir.join("local-appdata")),
            home: Some(temp_dir.join("home")),
            xdg_data_home: Some(temp_dir.join("xdg")),
            platform: TargetPlatform::Linux,
        }
    }

    #[test]
    fn database_path_prefers_data_dir_then_portable() {
        let temp_dir = TestDir::new("db-path-precedence");
        let portable_parent = temp_dir.path().join("bin");
        fs::create_dir(&portable_parent).unwrap();
        fs::write(portable_parent.join("agent-linker.db"), "").unwrap();

        let mut context = base_context(temp_dir.path());
        context.aglink_data_dir = Some(temp_dir.path().join("aglink-data"));

        let data_dir = resolve_database_path_with(&context).unwrap();
        assert_eq!(
            data_dir.path,
            temp_dir.path().join("aglink-data").join("agent-linker.db")
        );
        assert_eq!(data_dir.reason, DbPathReason::ExplicitDataDirEnv);

        context.aglink_data_dir = None;
        let portable = resolve_database_path_with(&context).unwrap();
        assert_eq!(portable.path, portable_parent.join("agent-linker.db"));
        assert_eq!(portable.reason, DbPathReason::PortableDatabase);
    }

    #[test]
    fn database_path_uses_platform_defaults() {
        let temp_dir = TestDir::new("db-path-default");
        let mut context = base_context(temp_dir.path());
        context.executable_path = None;

        context.platform = TargetPlatform::Windows;
        let windows = resolve_database_path_with(&context).unwrap();
        assert_eq!(
            windows.path,
            temp_dir
                .path()
                .join("local-appdata")
                .join("agent-linker")
                .join("agent-linker.db")
        );

        context.platform = TargetPlatform::Macos;
        let macos = resolve_database_path_with(&context).unwrap();
        assert_eq!(
            macos.path,
            temp_dir
                .path()
                .join("home")
                .join("Library")
                .join("Application Support")
                .join("agent-linker")
                .join("agent-linker.db")
        );

        context.platform = TargetPlatform::Linux;
        let linux = resolve_database_path_with(&context).unwrap();
        assert_eq!(
            linux.path,
            temp_dir
                .path()
                .join("xdg")
                .join("agent-linker")
                .join("agent-linker.db")
        );

        context.xdg_data_home = None;
        let linux_fallback = resolve_database_path_with(&context).unwrap();
        assert_eq!(
            linux_fallback.path,
            temp_dir
                .path()
                .join("home")
                .join(".local")
                .join("share")
                .join("agent-linker")
                .join("agent-linker.db")
        );
    }

    #[test]
    fn windows_install_receipt_precedes_portable_and_ignores_localappdata() {
        let temp = TestDir::new("installed-path");
        let root = temp.path().join("custom-install");
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin/agent-linker.db"), "").unwrap();
        fs::write(root.join("install-receipt.json"), r#"{"schema_version":1,"product":"agent-linker","command":"aglink","portable":false,"installed_by":"aglink-installer.ps1"}"#).unwrap();
        let mut context = base_context(temp.path());
        context.platform = TargetPlatform::Windows;
        context.executable_path = Some(root.join("bin/aglink.exe"));
        context.local_appdata = None;
        let resolution = resolve_database_path_with(&context).unwrap();
        assert_eq!(resolution.path, root.join("data/agent-linker.db"));
        assert_eq!(resolution.reason, DbPathReason::WindowsInstall);
        context.local_appdata = Some(temp.path().join("sandbox-appdata"));
        assert_eq!(resolve_database_path_with(&context).unwrap(), resolution);
        context.aglink_data_dir = Some(temp.path().join("override"));
        assert_eq!(
            resolve_database_path_with(&context).unwrap().reason,
            DbPathReason::ExplicitDataDirEnv
        );
        context.aglink_data_dir = None;
        fs::write(root.join("install-receipt.json"), "broken").unwrap();
        assert!(resolve_database_path_with(&context)
            .unwrap_err()
            .to_string()
            .contains("install-receipt.json"));
        fs::write(root.join("install-receipt.json"), r#"{"product":"other"}"#).unwrap();
        assert!(resolve_database_path_with(&context).is_err());
    }

    #[test]
    fn cargo_bin_without_receipt_is_not_a_normal_install() {
        let temp = TestDir::new("cargo-bin-path");
        let mut context = base_context(temp.path());
        context.platform = TargetPlatform::Windows;
        context.executable_path = Some(temp.path().join(".cargo/bin/aglink.exe"));
        assert_eq!(
            resolve_database_path_with(&context).unwrap().reason,
            DbPathReason::PlatformDefault
        );
    }
}
