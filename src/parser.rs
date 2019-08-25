use nom::branch::alt;
use nom::bytes::complete::{escaped, tag, take_while, take_while1};
use nom::character::complete::{char as chr, newline, one_of};
use nom::combinator::{all_consuming, map};
use nom::error::ErrorKind;
use nom::multi::{fold_many1, many0, many1};
use nom::sequence::{delimited, preceded, terminated};
use nom::{Err, IResult};

// Commands are separated by newlines or semicolons
// New lines are ignored when inside a { } group
// When evaluating commands inside [ ] are substituted into the outer command
// $var or ${var} is substituted with the value of the variable var
// Double quotes can be used to ignore special characters like space
// Each command evaluates to a single result value

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Text<'a> {
    Text(&'a str),
    Variable(&'a str),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Word<'a> {
    Bare(Vec<Text<'a>>),
    Quoted(Vec<Text<'a>>),
    Subst(Command<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token<'a> {
    List(Vec<Word<'a>>),
    Subst(Command<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Command<'a>(pub(crate) Vec<Word<'a>>);

fn is_space(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn is_end(c: char) -> bool {
    c == '\n' || c == ';'
}

fn is_word(c: char) -> bool {
    is_grouped_word(c) && !is_end(c)
}

// Inside a group ; and \n are allowed
fn is_grouped_word(c: char) -> bool {
    c != '{' && c != '}' && c != '[' && c != ']' && c != '"' && c != '$' && !is_space(c)
}

fn ws(input: &str) -> IResult<&str, &str> {
    take_while(|c| is_space(c) || c == '\n')(input)
}

fn sep(input: &str) -> IResult<&str, &str> {
    take_while(is_space)(input)
}

fn inline_variable_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn bracketed_variable_char(c: char) -> bool {
    c != '{' && c != '}'
}

fn text(input: &str) -> IResult<&str, Text<'_>> {
    map(take_while1(is_word), Text::Text)(input)
}

fn escaped_text(input: &str) -> IResult<&str, Text<'_>> {
    let allowed = take_while1(|c| c != '\\' && c != '"' && c != '$');
    map(escaped(allowed, '\\', one_of(r#"\$"n"#)), Text::Text)(input)
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

fn text_or_variable(input: &str) -> IResult<&str, Text<'_>> {
    alt((text, variable))(input)
}

fn escaped_text_or_variable(input: &str) -> IResult<&str, Text<'_>> {
    alt((escaped_text, variable))(input)
}

fn word(input: &str) -> IResult<&str, Word<'_>> {
    map(many1(text_or_variable), Word::Bare)(input)
}

fn quoted_word(input: &str) -> IResult<&str, Word<'_>> {
    map(
        alt((
            delimited(chr('"'), many1(escaped_text_or_variable), chr('"')),
            map(tag("\"\""), |_| vec![Text::Text("")]),
        )),
        Word::Quoted,
    )(input)
}

fn word_list(input: &str) -> IResult<&str, Vec<Word<'_>>> {
    many1(preceded(sep, word_or_quoted))(input)
}

fn word_or_quoted(input: &str) -> IResult<&str, Word<'_>> {
    alt((word, quoted_word))(input)
}

fn group(input: &str) -> IResult<&str, Vec<Word<'_>>> {
    preceded(
        chr('{'),
        terminated(many0(preceded(ws, word_or_quoted)), preceded(ws, chr('}'))),
    )(input)
}

fn subst(input: &str) -> IResult<&str, Command<'_>> {
    preceded(chr('['), terminated(command, preceded(ws, chr(']'))))(input)
}

fn command(input: &str) -> IResult<&str, Command<'_>> {
    let inner = preceded(
        sep,
        alt((
            map(word_list, Token::List),
            map(group, Token::List),
            map(subst, Token::Subst),
        )),
    );

    let cmd = terminated(
        fold_many1(inner, Vec::new(), |mut acc: Vec<_>, item| {
            match item {
                Token::List(mut words) => acc.append(&mut words),
                Token::Subst(subst) => acc.push(Word::Subst(subst)),
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

pub fn parse(input: &str) -> Result<Vec<Command<'_>>, Err<(&str, ErrorKind)>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let empty_or_commands = alt((
        map(just_ws, |_| Vec::new()),
        many1(terminated(command, many0(newline))),
    ));
    all_consuming(empty_or_commands)(input).map(|(_remaining, commands)| commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Word::Quoted from str
    fn q(s: &str) -> Word<'_> {
        Word::Quoted(vec![Text::Text(s)])
    }

    // Word::Bare from str
    fn b(s: &str) -> Word<'_> {
        Word::Bare(vec![Text::Text(s)])
    }

    // Variable from str
    fn v(s: &str) -> Word<'_> {
        Word::Bare(vec![Text::Variable(s)])
    }

    #[test]
    fn test_is_word() {
        assert!(is_word('t'));
        assert!(!is_word('['));
    }

    #[test]
    fn test_word() {
        assert_eq!(word("test"), Ok(("", b("test"))));
        assert_eq!(word("hello world"), Ok((" world", b("hello"))));
        assert_eq!(word("hello\nworld"), Ok(("\nworld", b("hello"))));
    }

    #[test]
    fn test_quoted_word() {
        assert_eq!(quoted_word(r#""""#), Ok(("", q(""))));
        assert_eq!(quoted_word(r#""test""#), Ok(("", q("test"))));
        assert_eq!(quoted_word(r#""hello world""#), Ok(("", q("hello world"))));
        assert_eq!(
            quoted_word(r#""hello\nworld""#),
            Ok(("", q("hello\\nworld")))
        );
        assert_eq!(
            quoted_word(r#""Many\\escapes\n\"here\"""#),
            Ok(("", q(r#"Many\\escapes\n\"here\""#)))
        );
    }

    #[test]
    fn test_word_list() {
        assert_eq!(word_list("test"), Ok(("", vec![b("test")])));
        assert_eq!(word_list("test "), Ok((" ", vec![b("test")])));
        assert_eq!(
            word_list("hello world"),
            Ok(("", vec![b("hello"), b("world")]))
        );
        assert_eq!(
            word_list("hello\tworld"),
            Ok(("", vec![b("hello"), b("world")]))
        );
        assert_eq!(
            word_list("hello  \tworld"),
            Ok(("", vec![b("hello"), b("world")]))
        );
        assert_eq!(
            word_list("hello { world }"),
            Ok((" { world }", vec![b("hello")]))
        );
        assert_eq!(word_list("hello\nworld"), Ok(("\nworld", vec![b("hello")])));
        assert_eq!(word_list("").is_err(), true);
    }

    #[test]
    fn test_parse_no_variable() {
        assert_eq!(
            quoted_word(r#""Just some text""#),
            Ok(("", Word::Quoted(vec![Text::Text("Just some text")])))
        );
    }

    #[test]
    fn test_parse_inline_variable() {
        assert_eq!(
            word("Some$thing"),
            Ok((
                "",
                Word::Bare(vec![Text::Text("Some"), Text::Variable("thing")])
            ))
        );
    }

    #[test]
    fn test_parse_bracketed_variable() {
        assert_eq!(
            word("${a complicated variable name!}text"),
            Ok((
                "",
                Word::Bare(vec![
                    Text::Variable("a complicated variable name!"),
                    Text::Text("text"),
                ])
            ))
        );
    }

    #[test]
    fn test_parse_quoted_inline_variable() {
        assert_eq!(
            quoted_word(r#""Just some $thing""#),
            Ok((
                "",
                Word::Quoted(vec![Text::Text("Just some "), Text::Variable("thing")])
            ))
        );
    }

    #[test]
    fn test_parse_quoted_bracketed_variable() {
        assert_eq!(
            quoted_word(r#""Just ${a complicated variable name!} text""#),
            Ok((
                "",
                Word::Quoted(vec![
                    Text::Text("Just "),
                    Text::Variable("a complicated variable name!"),
                    Text::Text(" text")
                ])
            ))
        );
    }

    #[test]
    fn test_group() {
        assert_eq!(group("{ world }"), Ok(("", vec![b("world")])));
        assert_eq!(group("{world}"), Ok(("", vec![b("world")])));
        assert_eq!(
            group("{ hello\nworld }"),
            Ok(("", vec![b("hello"), b("world")]))
        );
        assert_eq!(
            group("{\n  hello\n  world\n}"),
            Ok(("", vec![b("hello"), b("world")]))
        );
        assert_eq!(group("{ world").is_err(), true);
    }

    #[test]
    fn test_subst() {
        assert_eq!(
            subst("[ + 1 2 ]"),
            Ok(("", Command(vec![b("+"), b("1"), b("2")])))
        );
        assert_eq!(
            subst("[ + 1 [ - 4 2 ] ]"),
            Ok((
                "",
                Command(vec![
                    b("+"),
                    b("1"),
                    Word::Subst(Command(vec![b("-"), b("4"), b("2")]))
                ])
            ))
        );
        assert_eq!(
            subst(r#"[ replace $version \..* "" ]"#),
            Ok((
                "",
                Command(vec![b("replace"), v("version"), b(r#"\..*"#), q("")])
            ))
        );
    }

    #[test]
    fn test_command() {
        assert_eq!(
            command("hello { world }"),
            Ok(("", Command(vec![b("hello"), b("world")])))
        );
        assert_eq!(
            command("hello \"{[ world ]}\""),
            Ok(("", Command(vec![b("hello"), q("{[ world ]}")])))
        );
        assert_eq!(
            command("puts \"Hello, world\""),
            Ok(("", Command(vec![b("puts"), q("Hello, world")])))
        );
        assert_eq!(
            command("demo {\n  hello\n  world\n}"),
            Ok(("", Command(vec![b("demo"), b("hello"), b("world")])))
        );
        assert_eq!(
            command("demo {\n  hello world\n}"),
            Ok(("", Command(vec![b("demo"), b("hello"), b("world")])))
        );
        assert_eq!(
            command("hello { world }\ndemo {\n  hello\n  world\n}\n"),
            Ok((
                "\ndemo {\n  hello\n  world\n}\n",
                Command(vec![b("hello"), b("world")])
            ))
        );
        assert_eq!(
            command(r#""hello { brackets }""#),
            Ok(("", Command(vec![q("hello { brackets }")])))
        );
        assert_eq!(
            command("puts ${example}"),
            Ok(("", Command(vec![b("puts"), v("example")])))
        );
    }

    #[test]
    fn test_command_subst() {
        assert_eq!(
            command(r#"set subdir [ replace $version \..* "" ]"#),
            Ok((
                "",
                Command(vec![
                    b("set"),
                    b("subdir"),
                    Word::Subst(Command(vec![
                        b("replace"),
                        v("version"),
                        b(r#"\..*"#),
                        q("")
                    ]))
                ])
            ))
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse(""), Ok(Vec::new()));
        assert_eq!(parse("  "), Ok(Vec::new()));
        assert_eq!(parse("\n\n\n"), Ok(Vec::new()));
    }

    #[test]
    fn test_parse_single() {
        assert_eq!(
            parse("hello { world }"),
            Ok(vec![Command(vec![b("hello"), b("world")])])
        );
    }

    #[test]
    fn test_parse_multiple() {
        assert_eq!(
            parse("hello { world }\ndemo {\n  hello\n  world\n}\n"),
            Ok(vec![
                Command(vec![b("hello"), b("world")]),
                Command(vec![b("demo"), b("hello"), b("world")])
            ])
        );
    }

    #[test]
    fn test_long_command() {
        let input = include_str!("../tests/pkg.tcl");
        let result = parse(input);
        assert_eq!(
            result,
            Ok(vec![
                Command(vec![b("set"), b("name"), b("ruby")]),
                Command(vec![b("set"), b("version"), b("2.6.3")]),
                Command(vec![b("set"), b("ruby_abiver"), b("2.6.0")]),
                Command(vec![
                    b("set"),
                    b("subdir"),
                    Word::Subst(Command(vec![
                        b("replace"),
                        v("version"),
                        b(r#"\..*"#),
                        q("")
                    ]))
                ]),
                Command(vec![b("pkgname"), v("name")]),
                Command(vec![b("version"), v("version")]),
                Command(vec![b("revision"), b("2")]),
                Command(vec![b("build-style"), b("gnu-configure")]),
                Command(vec![
                    b("configure_args"),
                    b("--enable-shared"),
                    b("--disable-rpath"),
                    b("DOXYGEN=/usr/bin/doxygen"),
                    b("DOT=/usr/bin/dot"),
                    b("PKG_CONFIG=/usr/bin/pkg-config")
                ]),
                Command(vec![b("make_build_args"), b("all"), b("capi")]),
                Command(vec![
                    b("hostmakedepends"),
                    b("pkg-config"),
                    b("bison"),
                    b("groff")
                ]),
                Command(vec![
                    b("makedepends"),
                    b("zlib-devel"),
                    b("readline-devel"),
                    b("libffi-devel"),
                    b("libressl-devel"),
                    b("gdbm-devel"),
                    b("libyaml-devel"),
                    b("pango-devel")
                ]),
                Command(vec![b("checkdepends"), b("tzdata")]),
                Command(vec![b("short_desc"), q("Ruby programming language")]),
                Command(vec![b("homepage"), b("http://www.ruby-lang.org/en/")]),
                Command(vec![b("maintainer"), q("Wesley Moore <wes@wezm.net>")]),
                Command(vec![b("license"), b("Ruby"), b("BSD-2-Clause")]),
                Command(vec![
                    b("distfile"),
                    Word::Bare(vec![
                        Text::Text("https://cache.ruby-lang.org/pub/ruby/"),
                        Text::Variable("subdir"),
                        Text::Text("/"),
                        Text::Variable("pkgname"),
                        Text::Text("-"),
                        Text::Variable("version"),
                        Text::Text(".tar.bz2")
                    ]),
                    b("checksum"),
                    b("dd638bf42059182c1d04af0d5577131d4ce70b79105231c4cc0a60de77b14f2e")
                ])
            ])
        )
    }

    // #[test]
    // fn test_bench() {
    //     use std::path::Path;
    //     use std::fs;

    //     let manifest_path = env!("CARGO_MANIFEST_DIR");
    //     let script_path = Path::new(&manifest_path);
    //     let script_path = script_path.join("tests/pkg.tcl");
    //     let script = fs::read_to_string(&script_path).expect("Error reading input file");

    //     for _ in 0..100_000 {
    //         assert!(parse(&script).is_ok())
    //     }
    // }
}
