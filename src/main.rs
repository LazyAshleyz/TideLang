mod ast;
mod interpreter;
mod lexer;
mod parser;
mod commands;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    commands::run(&args);
}