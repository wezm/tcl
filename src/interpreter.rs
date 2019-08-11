use crate::parser::{self, Token, Word};
use std::borrow::Cow;

type EvalResult = Result<String, ()>;

pub trait Command<'a> {
    fn eval(&self, args: &[Cow<'a, str>]) -> String;
}

pub struct Interpreter<F: Fn(&str) -> Option<Box<dyn Command<'_>>>> {
    dispatch: F,
}

pub struct Set {}

impl Set {
    pub const NAME: &'static str = "set";
}

impl<F> Interpreter<F>
where
    F: Fn(&str) -> Option<Box<dyn Command<'_>>>,
{
    pub fn new(dispatch: F) -> Self {
        Interpreter { dispatch }
    }

    pub fn eval(&mut self, commands: &[parser::Command<'_>]) -> EvalResult {
        let mut result = String::new();
        for command in commands {
            for token in command.0.iter() {
                match token {
                    Token::List(words) => {
                        let words: Vec<_> =
                            words.iter().map(unescape_and_substitute_variables).collect();
                        result = (self.dispatch)(words[0].as_ref())
                            .expect("unknown command")
                            .eval(&words[1..]); // TODO handle no args

                        // TODO: it will need access to the interpreter state
                        // structs for each command that impl a trait?
                        // needs to be extensible/easy to add new commands
                        // maximising static dispatch would probably beneficial
                    }
                    Token::Subst(_subst) => {}
                }
            }
        }

        Ok(result)
    }
}

impl<'a> Command<'a> for Set {
    fn eval(&self, args: &[Cow<'a, str>]) -> String {
        println!("{:?}", args);

        String::new()
    }
}

fn unescape_and_substitute_variables<'a>(word: &'a Word<'a>) -> Cow<'a, str> {
    match word {
        Word::Bare(s) => Cow::from(*s),
        Word::Quoted(s) => unescape(s), // TODO: substitute variables in the escaped text
    }
}

fn unescape(escaped: &str) -> Cow<'_, str> {
    Cow::from(escaped)
}
