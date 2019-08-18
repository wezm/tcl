mod command;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

use crate::parser::{self, Word};
use crate::variables::substitute;

pub use command::{Command, Puts, Set};

pub type EvalResult = Result<String, Error>;
pub type Variables = HashMap<String, String>;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Arity {
        cmd: &'static str,
        expected: usize,
        received: usize,
    },
    UnknownCommand {
        cmd: String,
    },
    Conversion {
        value: String,
        message: &'static str,
    },
    Malformed {
        cmd: &'static str,
        message: &'static str,
        got: Vec<String>,
    },
}

pub trait Context<'a> {
    fn eval(
        &mut self,
        variables: &mut Variables,
        cmd: Cow<'a, str>,
        args: Vec<Cow<'a, str>>,
    ) -> EvalResult
    where
        Self: Sized;
}

pub struct Interpreter<'a, C: Context<'a>> {
    context: C,
    lifetime: PhantomData<&'a C>,
    variables: Option<Variables>,
}

impl<'a, C> Interpreter<'a, C>
where
    C: Context<'a>,
{
    pub fn new(context: C) -> Self {
        Interpreter {
            context,
            lifetime: PhantomData,
            variables: Some(HashMap::new()),
        }
    }

    pub fn eval(&mut self, commands: Vec<parser::Command<'a>>) -> EvalResult {
        let mut result = String::new();
        let mut variables = self.variables.take().unwrap();

        for command in commands {
            let mut words = command
                .0
                .into_iter()
                .map(|word| match word {
                    Word::Bare(text) => substitute(Cow::from(text), &variables).expect("FIXME"),
                    Word::Quoted(text) => substitute(unescape(text), &variables).expect("FIXME"),
                    Word::Subst(_) => unimplemented!(),
                })
                .collect::<Vec<_>>();
            let args = words.split_off(1);

            result = self
                .context
                .eval(&mut variables, words.pop().unwrap(), args)?;
        }

        self.variables.replace(variables);
        Ok(result)
    }

    pub fn context(&self) -> &C {
        &self.context
    }
}

/// Processes backslash escapes.
fn unescape(escaped: &str) -> Cow<'_, str> {
    // Benchmarks show that this check is worth it given the common case of text with
    // no escape characters.
    if escaped.contains('\\') {
        let mut result = String::with_capacity(escaped.len());
        let mut chars = escaped.chars();

        loop {
            match chars.next() {
                Some('\\') => match chars.next().expect("FIXME: truncated escape sequence") {
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    'n' => result.push('\n'),
                    c => panic!("invalid escape sequence '{}'", c),
                },
                Some(c) => result.push(c),
                None => break,
            };
        }

        Cow::from(result)
    } else {
        Cow::from(escaped)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Error::Arity {
                cmd,
                expected,
                received,
            } => write!(
                f,
                "Expected {} arguments to '{}', received {}",
                expected, cmd, received
            ),
            Error::UnknownCommand { cmd } => write!(f, "Unknown command '{}'", cmd),
            Error::Conversion { value, message } => {
                write!(f, "Unable to convert '{}': {}", value, message)
            }
            Error::Malformed { cmd, message, got } => write!(
                f,
                "Malformed '{}' command: {} got {}",
                cmd,
                message,
                got.join(" ")
            ),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::unescape;
    use easybench::bench;

    fn unescape_no_cow(escaped: &str) -> String {
        let mut result = String::new();
        let mut chars = escaped.chars();

        loop {
            match chars.next() {
                Some('\\') => match chars.next().expect("truncated escape sequence") {
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    'n' => result.push('\n'),
                    c => panic!("invalid escape sequence '{}'", c),
                },
                Some(c) => result.push(c),
                None => break,
            };
        }

        result
    }

    // #[test]
    fn bench_unescape() {
        println!(
            "unescape:        {}",
            bench(|| unescape("This is some sample text without escapes"))
        );
        println!(
            "unescape_no_cow: {}",
            bench(|| unescape_no_cow("This is some sample text without escapes"))
        );

        println!(
            "unescape esc:        {}",
            bench(|| unescape("This is some sample \\\"text\\\" with\\nescapes"))
        );
        println!(
            "unescape_no_cow esc: {}",
            bench(|| unescape_no_cow("This is some sample \\\"text\\\" with\\nescapes"))
        );
    }
}
