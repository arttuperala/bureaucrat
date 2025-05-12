use crate::Error;
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, path::PathBuf};

pub static CONFIG_FILENAMES: &[&str] = &[
    ".bureaucrat-config.yaml",
    ".bureaucrat-config.yml",
    ".bureaucrat.yaml",
    ".bureaucrat.yml",
];

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub codes: Vec<String>,

    #[serde(default)]
    pub branch_prefixes: Vec<String>,
}

impl Config {
    pub fn load(path: PathBuf) -> Result<Self, Error> {
        let contents = read_to_string(path).map_err(Error::Io)?;
        serde_yaml::from_str(&contents).map_err(Error::InvalidConfiguration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    static MINIMAL_CONFIGURATION: &str = "codes:
    - GH";
    static ADVANCED_CONFIGURATION: &str = "codes:
    - GH
    - GIT
branch_prefixes:
    - feature
    - release";

    #[test]
    fn load_config() {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        let config_path = tmp_dir.path().join("config.yaml");
        let mut file = fs::File::create(&config_path).expect("Could not create configuration file");
        file.write_all(MINIMAL_CONFIGURATION.as_bytes())
            .expect("Could not write configuration file");
        assert_eq!(
            Config::load(config_path).expect("Could not load test configuration"),
            Config {
                codes: Vec::from([String::from("GH")]),
                branch_prefixes: Vec::new()
            }
        );
    }

    #[test]
    fn load_config_branch_prefixes() {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        let config_path = tmp_dir.path().join("config.yaml");
        let mut file = fs::File::create(&config_path).expect("Could not create configuration file");
        file.write_all(ADVANCED_CONFIGURATION.as_bytes())
            .expect("Could not write configuration file");
        assert_eq!(
            Config::load(config_path).expect("Could not load test configuration"),
            Config {
                codes: Vec::from([String::from("GH"), String::from("GIT")]),
                branch_prefixes: Vec::from([String::from("feature"), String::from("release")])
            }
        );
    }

    #[test]
    fn load_config_invalid() {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        let config_path = tmp_dir.path().join("config.yaml");
        let mut file = fs::File::create(&config_path).expect("Could not create configuration file");
        file.write_all("not a config".as_bytes())
            .expect("Could not write configuration file");
        assert!(matches!(
            Config::load(config_path),
            Err(Error::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn load_config_no_file() {
        let tmp_dir = TempDir::new().expect("Could not create temporary directory");
        let config_path = tmp_dir.path().join("config.yaml");
        assert!(matches!(Config::load(config_path), Err(Error::Io(_))));
    }
}
