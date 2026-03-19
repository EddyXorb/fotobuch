//! Lexer: tokenize a raw page-command string into [`Token`]s.

use super::tokens::{ParseError, Token};

/// Tokenize a raw string into a list of [`Token`]s.
///
/// Whitespace is ignored. `->` and `<>` are recognised as two-character tokens.
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
            '-' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Arrow);
                    i += 2;
                } else {
                    return Err(ParseError::UnexpectedChar('-'));
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Swap);
                    i += 2;
                } else {
                    return Err(ParseError::UnexpectedChar('<'));
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
            c => {
                return Err(ParseError::UnexpectedChar(c));
            }
        }
    }

    Ok(tokens)
}
