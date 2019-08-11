use nom::bytes::complete::{take_while, take_while1};
use nom::character::complete::{char as chr, newline};
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    IResult,
};

use nom::branch::alt;
use nom::combinator::{all_consuming, cut, map, opt};
use nom::multi::{fold_many0, fold_many1, many0, many1, separated_list};
use nom::sequence::{delimited, preceded, terminated};

//use crate::Command;

// Commands are separated by newlines or semicolons
// New lines are ignored when inside a { } group
// When evaluating commands inside [ ] are substituted into the outer command
// $var is substituted with the value of the variable var
// Double quotes can be used to ignore special characters like space
// Each command evaluates to a single result value

pub type Word<'a> = &'a str;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token<'a> {
    List(Vec<Word<'a>>),
    Subst(Command<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Command<'a>(Vec<Token<'a>>);

fn is_space(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn is_end(c: char) -> bool {
    c == '\n' || c == ';' /* || eof */
}

fn is_word(c: char) -> bool {
    is_grouped_word(c) && !is_end(c)
}

fn ws(input: &str) -> IResult<&str, &str> {
    take_while(|c| is_space(c) || c == '\n')(input)
}

fn sep(input: &str) -> IResult<&str, &str> {
    take_while(is_space)(input)
}

fn word(input: &str) -> IResult<&str, &str> {
    take_while1(is_word)(input)
}

fn word_list(input: &str) -> IResult<&str, Vec<Word>> {
    many1(preceded(sep, word))(input)
}

// Inside a group ; and \n are allowed
fn is_grouped_word(c: char) -> bool {
    c != '{' && c != '}' && c != '[' && c != ']' /*&& c != '"'*/ && !is_space(c)
}

fn group(input: &str) -> IResult<&str, Vec<Word>> {
    preceded(
        chr('{'),
        terminated(many0(preceded(ws, word)), preceded(ws, chr('}'))),
    )(input)
}

fn subst(input: &str) -> IResult<&str, Command> {
    preceded(chr('['), terminated(command, preceded(ws, chr(']'))))(input)
}

fn command(input: &str) -> IResult<&str, Command> {
    let inner = preceded(
        sep,
        alt((
            map(word_list, Token::List),
            map(group, Token::List),
            map(subst, Token::Subst),
        )),
    );

    let cmd = terminated(
        fold_many1(inner, Vec::new(), |mut acc: Vec<_>, mut item| {
            match (item, acc.last_mut()) {
                (Token::List(ref mut list), Some(Token::List(last))) => {
                    last.append(list);
                }
                (item, _) => acc.push(item),
            }

            acc
        }),
        sep,
    );

    map(cmd, Command)(input)
}

fn just_ws(input: &str) -> IResult<&str, &str> {
    take_while1(|c| is_space(c) || c == '\n')(input)
}

pub fn parse(input: &str) -> IResult<&str, Vec<Command>> {
    if input.is_empty() {
        return Ok(("", Vec::new()));
    }

    let empty_or_commands = alt((
        map(just_ws, |_| Vec::new()),
        many1(terminated(command, many0(newline))),
    ));
    all_consuming(empty_or_commands)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_word() {
        assert!(is_word('t'));
        assert!(!is_word('['));
    }

    #[test]
    fn test_word() {
        assert_eq!(word("test"), Ok(("", "test")));
        assert_eq!(word("hello world"), Ok((" world", "hello")));
        assert_eq!(word("hello\nworld"), Ok(("\nworld", "hello")));
    }

    #[test]
    fn test_word_list() {
        assert_eq!(word_list("test"), Ok(("", vec!["test"])));
        assert_eq!(word_list("test "), Ok((" ", vec!["test"])));
        assert_eq!(word_list("hello world"), Ok(("", vec!["hello", "world"])));
        assert_eq!(word_list("hello\tworld"), Ok(("", vec!["hello", "world"])));
        assert_eq!(
            word_list("hello  \tworld"),
            Ok(("", vec!["hello", "world"]))
        );
        assert_eq!(
            word_list("hello { world }"),
            Ok((" { world }", vec!["hello"]))
        );
        assert_eq!(word_list("hello\nworld"), Ok(("\nworld", vec!["hello"])));
        assert_eq!(word_list("").is_err(), true);
    }

    //    #[test]
    //    fn test_quoted_word() {
    //        assert_eq!(grouped_word("hello\nworld"), Ok(("", "hello\nworld")));
    //    }

    #[test]
    fn test_group() {
        assert_eq!(group("{ world }"), Ok(("", vec!["world"])));
        assert_eq!(group("{world}"), Ok(("", vec!["world"])));
        assert_eq!(group("{ hello\nworld }"), Ok(("", vec!["hello", "world"])));
        assert_eq!(
            group("{\n  hello\n  world\n}"),
            Ok(("", vec!["hello", "world"]))
        );
        assert_eq!(group("{ world").is_err(), true);
    }

    #[test]
    fn test_subst() {
        assert_eq!(
            subst("[ + 1 2 ]"),
            Ok(("", Command(vec![Token::List(vec!["+", "1", "2"])])))
        );
        assert_eq!(
            subst("[ + 1 [ - 4 2 ] ]"),
            Ok((
                "",
                Command(vec![
                    Token::List(vec!["+", "1"]),
                    Token::Subst(Command(vec![Token::List(vec!["-", "4", "2"])]))
                ])
            ))
        );
    }

    #[test]
    fn test_command() {
        assert_eq!(
            command("hello { world }"),
            Ok(("", Command(vec![Token::List(vec!["hello", "world"])])))
        );
        assert_eq!(
            command("demo {\n  hello\n  world\n}"),
            Ok((
                "",
                Command(vec![Token::List(vec!["demo", "hello", "world"])])
            ))
        );
        assert_eq!(
            command("hello { world }\ndemo {\n  hello\n  world\n}\n"),
            Ok((
                "\ndemo {\n  hello\n  world\n}\n",
                Command(vec![Token::List(vec!["hello", "world"])])
            ))
        )
    }

    #[test]
    fn test_command_subst() {
        assert_eq!(
            command(r#"set subdir [ replace $version \..* "" ]"#),
            Ok((
                "",
                Command(vec![
                    Token::List(vec!["set", "subdir"]),
                    Token::Subst(Command(vec![Token::List(vec![
                        "replace", "$version", r#"\..*"#, r#""""#
                    ])]))
                ])
            ))
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse(""), Ok(("", vec![])));
        assert_eq!(parse("  "), Ok(("", vec![])));
        assert_eq!(parse("\n\n\n"), Ok(("", vec![])));
    }

    #[test]
    fn test_parse_single() {
        assert_eq!(
            parse("hello { world }"),
            Ok((
                "",
                vec![Command(vec![Token::List(vec!["hello", "world"])]),]
            ))
        );
    }

    #[test]
    fn test_parse_multiple() {
        assert_eq!(
            parse("hello { world }\ndemo {\n  hello\n  world\n}\n"),
            Ok((
                "",
                vec![
                    Command(vec![Token::List(vec!["hello", "world"])]),
                    Command(vec![Token::List(vec!["demo", "hello", "world"])])
                ]
            ))
        );
    }

    #[test]
    fn test_long_command() {
        let input = include_str!("../tests/pkg.tcl");
        let result = parse(input);
        assert_eq!(
            result,
            Ok((
                "",
                vec![
                    Command(vec![Token::List(vec!["set", "name", "ruby"])]),
                    Command(vec![Token::List(vec!["set", "version", "2.6.3"])]),
                    Command(vec![Token::List(vec!["set", "ruby_abiver", "2.6.0"])]),
                    Command(vec![
                        Token::List(vec!["set", "subdir"]),
                        Token::Subst(Command(vec![Token::List(vec![
                            "replace", "$version", r#"\..*"#, r#""""#
                        ])]))
                    ]),
                    Command(vec![Token::List(vec!["pkgname", "$name"])]),
                    Command(vec![Token::List(vec!["version", "$version"])]),
                    Command(vec![Token::List(vec!["revision", "2"])]),
                    Command(vec![Token::List(vec!["build-style", "gnu-configure"])]),
                    Command(vec![Token::List(vec![
                        "configure_args",
                        "--enable-shared",
                        "--disable-rpath",
                        "DOXYGEN",
                        "/usr/bin/doxygen",
                        "DOT",
                        "/usr/bin/dot",
                        "PKG_CONFIG",
                        "/usr/bin/pkg-config"
                    ])]),
                    Command(vec![Token::List(vec!["make_build_args", "all", "capi"])]),
                    Command(vec![Token::List(vec![
                        "hostmakedepends",
                        "pkg-config",
                        "bison",
                        "groff"
                    ])]),
                    Command(vec![Token::List(vec![
                        "makedepends",
                        "zlib-devel",
                        "readline-devel",
                        "libffi-devel",
                        "libressl-devel",
                        "gdbm-devel",
                        "libyaml-devel",
                        "pango-devel"
                    ])]),
                    Command(vec![Token::List(vec!["checkdepends", "tzdata"])]),
                    Command(vec![Token::List(vec![
                        "short_desc",
                        "\"Ruby",
                        "programming",
                        "language\""
                    ])]),
                    Command(vec![Token::List(vec![
                        "homepage",
                        "http://www.ruby-lang.org/en/"
                    ])]),
                    Command(vec![Token::List(vec![
                        "maintainer",
                        "\"Wesley",
                        "Moore",
                        "<wes@wezm.net>\""
                    ])]),
                    Command(vec![Token::List(vec!["license", "Ruby", "BSD-2-Clause"])]),
                    Command(vec![Token::List(vec![
                        "distfile",
                        "https://cache.ruby-lang.org/pub/ruby/$subdir/$pkgname-$version.tar.bz2",
                        "checksum",
                        "dd638bf42059182c1d04af0d5577131d4ce70b79105231c4cc0a60de77b14f2e"
                    ])])
                ]
            ))
        )
    }
}
