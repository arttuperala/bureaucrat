use crate::{config, util, Error};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{fs, io};

static HOOK_CONTENTS: &str = "#!/usr/bin/env bash
exec bureaucrat run \"$@\"";

pub struct Repository {
    pub repo: git2::Repository,
}

impl Repository {
    /// Open the Git repository we're currently in.
    pub fn open() -> Result<Repository, Error> {
        let repo = git2::Repository::open_from_env().map_err(|error| match error.code() {
            git2::ErrorCode::NotFound => Error::NoRepository,
            _ => Error::UnknownGit(error),
        })?;
        Ok(Repository { repo })
    }

    /// Find path to the Bureucrat configuration file.
    pub fn discover_config(&self) -> Result<PathBuf, Error> {
        let Some(root) = self.repo.workdir() else {
            log::warn!("Could not find work directory");
            return Err(Error::NoConfigurationFile);
        };
        for filename in config::CONFIG_FILENAMES {
            let path = root.join(filename);
            if path.exists() {
                log::debug!(
                    "Configuration file found at {}",
                    util::truncate_path(&path).display()
                );
                return Ok(path);
            }
        }
        Err(Error::NoConfigurationFile)
    }

    /// Get the name of the current HEAD branch.
    pub fn current_branch(&self) -> Result<String, Error> {
        let head = self.repo.head().map_err(|error| match error.code() {
            git2::ErrorCode::UnbornBranch => Error::NoBranch,
            _ => Error::UnknownGit(error),
        })?;
        match head.shorthand() {
            Some(head) => Ok(head.to_string()),
            None => Err(Error::NoBranch),
        }
    }

    /// Install prepare-commit-msg hook into the repository.
    pub fn install_hook(&self) -> Result<(), io::Error> {
        let mut file = fs::File::create(self.hook_path())?;
        file.write_all(HOOK_CONTENTS.as_bytes())?;
        let permissions = fs::Permissions::from_mode(0o755);
        file.set_permissions(permissions)?;
        Ok(())
    }

    pub fn hook_path(&self) -> PathBuf {
        self.repo.path().join("hooks/prepare-commit-msg")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::{env, fs};
    use tempfile::TempDir;
    use test_case::test_case;

    fn create_test_repository() -> (TempDir, Repository) {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        let repo = git2::Repository::init(tmp_dir.path()).expect("Could not create git repository");
        let repository = Repository { repo };
        (tmp_dir, repository)
    }

    #[test_case(".bureaucrat-config.yaml" ; "long yaml")]
    #[test_case(".bureaucrat-config.yml" ; "long yml")]
    #[test_case(".bureaucrat.yaml" ; "short yaml")]
    #[test_case(".bureaucrat.yml" ; "short yml")]
    fn discover_config(filename: &str) {
        let (tmp_dir, repository) = create_test_repository();
        let config_file_path = fs::canonicalize(tmp_dir.path())
            .expect("Could not get canonical path for temporary directory")
            .join(filename);
        fs::File::create(&config_file_path).expect("Could not create config file");
        assert_eq!(
            repository
                .discover_config()
                .expect("Could not discover config"),
            config_file_path
        );
    }

    #[test]
    fn discover_config_not_found() {
        let (_, repository) = create_test_repository();
        assert!(matches!(
            repository.discover_config(),
            Err(Error::NoConfigurationFile)
        ));
    }

    #[test]
    fn get_current_branch() {
        let (tmp_dir, repository) = create_test_repository();
        env::set_current_dir(tmp_dir.path()).expect("Could not set current directory");
        let signature = git2::Signature::now("John Developer", "john@example.com")
            .expect("Could not create signature");
        let tree_id = {
            let mut index = repository
                .repo
                .index()
                .expect("Could not get index for repository");
            index.write_tree().expect("Could not write tree")
        };
        let tree = repository
            .repo
            .find_tree(tree_id)
            .expect("Could not find tree");
        repository
            .repo
            .set_head("refs/heads/feature/GH-1234-test-branch")
            .expect("Could not set HEAD");
        repository
            .repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )
            .expect("Could not create initial commit");
        assert_eq!(
            repository
                .current_branch()
                .expect("Could not get current branch"),
            String::from("feature/GH-1234-test-branch")
        );
    }

    #[test]
    fn get_current_branch_unborn() {
        let (tmp_dir, repository) = create_test_repository();
        env::set_current_dir(tmp_dir.path()).expect("Could not set current directory");
        assert!(matches!(repository.current_branch(), Err(Error::NoBranch)));
    }

    #[test]
    fn install() {
        let (tmp_dir, repository) = create_test_repository();
        repository.install_hook().expect("Could not install hook");
        let expected_hook_path = fs::canonicalize(tmp_dir.path())
            .expect("Could not get canonical path for temporary directory")
            .join(".git/hooks/prepare-commit-msg");
        let mut file = fs::File::open(expected_hook_path).expect("Could not open hook file");
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)
            .expect("Could not read hook file");
        assert_eq!(file_content, HOOK_CONTENTS);
    }

    #[test]
    fn hook_path() {
        let (tmp_dir, repository) = create_test_repository();
        let expected_hook_path = fs::canonicalize(tmp_dir.path())
            .expect("Could not get canonical path for temporary directory")
            .join(".git/hooks/prepare-commit-msg");
        assert_eq!(repository.hook_path(), expected_hook_path);
    }

    #[test]
    fn open() {
        let (tmp_dir, _) = create_test_repository();
        env::set_current_dir(tmp_dir.path()).expect("Could not set current directory");
        let repository = Repository::open();
        assert!(repository.is_ok());
    }

    #[test]
    fn open_from_subdirectory() {
        let (tmp_dir, _) = create_test_repository();
        let subdirectory = tmp_dir.path().join("src/module/submodule");
        fs::create_dir_all(&subdirectory).expect("Could not create directories");
        env::set_current_dir(subdirectory).expect("Could not set current directory");
        let repository = Repository::open();
        assert!(repository.is_ok());
    }

    #[test]
    fn open_without_repository() {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        env::set_current_dir(tmp_dir.path()).expect("Could not set current directory");
        let repository = Repository::open();
        assert!(matches!(repository, Err(Error::NoRepository)));
    }
}
