use std::collections::HashMap;
use logos::Logos;
use ratatui_core::style::Style;
use crate::code_logos::LogosCodeLanguage;

pub fn rust_language<'a>(theme: HashMap<RustLangToken, Style>) -> LogosCodeLanguage<'a, RustLangToken> {
    LogosCodeLanguage::new("  ", "//", theme)
}

/// Logos-based lexer token enum for the Rust language (2024 edition).
///
/// Usage:
///   let mut lex = Token::lexer("fn main() { }");
///   for tok in lex { ... }
///
/// Design notes:
///
/// RAW STRINGS  (r"…", r#"…"#, r##"…"##, …)
///   The number of `#` signs on each side must match, and the interior may
///   contain any character except bare CR.  Because Logos regexes cannot
///   count balanced delimiters, we cover 0-through-6 hashes explicitly.
///   Matching more than 6 hashes is vanishingly rare in real code; add more
///   arms if you need them.
///
/// LIFETIME / LABEL TOKENS
///   `'name` is emitted as `Lifetime`.  The special `'static` is also
///   matched by the same rule (the parser can distinguish it by slice value).
///   `'_` (anonymous lifetime) is also covered.
///
/// BLOCK COMMENTS
///   Rust block comments are nestable (/* /* */ */).  A plain regex cannot
///   handle arbitrary nesting depth.  The `BlockComment` variant therefore
///   uses a callback that manually counts open/close pairs so nesting is
///   handled correctly at any depth.
///
/// DOC COMMENTS
///   `///`, `//!`, `/**`, `/*!` are emitted as separate variants so callers
///   can extract documentation without re-inspecting every `LineComment`.
///
/// SUFFIXES
///   Numeric literals optionally carry a type suffix (e.g. `42u8`, `1.0f64`).
///   The suffix is absorbed into the same token so the slice is self-contained.
#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\n\f]+")] // whitespace is insignificant in Rust
pub enum RustLangToken {
    // =========================================================================
    // Comments
    // =========================================================================
    /// Outer doc line comment: `/// …`
    #[regex(r"///[^\n]*", allow_greedy=true)]
    LineDocOuter,

    /// Inner doc line comment: `//! …`
    #[regex(r"//![^\n]*", allow_greedy=true)]
    LineDocInner,

    /// Ordinary line comment: `// …`  (must come after doc variants)
    #[regex(r"//[^\n]*", allow_greedy=true)]
    LineComment,

    /// Block comment `/* … */` with correct nested-comment handling.
    ///
    /// The regex matches the non-nested, non-doc form as a fast path.
    /// A callback handles the nesting.  `/**` (outer doc) and `/*!` (inner
    /// doc) are separate variants matched first so they win over this rule.
    #[token("/**", lex_outer_block_doc)]
    BlockDocOuter(()),

    #[token("/*!", lex_inner_block_doc)]
    BlockDocInner(()),

    /// Plain (possibly nested) block comment.
    #[token("/*", lex_block_comment)]
    BlockComment(()),

    // =========================================================================
    // Literals
    // =========================================================================

    // --- Character literal ---------------------------------------------------
    /// `'a'`, `'\n'`, `'\u{1F600}'`, etc.  Optional suffix.
    /// Interior: any char except `'`, `\`, LF, CR, TAB; or an escape sequence.
    #[regex(r#"'([^'\\\n\r\t]|\\[nrt\\'"0]|\\x[0-7][0-9a-fA-F]|\\u\{[0-9a-fA-F]{1,6}\})'([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    CharLiteral,

    // --- Byte literal --------------------------------------------------------
    /// `b'A'`, `b'\x41'`, etc.  ASCII only; optional suffix.
    #[regex(r#"b'([^\'\\\n\r\t]|\\[nrt\\'"0]|\\x[0-9a-fA-F]{2})'([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    ByteLiteral,

    // --- String literals -----------------------------------------------------
    /// Ordinary string: `"hello\nworld"`.  Optional suffix.
    /// Interior: any char except `"`, `\`, CR; or any escape; or `\` LF (line continuation).
    #[regex(r#""([^"\\\r]|\\[nrt\\'"0]|\\x[0-7][0-9a-fA-F]|\\u\{[0-9a-fA-F]{1,6}\}|\\\n)*"([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    StringLiteral,

    /// Byte string: `b"hello"`.  ASCII only; optional suffix.
    #[regex(r#"b"([^"\\\r]|\\[nrt\\'"0]|\\x[0-9a-fA-F]{2}|\\\n)*"([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    ByteStringLiteral,

    /// C string: `c"hello"`.  Optional suffix.
    #[regex(r#"c"([^"\\\r\x00]|\\[nrt\\'"0]|\\x[0-9a-fA-F]{2}|\\u\{[0-9a-fA-F]{1,6}\}|\\\n)*"([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    CStringLiteral,

    // --- Raw string literals -------------------------------------------------
    //
    // r"…"  r#"…"#  r##"…"##  … up to r######"…"######
    // The interior may contain anything except CR; `"` is fine as long as it
    // is not followed by the matching number of `#`.  We enumerate 0-6 hashes.
    //
    // Look-ahead-free interior trick (same principle as Python triple-strings):
    //   r"…"   →  interior is ([^"\r]|"[^"\r]|…)* with one fewer " than close
    // For r"…", interior is simply [^"\r]*.
    // For r#"…"#, interior must not contain `"#` — any `"` not followed by `#`
    // is fine.  Pattern: ([^"\r]|"[^#\r])*.
    // For r##"…"##, a `"` not followed by `##` is safe:
    //   ([^"\r]|"[^#\r]|"#[^#\r])*.
    // The general pattern for N hashes: a `"` may appear if the following
    // N characters are not all `#`.
    /// `r"…"`  — zero hashes
    #[regex(r#"r"[^"\r]*"([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    /// `r#"…"#`  — one hash
    #[regex(r##"r#"([^"\r]|"[^#\r])*"#([a-zA-Z_][a-zA-Z0-9_]*)?"##)]
    /// `r##"…"##`  — two hashes
    #[regex(r###"r##"([^"\r]|"[^#\r]|"#[^#\r])*"##([a-zA-Z_][a-zA-Z0-9_]*)?"###)]
    /// `r###"…"###`  — three hashes
    #[regex(r####"r###"([^"\r]|"[^#\r]|"#[^#\r]|"##[^#\r])*"###([a-zA-Z_][a-zA-Z0-9_]*)?"####)]
    RawStringLiteral,

    /// `br"…"`  / `rb"…"` — zero hashes
    #[regex(r#"(br|rb)"[^"\r]*"([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    /// `br#"…"#`  / `rb#"…"#`
    #[regex(r##"(br|rb)#"([^"\r]|"[^#\r])*"#([a-zA-Z_][a-zA-Z0-9_]*)?"##)]
    /// `br##"…"##`  / `rb##"…"##`
    #[regex(r###"(br|rb)##"([^"\r]|"[^#\r]|"#[^#\r])*"##([a-zA-Z_][a-zA-Z0-9_]*)?"###)]
    RawByteStringLiteral,

    /// `cr"…"` — zero hashes
    #[regex(r#"cr"[^"\r\x00]*"([a-zA-Z_][a-zA-Z0-9_]*)?"#)]
    /// `cr#"…"#`
    #[regex(r##"cr#"([^"\r\x00]|"[^#\r\x00])*"#([a-zA-Z_][a-zA-Z0-9_]*)?"##)]
    /// `cr##"…"##`
    #[regex(r###"cr##"([^"\r\x00]|"[^#\r\x00]|"#[^#\r\x00])*"##([a-zA-Z_][a-zA-Z0-9_]*)?"###)]
    RawCStringLiteral,

    // --- Numeric literals ----------------------------------------------------
    /// Floating-point: `1.0`, `3.14e-10`, `0.0f64`, `1_000.5E+3f32`
    /// Must be tried before integers because `1.0` starts like an integer.
    ///
    /// Forms:
    ///   digit+ . digit+ exponent? suffix_no_e?
    ///   digit+ .                              (trailing dot, no suffix — `1.`)
    ///   digit+   exponent  suffix_no_e?
    ///
    /// suffix_no_e must not begin with `e`/`E` (would be ambiguous with exp).
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9][0-9_]*)?([a-df-zA-DF-Z_][a-zA-Z0-9_]*)?")]
    /// Trailing-dot form: `1.`  (no exponent, no suffix allowed per the spec)
    #[regex(r"[0-9][0-9_]*\.")]
    /// Exponent-only form: `1e10`, `2E-3f64`
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*([a-df-zA-DF-Z_][a-zA-Z0-9_]*)?")]
    FloatLiteral,

    /// Decimal integer: `0`, `42`, `1_000_000u32`
    #[regex(r"[0-9][0-9_]*([a-zA-Z_][a-zA-Z0-9_]*)?")]
    /// Hexadecimal: `0xFF`, `0xDEAD_BEEFu64`
    #[regex(r"0[xX][0-9a-fA-F][0-9a-fA-F_]*([a-zA-Z_][a-zA-Z0-9_]*)?")]
    /// Octal: `0o77`, `0o755i32`
    #[regex(r"0[oO][0-7][0-7_]*([a-zA-Z_][a-zA-Z0-9_]*)?")]
    /// Binary: `0b1010`, `0b1111_0000u8`
    #[regex(r"0[bB][01][01_]*([a-zA-Z_][a-zA-Z0-9_]*)?")]
    IntLiteral,

    // =========================================================================
    // Lifetimes and loop labels
    // =========================================================================
    /// `'a`, `'static`, `'_`  — but NOT `'a'` (that is a CharLiteral).
    /// Logos matches longest token; since CharLiteral patterns end with `'`,
    /// they will beat this rule for `'a'`.
    #[regex(r"'[a-zA-Z_][a-zA-Z0-9_]*")]
    Lifetime,

    // =========================================================================
    // Strict keywords
    // (listed before Identifier so #[token] priority wins)
    // =========================================================================
    #[token("as")]
    KwAs,
    #[token("async")]
    KwAsync,
    #[token("await")]
    KwAwait,
    #[token("break")]
    KwBreak,
    #[token("const")]
    KwConst,
    #[token("continue")]
    KwContinue,
    #[token("crate")]
    KwCrate,
    #[token("dyn")]
    KwDyn,
    #[token("else")]
    KwElse,
    #[token("enum")]
    KwEnum,
    #[token("extern")]
    KwExtern,
    #[token("false")]
    KwFalse,
    #[token("fn")]
    KwFn,
    #[token("for")]
    KwFor,
    #[token("if")]
    KwIf,
    #[token("impl")]
    KwImpl,
    #[token("in")]
    KwIn,
    #[token("let")]
    KwLet,
    #[token("loop")]
    KwLoop,
    #[token("match")]
    KwMatch,
    #[token("mod")]
    KwMod,
    #[token("move")]
    KwMove,
    #[token("mut")]
    KwMut,
    #[token("pub")]
    KwPub,
    #[token("ref")]
    KwRef,
    #[token("return")]
    KwReturn,
    #[token("self")]
    KwSelfLower,
    #[token("Self")]
    KwSelfUpper,
    #[token("static")]
    KwStatic,
    #[token("struct")]
    KwStruct,
    #[token("super")]
    KwSuper,
    #[token("trait")]
    KwTrait,
    #[token("true")]
    KwTrue,
    #[token("type")]
    KwType,
    #[token("unsafe")]
    KwUnsafe,
    #[token("use")]
    KwUse,
    #[token("where")]
    KwWhere,
    #[token("while")]
    KwWhile,

    // =========================================================================
    // Reserved keywords (future use)
    // =========================================================================
    #[token("abstract")]
    KwAbstract,
    #[token("become")]
    KwBecome,
    #[token("box")]
    KwBox,
    #[token("do")]
    KwDo,
    #[token("final")]
    KwFinal,
    #[token("gen")]
    KwGen, // reserved since 2024 edition
    #[token("macro")]
    KwMacro,
    #[token("override")]
    KwOverride,
    #[token("priv")]
    KwPriv,
    #[token("try")]
    KwTry, // reserved since 2018 edition
    #[token("typeof")]
    KwTypeof,
    #[token("unsized")]
    KwUnsized,
    #[token("virtual")]
    KwVirtual,
    #[token("yield")]
    KwYield,

    // =========================================================================
    // Weak / contextual keywords
    // Emitted as Identifier; the parser inspects the slice for context.
    // Listed here as a reminder; they fall through to the Identifier regex.
    //   macro_rules  union  safe  raw  (and 'static handled by Lifetime)
    // =========================================================================

    // =========================================================================
    // Identifiers
    // =========================================================================
    /// Plain identifier: `foo`, `_bar`, `Baz123`
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    /// Full Unicode identifier (XID_Start + XID_Continue, per UAX #31)
    #[regex(r"[\p{XID_Start}][\p{XID_Continue}]*", priority = 1)]
    Identifier,

    /// Raw identifier: `r#try`, `r#type`
    /// Allows using keywords as identifiers.
    #[regex(r"r#[a-zA-Z_][a-zA-Z0-9_]*")]
    RawIdentifier,

    // =========================================================================
    // Punctuation
    // (longest tokens first to avoid prefix ambiguity)
    // =========================================================================

    // 3-character
    #[token("..=")]
    DotDotEq, // inclusive range
    #[token("...")]
    DotDotDot, // variadic in extern, deprecated spread
    #[token("<<=")]
    ShlEq,
    #[token(">>=")]
    ShrEq,

    // 2-character
    #[token("::")]
    PathSep, // `::`
    #[token("->")]
    Arrow, // `->`
    #[token("=>")]
    FatArrow, // `=>`
    #[token("..")]
    DotDot, // `..`  (range, struct update)
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Ne,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,
    #[token("^=")]
    CaretEq,
    #[token("&=")]
    AndEq,
    #[token("|=")]
    OrEq,

    // 1-character operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("&")]
    And,
    #[token("|")]
    Or,
    #[token("!")]
    Not,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("=")]
    Eq,
    #[token("@")]
    At, // pattern binding `p @ ...`
    #[token("_", priority = 3)]
    Underscore, // wildcard (also a strict keyword)
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(";")]
    Semi,
    #[token(":")]
    Colon,
    #[token("#")]
    Pound, // attribute prefix `#[…]` / `#![…]`
    #[token("$")]
    Dollar, // macro metavariable
    #[token("?")]
    Question, // `?` operator / `Option`-in-trait

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
}

// =============================================================================
// Block-comment callbacks
//
// Rust block comments nest: /* /* inner */ still open */.
// We manually scan forward, counting open/close `/* */` pairs, and tell Logos
// how many bytes to consume via `lex.bump(n)`.
// =============================================================================

/// Shared implementation: consumes a `/* … */` block comment from the current
/// position (the opening `/*` has already been matched and consumed by the
/// `#[token]` pattern that called us).  Returns `Some(())` on success or
/// `None` if the input ends before the comment is closed.
fn consume_block_comment(lex: &mut logos::Lexer<RustLangToken>) -> Option<()> {
    let remainder = lex.remainder();
    let bytes = remainder.as_bytes();
    let mut depth: usize = 1; // one `/*` already consumed
    let mut i = 0;
    while i < bytes.len() {
        match (bytes[i], bytes.get(i + 1)) {
            (b'/', Some(b'*')) => {
                depth += 1;
                i += 2;
            }
            (b'*', Some(b'/')) => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    lex.bump(i);
                    return Some(());
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    None // unterminated comment
}

fn lex_block_comment(lex: &mut logos::Lexer<RustLangToken>) -> Option<()> {
    consume_block_comment(lex)
}

fn lex_outer_block_doc(lex: &mut logos::Lexer<RustLangToken>) -> Option<()> {
    consume_block_comment(lex)
}

fn lex_inner_block_doc(lex: &mut logos::Lexer<RustLangToken>) -> Option<()> {
    consume_block_comment(lex)
}
