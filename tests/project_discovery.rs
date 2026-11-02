use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static PROJECT_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf, Vec<u8>);

impl Project {
    fn new() -> Self {
        for _ in 0..20 {
            let counter = PROJECT_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "aglink-discovery-cli-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                counter
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::create_dir_all(path.join(".agents")).unwrap();
                    agent_linker::core::manifest::save_manifest(
                        &path,
                        &agent_linker::core::manifest::Manifest::empty(),
                    )
                    .unwrap();
                    let original = fs::read(path.join(".agents/links.toml")).unwrap();
                    return Self(path, original);
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!(
                    "failed to create project discovery directory {}: {err}",
                    path.display()
                ),
            }
        }
        panic!("failed to create exclusive project discovery directory after multiple attempts");
    }

    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aglink"))
            .current_dir(cwd)
            .env("AGLINK_DATA_DIR", self.0.join("data"))
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn project_commands_use_the_parent_manifest_from_the_actual_working_directory() {
    let project = Project::new();
    let child = project.0.join("child/deeper");
    fs::create_dir_all(&child).unwrap();
    let status = success(project.run(&child, &["status", "--json"]));
    let status: serde_json::Value = serde_json::from_str(&status).unwrap();
    assert_eq!(
        fs::canonicalize(status["project_root"].as_str().unwrap()).unwrap(),
        fs::canonicalize(&project.0).unwrap()
    );
    success(project.run(&child, &["clean"]));
    success(project.run(&child, &["unlink", "--all"]));
    success(project.run(&child, &["db", "migrate"]));
    let source = project.0.join("source.txt");
    fs::write(&source, "source").unwrap();
    success(project.run(
        &child,
        &[
            "resource",
            "add",
            source.to_str().unwrap(),
            "--name",
            "asset",
            "--target-dir",
            "./assets",
        ],
    ));
    success(project.run(&child, &["link", "asset", "--dry-run"]));
    assert!(!project
        .run(&child, &["link", "asset", "--target-dir", ".", "--dry-run"])
        .status
        .success());
    success(project.run(
        &child,
        &[
            "link",
            "asset",
            "--target-dir",
            ".",
            "--as",
            "root-link.txt",
            "--dry-run",
        ],
    ));
    assert_eq!(fs::read_to_string(&source).unwrap(), "source");
    assert!(!child.join(".agents").exists());
    assert!(!project.0.join("assets").exists());
    assert_eq!(
        fs::read(project.0.join(".agents/links.toml")).unwrap(),
        project.1
    );
}

#[test]
fn init_stays_in_the_actual_working_directory() {
    let project = Project::new();
    let child = project.0.join("child");
    fs::create_dir(&child).unwrap();
    // A real conflict makes this independent of platform symlink privileges.
    fs::write(child.join("CLAUDE.md"), "user content").unwrap();
    let output = project.run(&child, &["init"]);
    assert!(!output.status.success());
    assert!(child.join("AGENTS.md").is_file());
    assert!(child.join(".agents/skills").is_dir());
    assert!(!project.0.join("AGENTS.md").exists());
    assert_eq!(
        fs::read(project.0.join(".agents/links.toml")).unwrap(),
        project.1
    );
    assert_eq!(
        fs::read_to_string(child.join("CLAUDE.md")).unwrap(),
        "user content"
    );
}

#[test]
fn project_fixture_concurrent_creation_and_isolation() {
    use std::thread;

    let mut handles = Vec::new();
    for _ in 0..6 {
        handles.push(thread::spawn(|| {
            let project = Project::new();
            assert!(project.0.is_dir());
            assert!(project.0.join(".agents/links.toml").is_file());
            let out = project.run(&project.0, &["status"]);
            assert!(out.status.success());
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
