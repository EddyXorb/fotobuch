//! Token types and parse-error types for the page CLI parser.

// ── Tokens ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(u32),
    Comma,
    Range, // ".."
    Colon, // ":"
    Arrow, // "->"
    Swap,  // "<>"
    Plus,  // "+"
}

// ── ParseError ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken { got: String, expected: &'static str },
    MissingOperator,
    MissingDestination,
    InvalidNumber(String),
    UnexpectedEnd { expected: &'static str },
    UnexpectedChar(char),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedToken { got, expected } => {
                write!(f, "unexpected '{got}', expected {expected}")
            }
            Self::MissingOperator => write!(f, "missing operator ('->' or '<>')"),
            Self::MissingDestination => write!(f, "missing destination after operator"),
            Self::InvalidNumber(s) => write!(f, "invalid number: '{s}'"),
            Self::UnexpectedEnd { expected } => {
                write!(f, "unexpected end of input, expected {expected}")
            }
            Self::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
        }
    }
}
