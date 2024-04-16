use crate::{
    config::Config,
    history_parser::{get_parser, HistoryParser},
};
use anyhow::{ensure, Context};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::{
    env,
    fs::{self, read_dir},
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

/// Represents a script file in the please/scripts folder,
/// the String in the struct is the full path to the file
pub struct Script(String);

impl FromStr for Script {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ensure!(!s.is_empty(), "script name cannot be empty");

        let config = Config::default();

        let script_path = if s.ends_with(".sh") {
            config.scripts_dir.join(s)
        } else {
            config.scripts_dir.join(format!("{}.sh", s))
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
        let path = PathBuf::from(&self.0);
        assert!(path.exists(), "script does not exist");

        let content = fs::read_to_string(&path).expect("read script file");
        let mut editor = dialoguer::Editor::new();
        let editor = editor.extension(".sh").trim_newlines(false);

        if let Some(changed_content) = editor.edit(&content).expect("open editor") {
            fs::write(path, changed_content).expect("save changes to file");
        };
    }

    /// Returns script name i.e script
    pub fn script_name(&self) -> &str {
        let path = Path::new(&self.0);
        path.file_stem()
            .expect("extract file stem")
            .to_str()
            .expect("convert to str")
    }
}

pub fn get_scripts(config: Config) -> anyhow::Result<Vec<Script>> {
    let scripts = read_dir(config.scripts_dir).context("read scripts dir")?;
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
    config: Config,
}

impl ScriptBuilder {
    pub fn build_new(script_name: impl AsRef<str>) -> Self {
        assert!(
            !script_name.as_ref().ends_with(".sh"),
            "script cannot end with .sh"
        );

        let build_file = BuildFile::new(script_name.as_ref());
        ScriptBuilder {
            build_file,
            config: Config::default(),
        }
    }

    pub fn load_current() -> anyhow::Result<Self> {
        let config = Config::default();
        let builder = Self {
            build_file: BuildFile::current_build(&config)?,
            config,
        };

        assert!(
            !builder.build_file.script_name.ends_with(".sh"),
            "script cannot end with .sh"
        );

        Ok(builder)
    }

    pub fn start_build(&self) -> anyhow::Result<()> {
        let build_file_path = self.config.state_dir.join("build.json");

        ensure!(
            !build_file_path.exists(),
            "Seems like you are already building a script"
        );

        self.build_file.save_as_new(build_file_path)
    }

    pub fn build(self) -> anyhow::Result<()> {
        let name = self.build_file.script_name.clone();
        let path = self.config.scripts_dir.join(format!("{name}.sh"));

        let mut script = fs::File::create(&path).context("create script file")?;

        let content = self.parse_lines()?.join("\n");

        script
            .write_all(content.as_bytes())
            .context("write contents to script")?;

        // Make the script executable
        let meta = fs::metadata(&path).context("get metadata")?;
        let mut perms = meta.permissions();
        perms.set_mode(0o755);

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
        assert!(
            self.config.build_file_path.exists(),
            "Build file does not exist"
        );
        fs::remove_file(&self.config.build_file_path).context("remove build file")
    }

    pub fn add_var(&mut self, var_name: String, var_expr: String) {
        self.build_file.variables.push(Variable {
            value: var_name,
            expr: var_expr,
        })
    }

    pub fn save_replace(&self) -> anyhow::Result<()> {
        self.build_file.save_replace(&self.config.build_file_path)
    }

    pub fn get_script_name(&self) -> String {
        self.build_file.script_name.clone()
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

    fn current_build(config: &Config) -> anyhow::Result<Self> {
        let file = &config.build_file_path;
        ensure!(file.exists(), "No build file found");

        let file = std::fs::File::open(file)?;
        serde_json::from_reader(file).context("parse build file")
    }
}

#[cfg(test)]
mod should {

    use super::*;

    #[test]
    fn parse_script() {
        let config = Config::default();
        let script: Script = "test.sh".parse().expect("parse script");
        assert_eq!(
            script.0,
            config.scripts_dir.join("test.sh").to_str().unwrap()
        );

        let script: Script = "test".parse().expect("parse script");
        assert_eq!(
            script.0,
            config.scripts_dir.join("test.sh").to_str().unwrap()
        );
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
    fn start_build() {
        let mut builder = ScriptBuilder::build_new("foo");
        builder.config = Config::from_base_dir("/tmp");
        builder.start_build().unwrap();

        assert_eq!(builder.get_script_name(), "foo".to_string());

        let p = PathBuf::from("/tmp/please/build.json");
        assert!(p.exists());

        fs::remove_dir_all("/tmp/please").unwrap();
    }

    #[test]
    fn give_corrent_string() {
        let script: Script = "foobar".parse().unwrap();
        let str = script.to_string();
        assert_eq!(str, "foobar".to_string())
    }

    #[test]
    fn run_script() {
        fs::create_dir("/tmp/please3").unwrap_or_default();
        let config = Config::from_base_dir("/tmp/please3");
        fs::write(config.scripts_dir.join("foo.sh"), "echo bar > /dev/null").unwrap();
        let script = Script("/tmp/please3/please/scripts/foo.sh".to_string());
        script.run().unwrap();

        fs::remove_dir_all("/tmp/please3").unwrap()
    }

    #[test]
    #[should_panic]
    fn not_run_invalid_script() {
        let script: Script = "foobar".parse().unwrap();
        script.run().unwrap()
    }

    #[test]
    fn list_scripts() {
        fs::create_dir("/tmp/please2").unwrap();
        let config = Config::from_base_dir("/tmp/please2");
        fs::write(config.scripts_dir.join("foo.sh"), "echo bar").unwrap();

        let scripts = get_scripts(config).unwrap();

        assert_eq!(scripts.len(), 1);

        let script = &scripts[0];
        assert_eq!(script.to_string(), "foo".to_string());

        fs::remove_dir_all("/tmp/please2").unwrap()
    }

    #[test]
    fn add_variable() {
        let config = Config::from_base_dir("/tmp/builder");
        let bf = BuildFile {
            script_name: "foo".to_string(),
            variables: vec![],
        };

        let mut builder = ScriptBuilder {
            build_file: bf,
            config: config.clone(),
        };

        builder.add_var("foo".to_string(), "bar".to_string());
        builder.save_replace().unwrap();

        let bf = BuildFile::current_build(&config).unwrap();

        assert_eq!(bf.variables.len(), 1);
        assert_eq!(bf.variables[0].value, "foo");
        assert_eq!(bf.variables[0].expr, "bar");

        fs::remove_dir_all("/tmp/builder").unwrap()
    }

    #[test]
    fn delete_build() {
        let config = Config::from_base_dir("/tmp/builder2");
        let bf = BuildFile {
            script_name: "foo".to_string(),
            variables: vec![],
        };

        let builder = ScriptBuilder {
            build_file: bf,
            config: config.clone(),
        };

        builder.start_build().unwrap();
        assert!(config.build_file_path.exists());
        builder.delete_build().unwrap();

        assert!(!config.build_file_path.exists());

        fs::remove_dir_all("/tmp/builder2").unwrap()
    }
}
