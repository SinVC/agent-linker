//! Project and backend diagnostics.
#[cfg(windows)]
use crate::core::{
    error::{Error, Result},
    symlink::SymlinkBackend,
};
use crate::core::{
    manifest::{load_manifest, manifest_path},
    paths::DbPathResolution,
    store::{self, Store},
    symlink::default_provider,
};
#[cfg(windows)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fs, path::Path, path::PathBuf};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub summary: String,
    pub details: Vec<String>,
}

pub fn doctor_project(
    project_root: &Path,
    resolution: &DbPathResolution,
    verbose: bool,
) -> DoctorReport {
    doctor_project_with_probe(project_root, resolution, verbose, check_broker_probe)
}

pub fn doctor_project_with_probe(
    project_root: &Path,
    resolution: &DbPathResolution,
    verbose: bool,
    broker_probe: impl FnOnce(bool) -> DoctorCheck,
) -> DoctorReport {
    let mut checks = Vec::new();

    checks.push(match store::check_database(resolution) {
        Ok(report) => DoctorCheck {
            name: "database".to_string(),
            ok: report.is_ok(),
            summary: report.status.as_str().to_string(),
            details: verbose
                .then(|| {
                    let mut details = vec![
                        format!("path={}", report.path.display()),
                        format!("reason={}", report.reason.as_str()),
                        format!("exists={}", report.exists),
                        format!("status={}", report.status.as_str()),
                        format!(
                            "schema={}/{}",
                            report.schema_version.map_or_else(
                                || "missing".to_string(),
                                |version| version.to_string()
                            ),
                            report.latest_schema_version
                        ),
                    ];
                    details.extend(report.details.clone());
                    details
                })
                .unwrap_or_default(),
        },
        Err(error) => DoctorCheck {
            name: "database".to_string(),
            ok: false,
            summary: error.to_string(),
            details: Vec::new(),
        },
    });

    checks.push(match load_manifest(project_root) {
        Ok(manifest) => DoctorCheck {
            name: "manifest".to_string(),
            ok: true,
            summary: format!("{} managed links", manifest.links.len()),
            details: verbose
                .then(|| vec![format!("path={}", manifest_path(project_root).display())])
                .unwrap_or_default(),
        },
        Err(error) => DoctorCheck {
            name: "manifest".to_string(),
            ok: false,
            summary: error.to_string(),
            details: verbose
                .then(|| vec![format!("path={}", manifest_path(project_root).display())])
                .unwrap_or_default(),
        },
    });

    checks.push(
        match Store::open_ro(resolution).and_then(|store| store.list_harnesses()) {
            Ok(harnesses) => {
                let mapping_count: usize =
                    harnesses.iter().map(|harness| harness.mappings.len()).sum();
                DoctorCheck {
                    name: "harness-adapter".to_string(),
                    ok: true,
                    summary: format!("{} harnesses, {} mappings", harnesses.len(), mapping_count),
                    details: verbose
                        .then(|| {
                            harnesses
                                .iter()
                                .map(|harness| {
                                    format!(
                                        "{} enabled={} mappings={}",
                                        harness.name,
                                        harness.enabled,
                                        harness.mappings.len()
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            }
            Err(error) => DoctorCheck {
                name: "harness-adapter".to_string(),
                ok: false,
                summary: error.to_string(),
                details: Vec::new(),
            },
        },
    );

    let provider = default_provider();
    checks.push(DoctorCheck {
        name: "symlink-backend".to_string(),
        ok: true,
        summary: provider.backend().to_string(),
        details: verbose
            .then(|| vec![format!("backend={}", provider.backend())])
            .unwrap_or_default(),
    });
    drop(provider);

    checks.push(broker_probe(verbose));

    let ok = checks.iter().all(|check| check.ok);
    DoctorReport { checks, ok }
}

#[derive(Debug)]
pub(crate) struct ProbeDirGuard {
    path: PathBuf,
}

impl ProbeDirGuard {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ProbeDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn create_exclusive_probe_dir_in(
    base_dir: &Path,
    mut next_name: impl FnMut() -> String,
    max_attempts: usize,
) -> std::io::Result<ProbeDirGuard> {
    for _ in 0..max_attempts {
        let candidate = base_dir.join(next_name());
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(ProbeDirGuard { path: candidate }),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "failed to create exclusive probe directory after maximum attempts",
    ))
}

#[cfg(windows)]
static BROKER_PROBE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(windows)]
fn create_exclusive_probe_dir() -> std::io::Result<ProbeDirGuard> {
    create_exclusive_probe_dir_in(
        &std::env::temp_dir(),
        || {
            let counter = BROKER_PROBE_COUNTER.fetch_add(1, Ordering::Relaxed);
            format!(
                "agent-linker-broker-probe-{}-{}-{}",
                std::process::id(),
                crate::core::util::timestamp_nanos(),
                counter
            )
        },
        10,
    )
}

fn check_broker_probe(verbose: bool) -> DoctorCheck {
    #[cfg(windows)]
    {
        let mut provider = default_provider();
        if provider.backend() != SymlinkBackend::WindowsBroker {
            return DoctorCheck {
                name: "windows-broker".to_string(),
                ok: false,
                summary: format!("unexpected backend {}", provider.backend()),
                details: Vec::new(),
            };
        }

        let guard = match create_exclusive_probe_dir() {
            Ok(guard) => guard,
            Err(error) => {
                return DoctorCheck {
                    name: "windows-broker".to_string(),
                    ok: false,
                    summary: format!("failed to create probe directory: {error}"),
                    details: verbose
                        .then(|| vec![format!("error={error}")])
                        .unwrap_or_default(),
                };
            }
        };

        let probe_root = guard.path().to_path_buf();
        let source = probe_root.join("source.txt");
        let link = probe_root.join("link.txt");

        let probe = (|| -> Result<()> {
            fs::write(&source, "probe")?;
            provider.create_symlink(&source, &link, crate::core::symlink::LinkKind::File)?;
            provider.remove_symlink(&link)?;
            Ok(())
        })();
        drop(guard);

        match probe {
            Ok(()) => DoctorCheck {
                name: "windows-broker".to_string(),
                ok: true,
                summary: "available".to_string(),
                details: verbose
                    .then(|| {
                        vec![
                            "backend=windows-broker".to_string(),
                            format!("probe_root={}", probe_root.display()),
                        ]
                    })
                    .unwrap_or_default(),
            },
            Err(Error::Symlink(error)) => DoctorCheck {
                name: "windows-broker".to_string(),
                ok: false,
                summary: error.kind().to_string(),
                details: verbose
                    .then(|| {
                        let mut details = vec![format!("backend={}", error.backend())];
                        if let Some(system_code) = error.system_code() {
                            details.push(format!("system_code={system_code}"));
                        }
                        if let Some(broker_code) = error.broker_code() {
                            details.push(format!("broker_code={broker_code}"));
                        }
                        if let Some(detail) = error.detail() {
                            details.push(format!("detail={detail}"));
                        }
                        details
                    })
                    .unwrap_or_default(),
            },
            Err(error) => DoctorCheck {
                name: "windows-broker".to_string(),
                ok: false,
                summary: error.to_string(),
                details: Vec::new(),
            },
        }
    }

    #[cfg(not(windows))]
    {
        let _ = verbose;
        DoctorCheck {
            name: "windows-broker".to_string(),
            ok: true,
            summary: "not applicable".to_string(),
            details: Vec::new(),
        }
    }
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
