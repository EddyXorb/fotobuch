//! Lexer: tokenize a raw page-command string into [`Token`]s.

use super::tokens::{ParseError, Token};

/// Tokenize a raw string into a list of [`Token`]s.
///
/// Whitespace is ignored. `~` is the swap operator. `to` and `out` are keywords.
pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => {
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '.' => {
                if i + 1 < chars.len() && chars[i + 1] == '.' {
                    tokens.push(Token::Range);
                    i += 2;
                } else {
                    return Err(ParseError::UnexpectedChar('.'));
                }
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n: u32 = s
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber(s.clone()))?;
                tokens.push(Token::Number(n));
            }
            c if c.is_ascii_alphabetic() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match word.as_str() {
                    "to" => tokens.push(Token::To),
                    "out" => tokens.push(Token::Out),
                    _ => return Err(ParseError::UnknownKeyword(word)),
                }
            }
            c => {
                return Err(ParseError::UnexpectedChar(c));
            }
        }
    }

    Ok(tokens)
}
