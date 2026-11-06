mod ast;
mod interpreter;
mod lexer;
mod parser;

use interpreter::Interpreter;
use lexer::{Lexer, Token};
use parser::Parser;
use std::env;
use std::fs;
use std::process;

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

fn main() {
    let args: Vec<String> = env::args().collect();

    let command = match args.get(1) {
        Some(command) => command.as_str(),
        None => {
            print_usage();
            process::exit(1);
        }
    };

    match command {
        "run" => {
            let source = read_source(file_arg(&args, "run"));
            let program = parse(lex(&source));
            // Anything after the file name is passed to the script (std.args())
            let script_args: Vec<String> = args[3..].to_vec();
            if let Err(message) = Interpreter::new(script_args).run(&program) {
                fail("Runtime error", &message);
            }
        }
        "tokens" => {
            let source = read_source(file_arg(&args, "tokens"));
            for token in lex(&source) {
                println!("{:?}", token);
            }
        }
        "ast" => {
            let source = read_source(file_arg(&args, "ast"));
            let program = parse(lex(&source));
            println!("{:#?}", program);
        }
        "help" | "--help" | "-h" => print_usage(),
        "version" | "--version" | "-V" => println!("wave {}", VERSION),
        other => {
            eprintln!("Unknown command '{}'", other);
            eprintln!();
            print_usage();
            process::exit(1);
        }
    }
}