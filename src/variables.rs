use crate::interpreter::Variables;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::char as chr;
use nom::combinator::{all_consuming, map};
use nom::error::ErrorKind;
use nom::multi::many1;
use nom::sequence::{delimited, preceded};
use nom::{Err, IResult};
use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum Text<'a> {
    Text(&'a str),
    Variable(&'a str),
}

fn regular_char(c: char) -> bool {
    c != '$'
}

fn inline_variable_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn bracketed_variable_char(c: char) -> bool {
    c != '{' && c != '}'
}

fn text(input: &str) -> IResult<&str, Text<'_>> {
    map(take_while1(regular_char), Text::Text)(input)
}

fn variable(input: &str) -> IResult<&str, Text<'_>> {
    map(alt((inline_variable, bracketed_variable)), Text::Variable)(input)
}

fn inline_variable(input: &str) -> IResult<&str, &str> {
    preceded(chr('$'), take_while1(inline_variable_char))(input)
}

fn bracketed_variable(input: &str) -> IResult<&str, &str> {
    delimited(tag("${"), take_while1(bracketed_variable_char), chr('}'))(input)
}

fn parse(input: &str) -> Result<Vec<Text<'_>>, Err<(&str, ErrorKind)>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let text_or_variables = many1(alt((text, variable)));
    all_consuming(text_or_variables)(input).map(|(_remaining, text)| text)
}

pub fn substitute<'a>(input: Cow<'a, str>, vars: &Variables) -> Result<Cow<'a, str>, ()> {
    match &*parse(&input).map_err(|_err| ())? {
        [Text::Text(_)] => Ok(input), // Common case, no variables return as is
        fragments => {
            let string = fragments
                .iter()
                .fold(String::new(), |mut string, fragment| {
                    match fragment {
                        Text::Text(s) => string.push_str(s),
                        Text::Variable(name) => {
                            string.push_str(vars.get(*name).map(String::as_str).unwrap_or(""))
                        }
                    }
                    string
                });
            Ok(Cow::from(string))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_empty_string() {
        assert_eq!(parse(""), Ok(vec![]));
    }

    #[test]
    fn test_parse_no_variable() {
        assert_eq!(
            parse("Just some text"),
            Ok(vec![Text::Text("Just some text")])
        );
    }

    #[test]
    fn test_parse_inline_variable() {
        assert_eq!(
            parse("Just some $thing"),
            Ok(vec![Text::Text("Just some "), Text::Variable("thing")])
        );
    }

    #[test]
    fn test_parse_bracketed_variable() {
        assert_eq!(
            parse("Just ${a complicated variable name!} text"),
            Ok(vec![
                Text::Text("Just "),
                Text::Variable("a complicated variable name!"),
                Text::Text(" text")
            ])
        );
    }

    #[test]
    fn test_substitute_inline_variable() {
        let mut variables = HashMap::new();
        variables.insert("thing".to_string(), "test".to_string());

        assert_eq!(
            substitute(Cow::from("Just some $thing"), &variables),
            Ok(Cow::from("Just some test"))
        );
    }

    #[test]
    fn test_substitute_missing_variable() {
        assert_eq!(
            substitute(Cow::from("Just some $thing"), &HashMap::new()),
            Ok(Cow::from("Just some "))
        );
    }

    #[test]
    fn test_substitute_bracketed_variable() {
        assert_eq!(
            parse("Just ${a complicated variable name!} text"),
            Ok(vec![
                Text::Text("Just "),
                Text::Variable("a complicated variable name!"),
                Text::Text(" text")
            ])
        );

        let mut variables = HashMap::new();
        variables.insert(
            "a complicated variable name!".to_string(),
            "test".to_string(),
        );

        assert_eq!(
            substitute(
                Cow::from("Just ${a complicated variable name!} text"),
                &variables
            ),
            Ok(Cow::from("Just test text"))
        );
    }
}
