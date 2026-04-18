use crate::code::CodeLanguage;
use logos::Logos;
use ratatui_core::style::Style;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct LogosCodeLanguage<'a, Token: logos::Logos<'a>> {
    indent: &'a str,
    comment_prefix: &'a str,
    theme: HashMap<Token, Style>,
    token: PhantomData<Token>,
}

impl<'a, Token> LogosCodeLanguage<'a, Token>
where
    Token: logos::Logos<'a, Extras: Default, Source = str>,
{
    pub fn new(indent: &'a str, comment_prefix: &'a str, theme: HashMap<Token, Style>) -> Self {
        LogosCodeLanguage {
            indent,
            comment_prefix,
            theme,
            token: PhantomData,
        }
    }
}

impl<'a, Token> CodeLanguage for LogosCodeLanguage<'a, Token>
where
    Token: for<'s> logos::Logos<'s, Extras: Default, Source = str> + Eq + Hash,
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
        tokens
            .into_iter()
            .filter_map(|(token, span)| {
                let style = self.theme.get(&token);
                style.map(|style| (span, style.clone()))
            })
            .collect()
    }
}

pub fn plain_text_lang() -> Box<dyn CodeLanguage> {
    Box::new(LogosCodeLanguage {
        indent: "  ",
        comment_prefix: "//",
        theme: HashMap::new(),
        token: PhantomData::<PlainTextToken>,
    })
}

#[derive(Logos, Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum PlainTextToken {
    #[regex(".+", allow_greedy = true)]
    Any,
}
