use tcl::interpreter::{self, Command, Interpreter};
use tcl::parser;

fn main() {
    let mut tcl = Interpreter::new(dispatch);

    // we can register a dispatch function with the interpreter at compile time, which
    // will be responsible for delegating to the commands, you can choose that include
    // whatever built in commands you want.
    let test = include_str!("../test.tcl");
    let commands = parser::parse(test).unwrap();

    tcl.eval(&commands).unwrap();
}

fn dispatch(cmd: &str) -> Option<Box<dyn Command>> {
    match cmd {
        "set" => Some(Box::new(interpreter::Set {})),
        _ => None,
    }
}
