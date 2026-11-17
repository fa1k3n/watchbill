//! What `wb init` reads from a repository before it asks anything.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::{env, folder, git, put, Folder};
use watchbill::detect::{detect, git_server_of, parse_remote, project_name, Remote};

fn remote(host: &str, port: &str, path: &str) -> Remote {
    Remote { host: host.to_string(), port: port.to_string(), path: path.to_string() }
}

/// A folder that is no git repository, with the given files in it.
fn with_files(files: &[&str]) -> Folder {
    let folder = folder();
    for path in files {
        put(&folder.path, path, "");
    }
    folder
}

fn found(files: &[&str]) -> watchbill::detect::Setup {
    detect(with_files(files).path.to_str().unwrap(), Some(&env())).expect("detect failed")
}

#[test]
fn test_WB_INIT_3_an_address_is_read_in_each_of_its_forms() {
    assert_eq!(parse_remote("git@github.com:example/brake.git"), Some(remote("github.com", "", "example/brake")));
    assert_eq!(
        parse_remote("https://GitLab.example.com/group/sub/brake.git/"),
        Some(remote("gitlab.example.com", "", "group/sub/brake"))
    );
    assert_eq!(
        parse_remote("ssh://someone@review.example.com:29418/brakes/controller"),
        Some(remote("review.example.com", "29418", "brakes/controller"))
    );
    assert_eq!(parse_remote(""), None);
    assert_eq!(parse_remote("/srv/git/brake.git"), None);
}

#[test]
fn test_WB_INIT_2_the_kind_of_git_server_is_taken_from_the_host_and_the_port() {
    let kind = |url: &str| git_server_of(parse_remote(url).as_ref());

    assert_eq!(kind("git@github.com:example/brake.git"), "github");
    assert_eq!(kind("https://gitlab.example.com/group/brake.git"), "gitlab");
    assert_eq!(kind("ssh://someone@review.example.com:29418/brakes/controller"), "gerrit");
    assert_eq!(kind("https://bitbucket.org/example/brake.git"), "bitbucket");
    assert_eq!(kind("https://dev.azure.com/example/brakes/_git/controller"), "azure-devops");
    assert_eq!(kind("https://code.example.com/brake.git"), "other");
    assert_eq!(kind(""), "none");
}

#[test]
fn test_WB_INIT_1_a_project_name_is_made_from_the_name_of_the_folder() {
    assert_eq!(project_name("/work/brake-controller"), "brake-controller");
    assert_eq!(project_name("/work/My Project (2)/"), "my-project-2");
    assert_eq!(project_name("/work/___"), "project");
}

#[test]
fn test_WB_INIT_5_the_requirements_are_the_first_folder_that_holds_a_file() {
    // With the file ending most of its files have.
    let mixed = found(&["requirements/a.md", "requirements/b.md", "requirements/c.reqif"]);
    assert_eq!(mixed.requirements, "requirements/*.md");
    assert_eq!(found(&["docs/requirements/a.reqif"]).requirements, "docs/requirements/*.reqif");
    assert_eq!(found(&["requirements/sub/a.md"]).requirements, "none");
    assert_eq!(found(&[]).requirements, "none");
}

#[test]
fn test_WB_INIT_6_the_ci_driver_is_known_by_the_file_it_leaves() {
    assert_eq!(found(&[".github/workflows/ci.yaml"]).ci, "github-actions");
    assert_eq!(found(&[".github/dependabot.yml"]).ci, "none");
    assert_eq!(found(&[".gitlab-ci.yml"]).ci, "gitlab-ci");
    assert_eq!(found(&["Jenkinsfile"]).ci, "jenkins");
    assert_eq!(found(&["azure-pipelines.yml"]).ci, "azure-pipelines");
    assert_eq!(found(&["zuul.d/jobs.yaml"]).ci, "zuul");
    assert_eq!(found(&[".zuul.yaml"]).ci, "zuul");
    assert_eq!(found(&[".circleci/config.yml"]).ci, "circleci");
    assert_eq!(found(&["bitbucket-pipelines.yml"]).ci, "bitbucket-pipelines");
    assert_eq!(found(&[]).ci, "none");
}

#[test]
fn test_WB_INIT_4_the_tracker_is_the_git_server_when_that_is_github_or_gitlab() {
    let tracker = |url: &str| {
        let folder = folder();
        git(&folder.path, &["init", "-q"]);
        git(&folder.path, &["remote", "add", "origin", url]);
        detect(folder.path.to_str().unwrap(), Some(&env())).expect("detect failed").tracker
    };

    assert_eq!(tracker("git@github.com:example/brake.git"), "github");
    assert_eq!(tracker("https://gitlab.example.com/group/brake.git"), "gitlab");
    assert_eq!(tracker("https://bitbucket.org/example/brake.git"), "none");
    assert_eq!(found(&[]).tracker, "none");
}
