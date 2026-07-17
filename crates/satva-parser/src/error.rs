use std::fmt;

use crate::lexer::{Position, Token};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub position: Option<Position>,
}

impl ParseError {
    pub fn expected(expected: &Token, found: &Token, position: Option<Position>) -> Self {
        Self {
            message: format!("Expected {expected}, found {found}"),
            position,
        }
    }

    pub fn unexpected(found: &Token, expected: &str, position: Option<Position>) -> Self {
        Self {
            message: format!("Unexpected {found}, expected {expected}"),
            position,
        }
    }

    pub fn lexical(msg: impl Into<String>, position: Option<Position>) -> Self {
        Self {
            message: msg.into(),
            position,
        }
    }

    pub fn unknown_function(name: impl Into<String>, position: Option<Position>) -> Self {
        Self {
            message: format!("Unknown function '{}'", name.into()),
            position,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.position {
            Some(pos) => write!(f, "{}: {}", pos, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}
