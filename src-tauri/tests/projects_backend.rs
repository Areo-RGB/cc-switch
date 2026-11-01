use std::fs;

use cc_switch_lib::project_backend::{
    git::{parse_git_status_entries, safe_repo_relative_path},
    github::{filter_repos, github_slug, https_clone_url, parse_repos, valid_repo_name},
    launchers::detect_scripts,
    local_repos::scan,
};

use cc_switch_lib::project_backend::git;

#[test]
fn parses_git_porcelain_entries_and_renames() {
    let entries = parse_git_status_entries(" M src/main.rs\0R  old.rs\0new.rs\0?? notes.txt\0");

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].path, "new.rs");
    assert!(entries[0].staged);
    assert_eq!(entries[1].path, "notes.txt");
    assert!(entries[1].untracked);
    assert_eq!(entries[2].path, "src/main.rs");
    assert!(entries[2].unstaged);
}

#[test]
fn rejects_repository_paths_that_escape_the_root() {
    let root = tempfile::tempdir().unwrap();

    assert!(safe_repo_relative_path(root.path(), "src/main.rs").is_ok());
    assert!(safe_repo_relative_path(root.path(), "../secret").is_err());
    assert!(safe_repo_relative_path(root.path(), "/tmp/secret").is_err());
}

#[test]
fn detects_scripts_from_common_project_manifests() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"scripts":{"dev":"vite","test":"vitest"},"dependencies":{"react":"1"}}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\n",
    )
    .unwrap();
    fs::write(root.path().join("justfile"), "lint:\n  cargo clippy\n").unwrap();

    let scripts = detect_scripts(root.path());
    let names: Vec<_> = scripts.iter().map(|script| script.name.as_str()).collect();
    assert!(names.contains(&"dev"));
    assert!(names.contains(&"test"));
    assert!(names.contains(&"cargo-run"));
    assert!(names.contains(&"lint"));
}

#[test]
fn parses_and_filters_github_repositories_without_network() {
    let repos = parse_repos(
        r#"[
          {"name":"public","nameWithOwner":"me/public","description":"Rust tool","isPrivate":false,"isArchived":false,"url":"https://github.com/me/public","stargazerCount":3,"updatedAt":"2026-01-02T00:00:00Z","pushedAt":"2026-01-01T00:00:00Z","primaryLanguage":{"name":"Rust"}},
          {"name":"private","nameWithOwner":"me/private","description":"React app","isPrivate":true,"isArchived":false,"url":"https://github.com/me/private","stargazerCount":8,"updatedAt":"2026-01-03T00:00:00Z","pushedAt":"2026-01-04T00:00:00Z","primaryLanguage":{"name":"TypeScript"}}
        ]"#,
    );

    let filtered = filter_repos(&repos, "rust", "public", "name");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name_with_owner, "me/public");
    assert!(valid_repo_name("good-name_1"));
    assert!(!valid_repo_name("bad/name"));
    assert_eq!(
        github_slug("git@github.com:me/public.git").as_deref(),
        Some("me/public")
    );
    assert_eq!(
        https_clone_url(&repos[0]),
        "https://github.com/me/public.git"
    );
}

#[test]
fn scans_nested_local_github_repositories_with_depth_and_skip_rules() {
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("one").join("two");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::write(
        repo.join(".git/config"),
        "[remote \"origin\"]\n\turl = git@github.com:Me/Repo.git\n",
    )
    .unwrap();

    let skipped = root.path().join("node_modules").join("ignored");
    fs::create_dir_all(skipped.join(".git")).unwrap();
    fs::write(
        skipped.join(".git/config"),
        "[remote \"origin\"]\n\turl = https://github.com/me/ignored.git\n",
    )
    .unwrap();

    let found = scan(root.path());
    assert_eq!(
        found.get("me/repo").unwrap(),
        &vec![repo.display().to_string()]
    );
    assert!(!found.contains_key("me/ignored"));
}

#[test]
fn runs_local_git_status_stage_diff_and_commit_without_network() {
    let root = tempfile::tempdir().unwrap();
    git::init(root.path()).unwrap();
    git::run_git(
        root.path(),
        &[
            "config".into(),
            "user.email".into(),
            "tests@example.com".into(),
        ],
    )
    .unwrap();
    git::run_git(
        root.path(),
        &["config".into(), "user.name".into(), "Phase 3 Tests".into()],
    )
    .unwrap();
    fs::write(root.path().join("README.md"), "hello\n").unwrap();

    let status = git::git_status(root.path()).unwrap();
    assert_eq!(status.files[0].path, "README.md");
    assert!(status.files[0].untracked);
    let diff = git::diff(root.path(), "README.md", false, false, true).unwrap();
    assert!(diff.contains("+hello"));

    git::stage(root.path(), &["README.md".into()]).unwrap();
    assert!(git::git_status(root.path()).unwrap().files[0].staged);
    let commit = git::commit(root.path(), "Add readme").unwrap();
    assert!(!commit.hash.is_empty());
    git::run_git(root.path(), &["branch".into(), "feature".into()]).unwrap();
    git::checkout(root.path(), "feature").unwrap();
    assert_eq!(
        git::command_output(root.path(), &["branch", "--show-current"]),
        Some("feature".into())
    );

    fs::write(root.path().join("README.md"), "hello\nworking tree\n").unwrap();
    git::stage(root.path(), &["README.md".into()]).unwrap();
    fs::write(
        root.path().join("README.md"),
        "hello\nworking tree\nunstaged\n",
    )
    .unwrap();
    let combined_diff = git::diff(root.path(), "README.md", true, true, false).unwrap();
    assert!(combined_diff.contains("+working tree"));
    assert!(combined_diff.contains("+unstaged"));
    assert_eq!(git::git_status(root.path()).unwrap().files.len(), 1);
}
