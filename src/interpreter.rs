use std::borrow::Cow;
use std::marker::PhantomData;

use crate::parser::{self, Token, Word};

pub type EvalResult = Result<String, ()>;

pub trait Command<'a> {
    fn eval(&self, args: &[Cow<'a, str>]) -> EvalResult;
}

pub trait Context<'a> {
    fn eval(&mut self, cmd: &str, args: &[Cow<'a, str>]) -> EvalResult;
}

pub struct Set;

pub struct Interpreter<'a, C: Context<'a>> {
    context: C,
    lifetime: PhantomData<&'a C>,
}

impl<'a, C> Interpreter<'a, C>
where
    C: Context<'a>,
{
    pub fn new(context: C) -> Self {
        Interpreter {
            context,
            lifetime: PhantomData,
        }
    }

    pub fn eval(&mut self, commands: &'a [parser::Command<'_>]) -> EvalResult {
        let mut result = String::new();
        for command in commands {
            for token in command.0.iter() {
                match token {
                    Token::List(words) => {
                        let words: Vec<_> = words
                            .iter()
                            .map(unescape_and_substitute_variables)
                            .collect();
                        // TODO: Handle built-in commands
                        result = self
                            .context
                            .eval(words[0].as_ref(), &words[1..])
                            .expect("unknown command")

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
    fn eval(&self, args: &[Cow<'a, str>]) -> EvalResult {
        println!("{:?}", args);

        Ok(String::new())
    }
}

fn unescape_and_substitute_variables<'a>(word: &'a Word<'a>) -> Cow<'a, str> {
    match word {
        Word::Bare(s) => Cow::from(*s),
        Word::Quoted(s) => unescape(s), // TODO: substitute variables in the escaped text
    }
}

/// Processes backslash escapes.
fn unescape(escaped: &str) -> Cow<'_, str> {
    // Benchmarks show that this check is worth it given the common case of text with
    // no escape characters.
    if escaped.contains('\\') {
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

        Cow::from(result)
    } else {
        Cow::from(escaped)
    }
}

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

    #[test]
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
