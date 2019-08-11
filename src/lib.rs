mod parser;

pub struct Command<'a> {
    words: Vec<&'a str>,
}
