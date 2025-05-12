use assert_cmd::prelude::*;
use git2::Repository;
use predicates::prelude::*;
use std::process::Command;
use std::{env, fs};
use tempfile::TempDir;

fn binary() -> Command {
    Command::cargo_bin(env!("CARGO_PKG_NAME")).expect("Could not get binary")
}

fn create_repository() -> (TempDir, Repository) {
    let tmp_dir = TempDir::new().expect("Could not create temporary directory");
    let repository = Repository::init(tmp_dir.path()).expect("Could not create git repository");
    (tmp_dir, repository)
}

mod install {
    use super::*;

    fn create_conflicting_hook(tmp_dir: &TempDir) {
        fs::File::create(tmp_dir.path().join(".git/hooks/prepare-commit-msg"))
            .expect("Could not create hook file");
    }

    #[test]
    fn bare() {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        let _ =
            git2::Repository::init_bare(tmp_dir.path()).expect("Could not create git repository");

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path()).arg("install");
        let message = predicate::str::contains("Repository is bare; not installing hook");
        cmd.assert().failure().stderr(message);
    }

    #[test]
    fn empty() {
        let (tmp_dir, _) = create_repository();
        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path()).arg("install");
        let message = predicate::str::contains("Hook installed at .git/hooks/prepare-commit-msg");
        cmd.assert().success().stderr(message);
    }

    #[test]
    fn existing() {
        let (tmp_dir, _) = create_repository();
        create_conflicting_hook(&tmp_dir);

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path()).arg("install");
        let message =
            predicate::str::contains("Hook already exists at .git/hooks/prepare-commit-msg");
        cmd.assert().failure().stderr(message);
    }

    #[test]
    fn overwrite_existing() {
        let (tmp_dir, _) = create_repository();
        create_conflicting_hook(&tmp_dir);

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path())
            .arg("install")
            .arg("--overwrite");
        let message = predicate::str::contains("Hook installed at .git/hooks/prepare-commit-msg");
        cmd.assert().success().stderr(message);
    }
}

mod run {
    use super::*;
    use std::io::{Read, Write};
    use std::path::Path;
    use tempfile::NamedTempFile;
    use test_case::test_case;

    static GIT_COMMIT_MSG: &str = "\n# Please enter the commit message for your changes. \
        Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n";
    static CONFIGURATION: &str = "codes:\n- GH";

    fn create_commit_file() -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Couldn't create named temporary file");
        write!(file, "{}", GIT_COMMIT_MSG).expect("Couldn't write commit file");
        file
    }

    fn init_repository(repository: Repository, branch: &str) {
        let signature = git2::Signature::now("John Developer", "john@example.com")
            .expect("Could not create signature");
        let tree_id = {
            let mut index = repository
                .index()
                .expect("Could not get index for repository");
            index.write_tree().expect("Could not write tree")
        };
        let tree = repository.find_tree(tree_id).expect("Could not find tree");
        repository
            .set_head(&format!("refs/heads/{}", branch))
            .expect("Could not set HEAD");
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )
            .expect("Could not create initial commit");
    }

    fn read_commit_file(file: NamedTempFile) -> String {
        let mut file = fs::File::open(file.path()).expect("Could not open commit file");
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)
            .expect("Could not read commit file");
        file_content
    }

    fn write_configuration(root: &Path) {
        let path = root.join(".bureaucrat-config.yaml");
        let mut config_file = fs::File::create(path).expect("Could not create configuration file");
        write!(config_file, "{}", CONFIGURATION).expect("Couldn't write configuration");
    }

    #[test]
    fn empty_type() {
        let (tmp_dir, repository) = create_repository();
        init_repository(repository, "feature/GH-123-test");
        write_configuration(tmp_dir.path());
        let commit_file = create_commit_file();

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path())
            .arg("run")
            .arg(commit_file.path());
        cmd.assert().success().stderr("");

        let file_content = read_commit_file(commit_file);
        assert_eq!(file_content, format!("\n\nGH-123{}", GIT_COMMIT_MSG));
    }

    #[test]
    fn no_branch() {
        let (tmp_dir, _) = create_repository();
        write_configuration(tmp_dir.path());
        let commit_file = create_commit_file();

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path())
            .arg("run")
            .arg(commit_file.path());
        let message = predicate::str::contains("Branch doesn't exist yet");
        cmd.assert().success().stderr(message);

        let file_content = read_commit_file(commit_file);
        assert_eq!(file_content, GIT_COMMIT_MSG);
    }

    #[test]
    fn no_configuration() {
        let (tmp_dir, repository) = create_repository();
        init_repository(repository, "feature/GH-123-test");
        let commit_file = create_commit_file();

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path())
            .arg("run")
            .arg(commit_file.path());
        let message = predicate::str::contains("No configuration file was found");
        cmd.assert().success().stderr(message);

        let file_content = read_commit_file(commit_file);
        assert_eq!(file_content, GIT_COMMIT_MSG);
    }

    #[test]
    fn no_repository() {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        let commit_file = create_commit_file();

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path())
            .arg("run")
            .arg(commit_file.path());
        let message = predicate::str::contains("Could not find repository");
        cmd.assert().failure().stderr(message);

        let file_content = read_commit_file(commit_file);
        assert_eq!(file_content, GIT_COMMIT_MSG);
    }

    #[test]
    fn template() {
        let (tmp_dir, repository) = create_repository();
        init_repository(repository, "feature/GH-123-test");
        write_configuration(tmp_dir.path());
        let commit_file = create_commit_file();

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path())
            .arg("run")
            .arg(commit_file.path())
            .arg("template");
        cmd.assert().success().stderr("");

        let file_content = read_commit_file(commit_file);
        assert_eq!(file_content, format!("\n\nGH-123{}", GIT_COMMIT_MSG));
    }

    #[test_case("commit" ; "commit")]
    #[test_case("merge" ; "merge")]
    #[test_case("message" ; "message")]
    #[test_case("squash" ; "squash")]
    fn no_op(commit_source: &str) {
        let (tmp_dir, repository) = create_repository();
        init_repository(repository, "feature/GH-123-test");
        let commit_file = create_commit_file();

        let mut cmd = binary();
        cmd.current_dir(tmp_dir.path())
            .arg("run")
            .arg(commit_file.path())
            .arg(commit_source);
        cmd.assert().success().stderr("");

        let file_content = read_commit_file(commit_file);
        assert_eq!(file_content, GIT_COMMIT_MSG);
    }
}
