use nom::branch::alt;
use nom::bytes::complete::{escaped, tag, take_while, take_while1};
use nom::character::complete::{char as chr, newline, one_of};
use nom::combinator::{all_consuming, map};
use nom::multi::{fold_many1, many0, many1};
use nom::sequence::{delimited, preceded, terminated};
use nom::IResult;

// Commands are separated by newlines or semicolons
// New lines are ignored when inside a { } group
// When evaluating commands inside [ ] are substituted into the outer command
// $var or ${var} is substituted with the value of the variable var
// Double quotes can be used to ignore special characters like space
// Each command evaluates to a single result value

//pub type Word<'a> = &'a str;
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Word<'a> {
    Bare(&'a str),
    Quoted(&'a str),
}

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
    c == '\n' || c == ';'
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

fn word(input: &str) -> IResult<&str, Word<'_>> {
    map(take_while1(is_word), Word::Bare)(input)
}

//fn decode_escapes(escape: &str) -> IResult<&str, &str> {
//    alt((
//    map(tag("\\"),  |_| "\\" ),
//       map(tag("\"") ,  |_| "\"" ),
//       map(tag("n"), |_| "\n" )
//    ))(escape)
//}

fn escaped_text(input: &str) -> IResult<&str, &str> {
    let allowed = take_while1(|c| c != '\\' && c != '"');
    escaped(allowed, '\\', one_of(r#"\"n"#))(input)
}

fn quoted_word(input: &str) -> IResult<&str, Word<'_>> {
    map(
        alt((
            delimited(chr('"'), escaped_text, chr('"')),
            map(tag("\"\""), |_| ""),
        )),
        Word::Quoted,
    )(input)
}

fn word_list(input: &str) -> IResult<&str, Vec<Word<'_>>> {
    many1(preceded(sep, word_or_quoted))(input)
}

// Inside a group ; and \n are allowed
fn is_grouped_word(c: char) -> bool {
    // not(one_of("{}\"[]\\ \t"))
    c != '{' && c != '}' && c != '[' && c != ']' && c != '"' && !is_space(c)
}

fn word_or_quoted(input: &str) -> IResult<&str, Word<'_>> {
    alt((word, quoted_word))(input)
}

fn group(input: &str) -> IResult<&str, Vec<Word<'_>>> {
    preceded(
        chr('{'),
        terminated(
            many0(preceded(ws, word_or_quoted)),
            preceded(ws, chr('}')),
        ),
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

pub fn parse(input: &str) -> IResult<&str, Vec<Command<'_>>> {
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

    // Word::Quoted from str
    fn q(s: &str) -> Word<'_> {
        Word::Quoted(s)
    }

    // Word::Bare from str
    fn b(s: &str) -> Word<'_> {
        Word::Bare(s)
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
            Ok(("", Command(vec![Token::List(vec![b("+"), b("1"), b("2")])])))
        );
        assert_eq!(
            subst("[ + 1 [ - 4 2 ] ]"),
            Ok((
                "",
                Command(vec![
                    Token::List(vec![b("+"), b("1")]),
                    Token::Subst(Command(vec![Token::List(vec![b("-"), b("4"), b("2")])]))
                ])
            ))
        );
        assert_eq!(
            subst(r#"[ replace $version \..* "" ]"#),
            Ok((
                "",
                Command(vec![Token::List(vec![
                    b("replace"),
                    b("$version"),
                    b(r#"\..*"#),
                    q("")
                ])])
            ))
        );
    }

    #[test]
    fn test_command() {
        assert_eq!(
            command("hello { world }"),
            Ok(("", Command(vec![Token::List(vec![b("hello"), b("world")])])))
        );
        assert_eq!(
            command("hello \"{[ world ]}\""),
            Ok((
                "",
                Command(vec![Token::List(vec![b("hello"), q("{[ world ]}")])])
            ))
        );
        assert_eq!(
            command("puts \"Hello, world\""),
            Ok((
                "",
                Command(vec![Token::List(vec![b("puts"), q("Hello, world")])])
            ))
        );
        assert_eq!(
            command("demo {\n  hello\n  world\n}"),
            Ok((
                "",
                Command(vec![Token::List(vec![b("demo"), b("hello"), b("world")])])
            ))
        );
        assert_eq!(
            command("hello { world }\ndemo {\n  hello\n  world\n}\n"),
            Ok((
                "\ndemo {\n  hello\n  world\n}\n",
                Command(vec![Token::List(vec![b("hello"), b("world")])])
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
                    Token::List(vec![b("set"), b("subdir")]),
                    Token::Subst(Command(vec![Token::List(vec![
                        b("replace"),
                        b("$version"),
                        b(r#"\..*"#),
                        q("")
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
                vec![Command(vec![Token::List(vec![b("hello"), b("world")])]),]
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
                    Command(vec![Token::List(vec![b("hello"), b("world")])]),
                    Command(vec![Token::List(vec![b("demo"), b("hello"), b("world")])])
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
                    Command(vec![Token::List(vec![b("set"), b("name"), b("ruby")])]),
                    Command(vec![Token::List(vec![b("set"), b("version"), b("2.6.3")])]),
                    Command(vec![Token::List(vec![
                        b("set"),
                        b("ruby_abiver"),
                        b("2.6.0")
                    ])]),
                    Command(vec![
                        Token::List(vec![b("set"), b("subdir")]),
                        Token::Subst(Command(vec![Token::List(vec![
                            b("replace"),
                            b("$version"),
                            b(r#"\..*"#),
                            q("")
                        ])]))
                    ]),
                    Command(vec![Token::List(vec![b("pkgname"), b("$name")])]),
                    Command(vec![Token::List(vec![b("version"), b("$version")])]),
                    Command(vec![Token::List(vec![b("revision"), b("2")])]),
                    Command(vec![Token::List(vec![
                        b("build-style"),
                        b("gnu-configure")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("configure_args"),
                        b("--enable-shared"),
                        b("--disable-rpath"),
                        b("DOXYGEN"),
                        b("/usr/bin/doxygen"),
                        b("DOT"),
                        b("/usr/bin/dot"),
                        b("PKG_CONFIG"),
                        b("/usr/bin/pkg-config")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("make_build_args"),
                        b("all"),
                        b("capi")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("hostmakedepends"),
                        b("pkg-config"),
                        b("bison"),
                        b("groff")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("makedepends"),
                        b("zlib-devel"),
                        b("readline-devel"),
                        b("libffi-devel"),
                        b("libressl-devel"),
                        b("gdbm-devel"),
                        b("libyaml-devel"),
                        b("pango-devel")
                    ])]),
                    Command(vec![Token::List(vec![b("checkdepends"), b("tzdata")])]),
                    Command(vec![Token::List(vec![
                        b("short_desc"),
                        q("Ruby programming language")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("homepage"),
                        b("http://www.ruby-lang.org/en/")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("maintainer"),
                        q("Wesley Moore <wes@wezm.net>")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("license"),
                        b("Ruby"),
                        b("BSD-2-Clause")
                    ])]),
                    Command(vec![Token::List(vec![
                        b("distfile"),
                        b("https://cache.ruby-lang.org/pub/ruby/$subdir/$pkgname-$version.tar.bz2"),
                        b("checksum"),
                        b("dd638bf42059182c1d04af0d5577131d4ce70b79105231c4cc0a60de77b14f2e")
                    ])])
                ]
            ))
        )
    }
}
