use crate::{config::Config, script::get_scripts};
use anyhow::Context;
use clap::{command, Parser, Subcommand};
use dialoguer::Input;
use owo_colors::{OwoColorize, Style};
use script::{Script, ScriptBuilder};

mod config;
mod history_parser;
mod script;

// TODO: Add a comment to get some info about the script
// TODO: COLORIZE Maybe

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let purpel = Style::new().purple();

    if let Some(script) = args.script {
        println!("Okey, running `{}` for you!", script.style(purpel));
        parse_and_run(script)?
    } else {
        let cmd = args.command.expect("should have a command");
        cmd.run()?;
    };

    Ok(())
}

fn parse_and_run(script: String) -> anyhow::Result<()> {
    let script: Script = script.parse().context("parse script")?;
    script.run()
}

impl Command {
    fn run(self) -> anyhow::Result<()> {
        let purpel = Style::new().purple();

        match self {
            Command::Run { script } => {
                println!("Okey, running `{}` for you!", script.style(purpel));
                parse_and_run(script)?
            }
            Command::Build { script } => {
                if let Some(script) = script {
                    let builder = ScriptBuilder::build_new(&script);
                    builder.start_build()?;
                    println!("Started building script `{}` ^^", script.style(purpel));
                } else {
                    let builder = ScriptBuilder::load_current()?;
                    let name = builder.get_script_name();
                    builder.build()?;
                    println!("Built script `{}`", name.style(purpel));
                }
            }
            Command::List => {
                let scripts = get_scripts(Config::default())?;
                if scripts.is_empty() {
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
                let (var_name, var_expr, var_value) = ask_questions()?;

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
            Command::Delete { script } => {
                let script: Script = script.parse()?;
                script.try_delete()?;
            }
        };

        Ok(())
    }
}

/// Ask user for variable name, expression and value
/// and return them as a tuple in (name, expr, value) order
fn ask_questions() -> anyhow::Result<(String, String, String)> {
    let var_name = Input::<String>::new()
        .with_prompt("Variable name?")
        .interact_text()
        .context("read var name")?
        .trim()
        .to_string();

    let var_expr = Input::<String>::new()
        .with_prompt(format!("Expression with `{}`?", var_name))
        .interact_text()
        .context("read var expr")?
        .trim()
        .to_string();

    let var_value = Input::<String>::new()
        .with_prompt(format!("Value to use now?"))
        .interact_text()
        .context("read var value")?
        .trim()
        .to_string();

    Ok((var_name, var_expr, var_value))
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
