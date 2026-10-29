use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "aglink-discovery-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(path.join(".agents")).unwrap();
        fs::write(path.join(".agents/links.toml"), "schema_version = 1\n").unwrap();
        Self(path)
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
        fs::read_to_string(project.0.join(".agents/links.toml")).unwrap(),
        "schema_version = 1\n"
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
        fs::read_to_string(project.0.join(".agents/links.toml")).unwrap(),
        "schema_version = 1\n"
    );
    assert_eq!(
        fs::read_to_string(child.join("CLAUDE.md")).unwrap(),
        "user content"
    );
}
