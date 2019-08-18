use std::borrow::Cow;

use super::{Error, EvalResult, Variables};

pub trait Command<'a> {
    fn eval(&self, variables: &mut Variables, args: Vec<Cow<'a, str>>) -> EvalResult;
}

pub struct Set;

pub struct Puts;

impl<'a> Command<'a> for Set {
    fn eval(&self, variables: &mut Variables, args: Vec<Cow<'a, str>>) -> EvalResult {
        if args.len() != 2 {
            return Err(Error::Arity {
                cmd: "set",
                expected: 2,
                received: args.len(),
            });
        }

        variables.insert(args[0].to_string(), args[1].to_string());

        Ok(String::new())
    }
}

impl<'a> Command<'a> for Puts {
    fn eval(&self, variables: &mut Variables, args: Vec<Cow<'a, str>>) -> EvalResult {
        println!("{}", args.join(" "));

        Ok(String::new())
    }
}
