use std::env;
use std::fs;

use tcl::interpreter::{self, Command, Interpreter};
use tcl::parser;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage {} FILE",
            args.get(0).map(|s| s.as_str()).unwrap_or("tcl")
        );
        return;
    }

    let mut tcl = Interpreter::new(dispatch);

    // we can register a dispatch function with the interpreter at compile time, which
    // will be responsible for delegating to the commands, you can choose that include
    // whatever built in commands you want.
    let script = fs::read_to_string(&args[1]).expect("Error reading input file");
    let commands = parser::parse(&script).unwrap();

    tcl.eval(&commands).unwrap();
}

fn dispatch(cmd: &str) -> Option<Box<dyn Command>> {
    match cmd {
        "set" => Some(Box::new(interpreter::Set)),
        _ => None,
    }
}
