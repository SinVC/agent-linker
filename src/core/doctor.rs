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
use std::path::Path;
#[cfg(windows)]
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
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

    checks.push(check_broker_probe(verbose));

    let ok = checks.iter().all(|check| check.ok);
    DoctorReport { checks, ok }
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

        let probe_root = std::env::temp_dir().join(format!(
            "agent-linker-broker-probe-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let source = probe_root.join("source.txt");
        let link = probe_root.join("link.txt");

        let probe = (|| -> Result<()> {
            fs::create_dir_all(&probe_root)?;
            fs::write(&source, "probe")?;
            provider.create_symlink(&source, &link, crate::core::symlink::LinkKind::File)?;
            provider.remove_symlink(&link)?;
            Ok(())
        })();
        let _ = fs::remove_dir_all(&probe_root);

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
