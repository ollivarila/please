use dirs::state_dir;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub state_dir: PathBuf,
    pub scripts_dir: PathBuf,
    pub build_file_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let state_dir = state_dir().expect("get state dir").join("please");

        let config = Config {
            scripts_dir: state_dir.join("scripts"),
            build_file_path: state_dir.join("build.json"),
            state_dir,
        };

        config.ensure_state();

        config
    }
}

impl Config {
    #[allow(unused)]
    pub fn from_base_dir(dir: impl Into<PathBuf>) -> Self {
        let state_dir: PathBuf = dir.into();

        let state_dir = state_dir.join("please");

        let config = Config {
            scripts_dir: state_dir.join("scripts"),
            build_file_path: state_dir.join("build.json"),
            state_dir,
        };

        config.ensure_state();

        config
    }
    fn ensure_state(&self) {
        if !self.state_dir.exists() {
            std::fs::create_dir_all(self.state_dir.as_path()).expect("should create state dir");
        }

        if !self.scripts_dir.exists() {
            std::fs::create_dir_all(self.scripts_dir.as_path()).expect("should create scripts dir");
        }

        assert!(
            self.state_dir.exists(),
            "path {} does not exist",
            self.state_dir.to_str().unwrap_or_default()
        );

        assert!(
            self.scripts_dir.exists(),
            "path {} does not exist",
            self.scripts_dir.to_str().unwrap_or_default()
        );
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use std::fs;

    #[test]
    fn create_config() {
        fs::create_dir("/tmp/config").unwrap();
        let config = Config::from_base_dir("/tmp/config");

        assert!(config.scripts_dir.exists());
        assert!(config.state_dir.exists());

        let expected_path = PathBuf::from("/tmp/config/please");

        assert_eq!(config.state_dir, expected_path);
        assert_eq!(config.scripts_dir, expected_path.join("scripts"));

        fs::remove_dir_all("/tmp/config").unwrap();
    }
}
