use std::borrow::Cow;
use std::env;
use std::fs;

use tcl::interpreter::{self, Command, Context, Interpreter};
use tcl::parser;

struct Env;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage {} FILE",
            args.get(0).map(|s| s.as_str()).unwrap_or("tcl")
        );
        return;
    }

    let mut tcl = Interpreter::new(Env);
    let script = fs::read_to_string(&args[1]).expect("Error reading input file");
    let commands = parser::parse(&script).unwrap();

    tcl.eval(&commands).unwrap();
}

impl Context<'_> for Env {
    fn eval(&mut self, interpreter: Interpreter<'_, Self>, cmd: &str, args: &[Cow<str>]) -> Result<String, ()> {
        match cmd {
            "set" => interpreter::Set.eval(args),
            _ => Err(()),
        }
    }
}
