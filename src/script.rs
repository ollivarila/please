use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, read_dir},
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{ensure, Context};

use crate::history_parser::{get_parser, HistoryParser};
use crate::{SCRIPTS_DIR, STATE_DIR};

/// Represents a script file in the please/scripts folder,
/// the String in the struct is the full path to the file
pub struct Script(String);

impl FromStr for Script {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ensure!(!s.is_empty(), "script name cannot be empty");

        let script_path = if s.ends_with(".sh") {
            SCRIPTS_DIR.join(s)
        } else {
            SCRIPTS_DIR.join(format!("{}.sh", s))
        };

        let path_str = script_path
            .to_str()
            .expect("convert script path to str")
            .to_string();

        Ok(Script(path_str))
    }
}

impl ToString for Script {
    fn to_string(&self) -> String {
        self.script_name().to_string()
    }
}

impl Script {
    pub fn run(&self) -> anyhow::Result<()> {
        let path = PathBuf::from(&self.0);
        ensure!(
            path.exists(),
            "Script `{}` does not exist",
            self.script_name()
        );

        let status = std::process::Command::new("sh")
            .arg(path)
            .status()
            .context("run script")?;

        ensure!(status.success(), "Script exited with error");

        Ok(())
    }

    pub fn edit(&self) {
        todo!()
    }

    /// Returns script name i.e script
    pub fn script_name(&self) -> &str {
        let path = Path::new(&self.0);
        path.file_stem()
            .expect("extract file stem")
            .to_str()
            .expect("convert to str")
    }

    pub fn start_build(&self, save_path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let file_path: PathBuf = save_path.into();
        let file_path = if file_path.is_dir() {
            file_path.join("build.json")
        } else {
            file_path
        };

        ensure!(
            !file_path.exists(),
            "Seems like you are already building a script"
        );

        let script_path = Path::new(&self.0);
        let script_name = script_path
            .file_stem()
            .context("extract file stem")?
            .to_str()
            .context("convert to str")?;

        let build_file = BuildFile::new(script_name);
        build_file.save_as_new(file_path)
    }
}

pub fn get_scripts() -> anyhow::Result<Vec<Script>> {
    let scripts = read_dir(SCRIPTS_DIR.clone()).context("read scripts dir")?;
    let scripts = scripts
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()
                .expect("convert to str")
                .parse()
                .ok()
        })
        .collect::<Vec<Script>>();

    Ok(scripts)
}

pub struct ScriptBuilder {
    build_file: BuildFile,
}

impl ScriptBuilder {
    pub fn load_current() -> anyhow::Result<Self> {
        let builder = Self {
            build_file: BuildFile::current_build()?,
        };

        Ok(builder)
    }

    pub fn build(self) -> anyhow::Result<()> {
        let name = self.build_file.script_name.clone();
        assert!(!name.ends_with(".sh"), "invalid name");
        let path = SCRIPTS_DIR.join(format!("{name}.sh"));

        let mut script = fs::File::create(path).context("create script file")?;

        let content = self.parse_lines()?.join("\n");

        script
            .write_all(&content.as_bytes())
            .context("write contents to script")?;

        self.delete_build()?;

        Ok(())
    }

    fn parse_lines(&self) -> anyhow::Result<Vec<String>> {
        let history = get_histfile();
        let contents = fs::read_to_string(history).context("read histfile")?;
        let parser = get_parser();

        parser.parse_history(contents, &self.build_file.variables)
    }

    pub fn display_script(&self) -> anyhow::Result<()> {
        let lines = self.parse_lines()?;
        let script = lines.join("\n");
        println!("{}", script);

        Ok(())
    }

    pub fn delete_build(&self) -> anyhow::Result<()> {
        let file = STATE_DIR.join("build.json");
        assert!(file.exists(), "Build file does not exist");
        fs::remove_file(file).context("remove build file")
    }

    pub fn add_var(&mut self, var_name: String, var_expr: String) {
        self.build_file.variables.push(Variable {
            value: var_name,
            expr: var_expr,
        })
    }

    pub fn save_replace(&self) -> anyhow::Result<()> {
        self.build_file.save_replace(STATE_DIR.join("build.json"))
    }
}

fn get_histfile() -> String {
    if let Some(hist) = env::var_os("HISTFILE") {
        return hist.to_str().expect("convert to str").to_string();
    }
    let Ok(shell) = env::var("SHELL") else {
        panic!("Shell variable not set, cannot determine histfile");
    };

    let home = home_dir().expect("home dir");

    let path = match shell.as_str() {
        "/bin/zsh" => home.join(".zsh_history"),
        "/bin/bash" => home.join(".bash_history"),
        _ => unimplemented!("Cannot get histfile for this shell: {}", shell),
    };

    path.to_str().expect("path to str").to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct BuildFile {
    script_name: String,
    variables: Vec<Variable>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variable {
    pub value: String,
    pub expr: String,
}

impl BuildFile {
    fn new(script_name: impl Into<String>) -> Self {
        Self {
            script_name: script_name.into(),
            variables: Vec::new(),
        }
    }

    fn save_as_new(&self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path: PathBuf = path.into();
        ensure!(
            !path.exists(),
            "Build file already exists, cannot overwrite"
        );

        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self).context("write to build file")
    }

    fn save_replace(&self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path: PathBuf = path.into();
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self).context("write to build file")
    }

    fn current_build() -> anyhow::Result<Self> {
        let file_path = STATE_DIR.join("build.json");
        ensure!(file_path.exists(), "No build file found");

        let file = std::fs::File::open(file_path)?;
        serde_json::from_reader(file).context("parse build file")
    }
}

#[cfg(test)]
mod should {

    use super::*;

    #[test]
    fn parse_script() {
        let script: Script = "test.sh".parse().expect("parse script");
        assert_eq!(script.0, SCRIPTS_DIR.join("test.sh").to_str().unwrap());

        let script: Script = "test".parse().expect("parse script");
        assert_eq!(script.0, SCRIPTS_DIR.join("test.sh").to_str().unwrap());
    }

    #[test]
    fn saves_build_file() {
        let build_file = BuildFile::new("test");

        build_file
            .save_as_new("/tmp/build.json")
            .expect("save build file");

        let file = std::fs::File::open("/tmp/build.json").expect("open build file");
        let build_file: BuildFile = serde_json::from_reader(file).expect("parse build file");
        assert_eq!(build_file.script_name, "test");

        std::fs::remove_file("/tmp/build.json").expect("remove build file");
    }

    #[test]
    fn starts_build() {
        let script: Script = "test".parse().expect("parse script");
        let build_file = "/tmp/build2.json";
        script.start_build(build_file).expect("start build");

        let file = std::fs::File::open(build_file).expect("open build file");
        let build_file: BuildFile = serde_json::from_reader(file).expect("parse build file");
        assert_eq!(build_file.script_name, "test");

        std::fs::remove_file("/tmp/build2.json").expect("remove build file");
    }
}
