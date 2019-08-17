use std::borrow::Cow;
use std::env;
use std::fs;

use tcl::interpreter::{self, Command, Context, Error, EvalResult, Interpreter, Variables};
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

    match tcl.eval(&commands) {
        Ok(result) => println!("{}", result),
        Err(err) => eprintln!("Error: {}", err),
    }
}

impl Context<'_> for Env {
    fn eval(&mut self, variables: &mut Variables, cmd: &str, args: &[Cow<str>]) -> EvalResult {
        match cmd {
            "set" => interpreter::Set.eval(variables, args),
            "puts" => interpreter::Puts.eval(variables, args),
            _ => Err(Error::UnknownCommand {
                cmd: cmd.to_owned(),
            }),
        }
    }
}
