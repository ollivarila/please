use crate::script::Variable;

pub trait HistoryParser {
    fn parse_history(
        &self,
        history: String,
        variables: &Vec<Variable>,
    ) -> anyhow::Result<Vec<String>>;
}

struct Zsh;

struct Parser<Shell> {
    shell: std::marker::PhantomData<Shell>,
}

const SHEBANG: &str = "#!/bin/sh\n";
const BUILD_CMD: &str = "please build";
const IGNORED_COMMANDS: &[&str] = &[
    "please current",
    "please list",
    "please build",
    "cargo run -- list",
    "cargo run -- current",
];

impl HistoryParser for Parser<Zsh> {
    fn parse_history(
        &self,
        history: String,
        variables: &Vec<Variable>,
    ) -> anyhow::Result<Vec<String>> {
        let mut res = vec![];
        let mut var_iter = variables.iter();
        for line in history
            .lines()
            .rev()
            .map(|line| line.trim().split(";").skip(1).collect::<String>())
            .take_while(|line| !is_start_of_build(line))
        {
            assert!(!line.ends_with("\n"), "unexpected newline at {}", line);

            match line {
                cmd if cmd.contains("ask") => {
                    let var = var_iter.next().expect("contains var");
                    // please ask How are you doing? -> read -p "How are you doing?"
                    let prompt: String = cmd
                        .split(" ")
                        .skip_while(|s| !s.starts_with("ask"))
                        .skip(1)
                        .collect::<Vec<_>>()
                        .join(" ");

                    let cmd = format!("read -p \"{}\" {} ", prompt, var.value);

                    // These need to be in reverse order here
                    res.push(var.expr.clone());
                    res.push(cmd);
                }
                cmd if IGNORED_COMMANDS.iter().any(|w| cmd.contains(w)) => {
                    // Ignore these
                }
                cmd => res.push(cmd),
            }
        }

        res.push("set -e\n".to_string());
        res.push(SHEBANG.to_string());

        let correct_order = res.into_iter().rev().collect();
        Ok(correct_order)
    }
}

/// Checks if the line is the start of build command
/// please build "script-name" -> true
/// please build -> false (finalize cmd)
fn is_start_of_build(line: impl AsRef<str>) -> bool {
    dbg!(line.as_ref());
    let line = line.as_ref();
    if !line.starts_with(BUILD_CMD) {
        return false;
    }
    // Trim the line and check if it starts with BUILD_CMD
    let remainder = line.trim().trim_start_matches(BUILD_CMD);
    remainder.len() != 0
}

pub fn get_parser() -> impl HistoryParser {
    Parser {
        shell: std::marker::PhantomData::<Zsh>,
    }
}

#[cfg(test)]
mod should {
    use super::*;
    #[test]
    fn find_start_of_build() {
        assert!(!is_start_of_build("please build\n"));
        assert!(!is_start_of_build("please build"));
        assert!(is_start_of_build("please build \"script-name\""));
        assert!(is_start_of_build("please build \"script-name\"\n"));
        assert!(is_start_of_build("please build ts-jest"));
        assert!(is_start_of_build("please build ts-jest\n"));
    }
}
