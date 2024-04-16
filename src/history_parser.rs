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
    "please build --help",
    "please build -h",
    "cargo run -- list",
    "cargo run -- current",
    "please ask --help",
    "please ask -h",
];

impl HistoryParser for Parser<Zsh> {
    fn parse_history(
        &self,
        history: String,
        variables: &Vec<Variable>,
    ) -> anyhow::Result<Vec<String>> {
        let mut res = vec![];
        let mut var_iter = variables.iter().rev();
        for line in history
            .lines()
            .rev()
            .map(|line| line.trim().split(";").skip(1).collect::<String>())
            .take_while(|line| !is_start_of_build(line))
        {
            assert!(!line.ends_with("\n"), "unexpected newline at {}", line);

            match line {
                cmd if IGNORED_COMMANDS.iter().any(|w| cmd.contains(w)) => {
                    // Ignore these
                }
                cmd if is_please_ask(&cmd) => {
                    let var = var_iter.next().expect("contains var");
                    // please ask How are you doing? -> read -p "How are you doing?"
                    let prompt: String = cmd
                        .split(' ')
                        .skip_while(|s| !s.starts_with("ask"))
                        .skip(1)
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim_matches('\"')
                        .to_string();

                    let cmd = format!("read -p \"{} \" {}", prompt, var.value);

                    // These need to be in reverse order here
                    res.push(var.expr.clone());
                    res.push(cmd);
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

/// Checks if the line is a please ask command
/// please ask "What is your name?" -> true
/// please ask --help -> false
/// something else -> false
fn is_please_ask(line: impl AsRef<str>) -> bool {
    let line = line.as_ref();
    let is_ask = line.contains("please ask") || line.contains("cargo run -- ask");
    is_ask && !IGNORED_COMMANDS.contains(&line)
}

/// Checks if the line is the start of build command
/// please build "script-name" -> true
/// please build -> false (finalize cmd)
fn is_start_of_build(line: impl AsRef<str>) -> bool {
    let line = line.as_ref();
    if !line.starts_with(BUILD_CMD) {
        return false;
    }
    // Trim the line and check if it starts with BUILD_CMD
    let remainder = line.trim().trim_start_matches(BUILD_CMD).trim();

    match remainder {
        "--help" | "-h" => false,
        _ => !remainder.is_empty(),
    }
}

pub fn get_parser() -> impl HistoryParser {
    Parser {
        shell: std::marker::PhantomData::<Zsh>,
    }
}

#[cfg(test)]
mod should {
    use std::fs;

    use super::*;
    #[test]
    fn find_start_of_build() {
        assert!(!is_start_of_build("please build\n"));
        assert!(!is_start_of_build("please build"));
        assert!(!is_start_of_build("echo foobar"));
        assert!(!is_start_of_build("please build -h"));
        assert!(!is_start_of_build("please build --help"));
        assert!(is_start_of_build("please build \"script-name\""));
        assert!(is_start_of_build("please build \"script-name\"\n"));
        assert!(is_start_of_build("please build ts-jest"));
        assert!(is_start_of_build("please build ts-jest\n"));
    }

    #[test]
    fn parse_zsh_history() {
        let parser = get_parser();
        let hist = fs::read_to_string("test-data/.zsh_history").unwrap();
        let vars = vec![Variable {
            value: "VAR1".to_string(),
            expr: "echo $VAR1".to_string(),
        }];
        let res = parser.parse_history(hist, &vars).unwrap();
        assert_eq!(res.len(), 6);
        assert!(res[0].starts_with("#!"))
    }

    #[test]
    fn parse_zsh_input_thing() {
        let parser = get_parser();
        let vars = vec![Variable {
            value: "VAR1".to_string(),
            expr: "echo $VAR1".to_string(),
        }];
        let hist = ": 1713204117:0;please ask \"What is your name?\"".to_string();
        let res = parser.parse_history(hist, &vars).unwrap();
        assert_eq!(res.len(), 4);
        let cmd = res[2].as_str();
        assert!(cmd.contains("read -p \"What is your name? \" VAR1"));
    }

    #[test]
    fn use_two_variables() {
        let parser = get_parser();
        let vars = vec![
            Variable {
                value: "VAR1".to_string(),
                expr: "echo $VAR1".to_string(),
            },
            Variable {
                value: "VAR2".to_string(),
                expr: "echo $VAR2".to_string(),
            },
        ];

        let hist = ": 1713204117:0;please ask \"What is your name?\"\n: 1713204117:0;please ask \"What is your age?\"".to_string();
        let res = parser.parse_history(hist, &vars).unwrap();

        assert_eq!(res.len(), 6);
        let cmd = res[2].as_str();
        assert!(cmd.contains("read -p \"What is your name? \" VAR1"));
    }

    #[test]
    fn ignore_things() {
        let parser = get_parser();
        let vars = vec![];

        let hist = fs::read_to_string("test-data/ignored_history").unwrap();
        let res = parser.parse_history(hist, &vars).unwrap();

        assert_eq!(res.len(), 3);
    }

    macro_rules! ask {
        (not $s:expr) => {
            assert!(!is_please_ask($s));
        };
        ($s:expr) => {
            assert!(is_please_ask($s));
        };
    }

    #[test]
    fn recognizes_please_ask() {
        ask!("please ask \"What is your name?\"");
        ask!(not "please ask --help");
        ask!(not "please ask -h");
    }
}
