use std::collections::HashMap;
use logos::Logos;
use ratatui_core::style::Style;
use crate::code_logos::LogosCodeLanguage;

pub fn python_language<'a>(theme: HashMap<PythonLangToken, Style>) -> LogosCodeLanguage<'a, PythonLangToken> {
    LogosCodeLanguage::new("  ", "#", theme)
}

/// Logos-based lexer token enum for Python 3.13+
///
/// Usage:
///   let mut lex = Token::lexer("x = 1 + 2");
///   for tok in lex { ... }
///
/// Notes:
///   - INDENT/DEDENT are context-sensitive; drive an indent-tracking pass over
///     the `Newline` tokens emitted here.
///   - f-string interiors require stateful/recursive handling beyond what a
///     single Logos regex can express; `FStringStart` is emitted so you can
///     hand off to a sub-lexer.
///   - Comments are emitted as `Comment` (slice includes the leading `#`).
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\f]+")] // skip horizontal whitespace only (newlines matter)
pub enum PythonLangToken {

    // -------------------------------------------------------------------------
    // Comments
    // -------------------------------------------------------------------------

    /// A `#`-comment running to end of line (newline not included).
    #[regex(r"#[^\r\n]*", allow_greedy=true)]
    Comment,

    // -------------------------------------------------------------------------
    // Literals
    // -------------------------------------------------------------------------

    /// Integer literals: decimal, hex, octal, binary (with optional underscores)
    /// e.g. 0, 42, 1_000_000, 0xFF, 0o77, 0b1010
    #[regex(r"0[xX][0-9a-fA-F][0-9a-fA-F_]*|0[oO][0-7][0-7_]*|0[bB][01][01_]*|0+|[1-9][0-9_]*")]
    Int,

    /// Floating-point literals, including exponent notation and underscore separators.
    /// e.g. 3.14, 1_000.5e-3, .5, 1.
    ///
    /// Alternatives (order matters for greedy matching):
    ///   1. digit+ . digit+  e  …   (e.g. 1.5e3)
    ///   2. digit+ .         e  …   (e.g. 1.e3)
    ///   3.        . digit+  e  …   (e.g. .5e3)
    ///   4. digit+           e  …   (e.g. 1e3)
    ///   5. digit+ . digit+          (e.g. 1.5)
    ///   6. digit+ .                 (e.g. 1.)
    ///   7.        . digit+          (e.g. .5)
    #[regex(
        r"[0-9][0-9_]*\.[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*\
        |[0-9][0-9_]*\.[eE][+-]?[0-9][0-9_]*\
        |\.[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*\
        |[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*\
        |[0-9][0-9_]*\.[0-9][0-9_]*\
        |[0-9][0-9_]*\.\
        |\.[0-9][0-9_]*"
    )]
    Float,

    /// Imaginary number literals, e.g. 3j, 4.5J, 1e2j
    #[regex(
        r"([0-9][0-9_]*\.[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*\
        |[0-9][0-9_]*\.[eE][+-]?[0-9][0-9_]*\
        |\.[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*\
        |[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*\
        |[0-9][0-9_]*\.[0-9][0-9_]*\
        |[0-9][0-9_]*\.\
        |\.[0-9][0-9_]*\
        |[0-9][0-9_]*)[jJ]"
    )]
    Imaginary,

    // -------------------------------------------------------------------------
    // String literals
    //
    // Triple-quoted variants are listed before single-quoted ones so Logos
    // prefers the longer match.
    //
    // Look-ahead-free triple-quote interior:
    //   ([^"\\] | \\. | "[^"\\] | ""[^"\\])*
    //
    // Each step is one of:
    //   • a non-quote, non-backslash character
    //   • a backslash escape (any char after \)
    //   • exactly one " followed by a non-" (can't be the start of closing """)
    //   • exactly two " followed by a non-" (same reason)
    // This never accidentally consumes the closing """ because that would
    // require three consecutive unescaped quotes, which none of the above
    // alternatives produce.  Analogous logic applies for '''.
    // -------------------------------------------------------------------------

    /// Triple-quoted plain string:     """..."""   '''...'''
    /// Triple-quoted b/u-prefixed:  b"""..."""  u'''...'''
    #[regex(r#"[bBuU]?"""([^"\\]|\\.|"[^"\\]|""[^"\\])*""""#)]
    #[regex(r#"[bBuU]?'''([^'\\]|\\.|'[^'\\]|''[^'\\])*'''"#)]
    TripleStringLiteral,

    /// Triple-quoted raw string:  r"""..."""  rb'''...'''  br"""..."""
    #[regex(r#"[rR][bB]?"""([^"\\]|\\.|"[^"\\]|""[^"\\])*""""#)]
    #[regex(r#"[rR][bB]?'''([^'\\]|\\.|'[^'\\]|''[^'\\])*'''"#)]
    #[regex(r#"[bB][rR]"""([^"\\]|\\.|"[^"\\]|""[^"\\])*""""#)]
    #[regex(r#"[bB][rR]'''([^'\\]|\\.|'[^'\\]|''[^'\\])*'''"#)]
    RawTripleStringLiteral,

    /// Single-quoted plain / b / u string:  "hello"  b'hi'  u"…"
    #[regex(r#"[bBuU]?"([^"\\]|\\.)*""#)]
    #[regex(r#"[bBuU]?'([^'\\]|\\.)*'"#)]
    StringLiteral,

    /// Single-quoted raw string:  r"…"  rb'…'  br"…"
    #[regex(r#"[rR][bB]?"([^"\\]|\\.)*""#)]
    #[regex(r#"[rR][bB]?'([^'\\]|\\.)*'"#)]
    #[regex(r#"[bB][rR]"([^"\\]|\\.)*""#)]
    #[regex(r#"[bB][rR]'([^'\\]|\\.)*'"#)]
    RawStringLiteral,

    /// f-string / rf-string opening quote (single or triple).
    /// Full interior parsing requires stateful lexing; emit this token and
    /// delegate to a sub-lexer for the `{…}` interpolation regions.
    #[regex(r#"[fF][rR]?"{1,3}"#)]
    #[regex(r#"[fF][rR]?'{1,3}"#)]
    #[regex(r#"[rR][fF]"{1,3}"#)]
    #[regex(r#"[rR][fF]'{1,3}"#)]
    FStringStart,

    // -------------------------------------------------------------------------
    // Keywords  (Python 3.13)
    // `#[token]` has higher priority than `#[regex]`, so keywords always win
    // over the Identifier regex below — no need for boundary assertions.
    // -------------------------------------------------------------------------

    #[token("False")]    KwFalse,
    #[token("None")]     KwNone,
    #[token("True")]     KwTrue,
    #[token("and")]      KwAnd,
    #[token("as")]       KwAs,
    #[token("assert")]   KwAssert,
    #[token("async")]    KwAsync,
    #[token("await")]    KwAwait,
    #[token("break")]    KwBreak,
    #[token("class")]    KwClass,
    #[token("continue")] KwContinue,
    #[token("def")]      KwDef,
    #[token("del")]      KwDel,
    #[token("elif")]     KwElif,
    #[token("else")]     KwElse,
    #[token("except")]   KwExcept,
    #[token("finally")]  KwFinally,
    #[token("for")]      KwFor,
    #[token("from")]     KwFrom,
    #[token("global")]   KwGlobal,
    #[token("if")]       KwIf,
    #[token("import")]   KwImport,
    #[token("in")]       KwIn,
    #[token("is")]       KwIs,
    #[token("lambda")]   KwLambda,
    #[token("match")]    KwMatch,    // soft keyword (context-dependent)
    #[token("case")]     KwCase,     // soft keyword
    #[token("type")]     KwType,     // soft keyword (PEP 695, Python 3.12+)
    #[token("nonlocal")] KwNonlocal,
    #[token("not")]      KwNot,
    #[token("or")]       KwOr,
    #[token("pass")]     KwPass,
    #[token("raise")]    KwRaise,
    #[token("return")]   KwReturn,
    #[token("try")]      KwTry,
    #[token("while")]    KwWhile,
    #[token("with")]     KwWith,
    #[token("yield")]    KwYield,

    // -------------------------------------------------------------------------
    // Identifiers
    // Listed after keywords; `#[token]` priority ensures keywords win.
    // -------------------------------------------------------------------------

    /// ASCII-only fast path.
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    /// Full Unicode identifier per PEP 3131 / UAX #31.
    // #[regex(r"[\p{XID_Start}_][\p{XID_Continue}]*")]
    Identifier,

    // -------------------------------------------------------------------------
    // Operators
    // Three-character operators must be listed before their two-character
    // prefixes, which must be listed before single-character ones.
    // Logos resolves ties by longest match, but explicit ordering is clearer.
    // -------------------------------------------------------------------------

    // Augmented assignment (3-char)
    #[token("**=")]  DoubleStarEqual,
    #[token("//=")]  SlashSlashEqual,
    #[token("<<=")]  LessLessEqual,
    #[token(">>=")]  GreaterGreaterEqual,

    // Augmented assignment (2-char)
    #[token("+=")]   PlusEqual,
    #[token("-=")]   MinusEqual,
    #[token("*=")]   StarEqual,
    #[token("/=")]   SlashEqual,
    #[token("%=")]   PercentEqual,
    #[token("&=")]   AmpersandEqual,
    #[token("|=")]   PipeEqual,
    #[token("^=")]   CaretEqual,
    #[token("@=")]   AtEqual,

    // Comparison (2-char)
    #[token("==")]   EqEqual,
    #[token("!=")]   NotEqual,
    #[token("<=")]   LessEqual,
    #[token(">=")]   GreaterEqual,

    // Other 2-char
    #[token("->")]   Arrow,
    #[token(":=")]   ColonEqual,   // walrus (PEP 572)
    #[token("**")]   DoubleStar,
    #[token("//")]   SlashSlash,
    #[token("<<")]   LessLess,
    #[token(">>")]   GreaterGreater,

    // Single-char operators
    #[token("<")]    Less,
    #[token(">")]    Greater,
    #[token("+")]    Plus,
    #[token("-")]    Minus,
    #[token("*")]    Star,
    #[token("/")]    Slash,
    #[token("%")]    Percent,
    #[token("~")]    Tilde,
    #[token("&")]    Ampersand,
    #[token("|")]    Pipe,
    #[token("^")]    Caret,
    #[token("@")]    At,
    #[token("=")]    Equal,

    // -------------------------------------------------------------------------
    // Delimiters & punctuation
    // -------------------------------------------------------------------------

    #[token("...")]  Ellipsis,   // must come before Dot
    #[token("(")]    LParen,
    #[token(")")]    RParen,
    #[token("[")]    LBracket,
    #[token("]")]    RBracket,
    #[token("{")]    LBrace,
    #[token("}")]    RBrace,
    #[token(",")]    Comma,
    #[token(":")]    Colon,
    #[token(".")]    Dot,
    #[token(";")]    Semi,

    // -------------------------------------------------------------------------
    // Whitespace-sensitive tokens
    // -------------------------------------------------------------------------

    /// Physical newline (CR+LF, CR, or LF).
    /// Logical-newline / INDENT / DEDENT handling must be done in a
    /// post-processing pass that tracks this token plus bracket depth.
    #[regex(r"\r\n|\r|\n")]
    Newline,

    /// Explicit line continuation: `\` immediately followed by a newline.
    /// Emit so the indent pass knows to suppress the following `Newline`.
    #[regex(r"\\(\r\n|\r|\n)")]
    LineContinuation,
}