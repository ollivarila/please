use crate::script::get_scripts;
use anyhow::Context;
use clap::{command, Parser, Subcommand};
use dirs::home_dir;
use lazy_static::lazy_static;
use owo_colors::{OwoColorize, Style};
use script::{Script, ScriptBuilder};
use std::{io::stdin, path::PathBuf};

mod history_parser;
mod script;

lazy_static! {
    static ref STATE_DIR: PathBuf = home_dir()
        .expect("should have home dir defined")
        .join(".local/state/please");
    static ref SCRIPTS_DIR: PathBuf = STATE_DIR.join("scripts");
}

// TODO: chmod +x the script
// TODO: Add a comment to get some info about the script
// TODO: Add newline at end of script
// TODO: Add message at end of build
// TODO: COLORIZE Maybe
// TODO: Better error handling and messages, build file needss to be delete perhaps in some cases?
// TODO: Maybe test some edge cases
// TODO: Add README

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    ensure_state();

    let purpel = Style::new().purple();

    if let Some(script) = args.script {
        println!("Okey, running `{}` for you!", script.style(purpel));
        parse_an_run(script)?
    } else {
        let cmd = args.command.expect("should have a command");
        handle_command(cmd)?;
    };

    Ok(())
}

fn parse_and_run(script: String) -> anyhow::Result<()> {
    let script: Script = script.parse().context("parse script")?;
    script.run()
}

fn handle_command(cmd: Command) -> anyhow::Result<()> {
    let purpel = Style::new().purple();

    match cmd {
        Command::Run { script } => {
            println!("Okey, running `{}` for you!", script.style(purpel));
            parse_and_run(script)?
        }
        Command::Build { script } => {
            if let Some(script) = script {
                let script: Script = script.parse().context("parse script")?;
                script.start_build(STATE_DIR.clone())?;
                println!("Started building script `{}` ^^", script.script_name());
            } else {
                let builder = ScriptBuilder::load_current()?;
                builder.build()?;
            }
        }
        Command::List => {
            let scripts = get_scripts()?;
            if scripts.len() == 0 {
                println!("Looks like you don't have any scripts yet!");
                println!("You can start creating one with `please create <script name>` ^^");
                return Ok(());
            }
            println!("Here are your scripts: ^^");
            for script in scripts {
                println!("\t{}", script.script_name().style(purpel));
            }
        }
        Command::Current => {
            let builder = ScriptBuilder::load_current()?;
            println!("This is what your current script looks like: ^^\n");
            builder.display_script()?;
        }
        Command::Edit { script } => {
            let script: Script = script.parse().context("parse script")?;
            script.edit()
        }
        Command::Reset => {
            let builder = ScriptBuilder::load_current()?;
            builder.delete_build()?;
            println!("Build deleted ^^");
        }
        Command::Ask { words: _ } => {
            let mut builder = ScriptBuilder::load_current()?;
            let mut var_name = String::new();
            let mut var_expr = String::new();
            let mut var_value = String::new();

            println!("Please enter variable name:");
            stdin()
                .read_line(&mut var_name)
                .expect("read variable name");

            println!(
                "Please enter the expression for the variable `{}`:",
                var_name
            );
            stdin().read_line(&mut var_expr).expect("read expr");

            println!("Please enter the value for the variable `{}`:", var_name);
            stdin().read_line(&mut var_value).expect("read value");

            let var_expr = var_expr.trim().to_string();
            let var_name = var_name.trim().to_string();
            let var_value = var_value.trim().to_string();

            // Add var to build cache
            builder.add_var(var_name.clone(), var_expr.clone());

            // Set var in env
            std::env::set_var(var_name, var_value);

            // Run command for user
            std::process::Command::new("sh")
                .arg("-c")
                .arg(var_expr)
                .status()
                .expect("run command");

            // Save build cache
            builder.save_replace()?;
        }
        Command::Delete { script: _ } => {
            todo!()
        }
    };

    Ok(())
}

fn ensure_state() {
    if !STATE_DIR.exists() {
        std::fs::create_dir(STATE_DIR.as_path()).expect("should create state dir");
    }

    if !SCRIPTS_DIR.exists() {
        std::fs::create_dir(SCRIPTS_DIR.as_path()).expect("should create scripts dir");
    }

    assert!(STATE_DIR.exists(), "~/.local/state/please does not exist");
    assert!(
        SCRIPTS_DIR.exists(),
        "~/.local/state/please/scripts does not exist"
    );
}

#[derive(Parser, Debug)]
#[command(version, about, arg_required_else_help = true)]
struct Args {
    script: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Run a script")]
    Run {
        #[arg(help = "Name of the script you want to run")]
        script: String,
    },
    #[command(about = "Build current script")]
    Build {
        #[arg(help = "Name of the script you want to create")]
        script: Option<String>,
    },
    #[command(about = "List created scripts")]
    List,
    #[command(about = "Show what the current script looks like")]
    Current,
    #[command(about = "Open a created script in editor")]
    Edit {
        #[arg(help = "Name of the script")]
        script: String,
    },
    #[command(about = "Reset script build")]
    Reset,
    #[command(about = "Add a prompt to your script")]
    Ask { words: Vec<String> },
    #[command(about = "Delete a script")]
    Delete {
        #[arg(help = "Name of the script")]
        script: String,
    },
}
