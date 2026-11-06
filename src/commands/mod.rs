
use crate::ast;
use crate::interpreter::Interpreter;
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;

use std::fs;
use std::process;
use std::fs::File;
use std::io::Write;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_usage() {
    println!("Wave {} - the Tide interpreter", VERSION);
    println!();
    println!("Usage:");
    println!("  wave run <file.td> [args...]   Run a Tide program");
    println!("  wave tokens <file.td>          Show the lexer's tokens");
    println!("  wave ast <file.td>             Show the parsed syntax tree");
    println!("  wave help                      Show this message");
    println!("  wave version                   Show the version");
}

fn fail(kind: &str, message: &str) -> ! {
    eprintln!("{}: {}", kind, message);
    process::exit(1);
}

fn file_arg<'a>(args: &'a [String], command: &str) -> &'a str {
    match args.get(2) {
        Some(path) => path.as_str(),
        None => {
            eprintln!("Usage: wave {} <file.td>", command);
            process::exit(1);
        }
    }
}

fn read_source(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => fail("Error", &format!("could not read '{}': {}", path, err)),
    }
}

fn lex(source: &str) -> Vec<Token> {
    Lexer::new(source)
        .tokenize()
        .unwrap_or_else(|message| fail("Lex error", &message))
}

fn parse(tokens: Vec<Token>) -> ast::Program {
    Parser::new(tokens)
        .parse()
        .unwrap_or_else(|message| fail("Parse error", &message))
}

pub fn run(args: &[String]) {
    let command = match args.get(1) {
        Some(command) => command.as_str(),
        None => {
            print_usage();
            process::exit(1);
        }
    };

    match command {
        "run" => {
            let source = read_source(file_arg(args, "run"));
            let program = parse(lex(&source));

            let script_args: Vec<String> = args[3..].to_vec();

            if let Err(message) = Interpreter::new(script_args).run(&program) {
                fail("Runtime error", &message);
            }
        }

        "tokens" => {
            let source = read_source(file_arg(args, "tokens"));

            for token in lex(&source) {
                println!("{:?}", token);
            }
        }

        "ast" => {
            let source = read_source(file_arg(args, "ast"));
            let program = parse(lex(&source));

            println!("{:#?}", program);
        }

        "--help" | "-h" => print_usage(),

        "--version" | "-V" => {
            println!("wave {}", VERSION);
        }

        "init" => {
            println!("Initializing a new Tide project...");

            fs::create_dir_all("src").unwrap();

            let mut config = File::create_new("Tide.toml")
                .expect("Tide.toml already exists or couldn't be created");

            config.write_all(br#"
[project]
name = "MyProject"
version = "1.0.0"
authors = ["Ashley", "Twlve"]
description = "A project written in Tide"

[language]
version = "1.0"
entry = "src/main.td"

[build]
mode = "interpreted" # interpreted | compiled
output = "build/"

[compiler]
target = "native"
"#).expect("Failed to write Tide.toml");

            let mut main = File::create_new("src/main.td")
                .expect("src/main.td already exists or couldn't be created");

            main.write_all(b"print(\"Hello, World!\");").expect("Failed to write main.td");

            println!("Created Tide.toml and src/main.td");
        }

        other => {
            eprintln!("Unknown command '{}'", other);
            eprintln!();
            print_usage();
            process::exit(1);
        }
    }
}
