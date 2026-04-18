use logos::Logos;
use ratatui_core::style::Style;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Range;
use std::sync::LazyLock;

#[derive(Clone, Debug)]
pub struct LogosCodeLanguage<'a, Token: logos::Logos<'a>> {
    indent: &'a str,
    comment_prefix: &'a str,
    theme: HashMap<Token, Style>,
    token: PhantomData<Token>,
}

impl<'a, Token: logos::Logos<'a, Extras: Default, Source = str>> LogosCodeLanguage<'a, Token> {
    pub fn new(indent: &'a str, comment_prefix: &'a str, theme: HashMap<Token, Style>) -> Self {
        LogosCodeLanguage {
            indent,
            comment_prefix,
            theme,
            token: PhantomData,
        }
    }
}

impl<'a, Token: logos::Logos<'a, Extras: Default, Source = str>> crate::code::CodeLanguage
    for LogosCodeLanguage<'a, Token>
{
    fn get_indent(&self) -> &'a str {
        self.indent
    }

    fn get_comment_prefix(&self) -> &'a str {
        self.comment_prefix
    }

    fn highlight(&self, text: &str) -> Vec<(Range<usize>, Style)> {
        let tokens: Vec<_> = Token::lexer(text)
            .spanned()
            .filter_map(|(token, span)| token.map(|token| (token, span)).ok())
            .collect();

        let mut results = Vec::new();
        for (token, span) in tokens {
            // TODO
        }

        results
    }
}

pub static PLAIN_TEXT: LazyLock<LogosCodeLanguage<PlainTextToken>> =
    LazyLock::new(|| LogosCodeLanguage {
        indent: "  ",
        comment_prefix: "//",
        theme: HashMap::new(),
        token: PhantomData,
    });

#[derive(Logos, Clone, Debug)]
pub(crate) enum PlainTextToken {
    #[regex(".+", allow_greedy = true)]
    Any,
}
