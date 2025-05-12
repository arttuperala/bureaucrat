use std::env;
use std::path::Path;

pub fn truncate_path(path: &Path) -> &Path {
    let Ok(cwd) = env::current_dir() else {
        return path;
    };
    match path.strip_prefix(cwd) {
        Ok(stripped_path) => stripped_path,
        Err(_) => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::{env, fs};
    use tempfile::TempDir;
    use test_case::test_case;

    fn create_temp_dir() -> PathBuf {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        fs::canonicalize(tmp_dir.path())
            .expect("Could not get canonical path for temporary directory")
    }

    #[test]
    fn truncate_path_no_cwd() {
        let tmp_dir_path = create_temp_dir();
        let other_path = tmp_dir_path.join(".git/info/exclude");
        assert_eq!(truncate_path(&other_path), other_path);
    }

    #[test]
    fn truncate_path_sibling_directory() {
        let tmp_dir_path = create_temp_dir();
        let subdirectory = tmp_dir_path.join("src");
        fs::create_dir_all(&subdirectory).expect("Could not create directories");
        env::set_current_dir(subdirectory).expect("Could not set current directory");
        let other_path = tmp_dir_path.join(".git/info/exclude");
        assert_eq!(truncate_path(&other_path), other_path);
    }

    #[test_case(".gitignore" ; "single level")]
    #[test_case(".git/info/exclude" ; "multi level")]
    fn truncate_path_subdirectory(path: &str) {
        let tmp_dir_path = create_temp_dir();
        let subdirectory = tmp_dir_path.join(path);
        fs::create_dir_all(&subdirectory).expect("Could not create directories");
        env::set_current_dir(tmp_dir_path).expect("Could not set current directory");
        assert_eq!(truncate_path(subdirectory.as_path()), Path::new(path));
    }
}
