use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Int(i64),
    Float(f64),
    String(String),
    True,
    False,
    Null,

    Ident(String),

    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    EqualEqual,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    AndAnd,
    OrOr,
    Bang,

    LeftParen,
    RightParen,
    Comma,

    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Int(v) => write!(f, "{v}"),
            Token::Float(v) => write!(f, "{v}"),
            Token::String(v) => write!(f, "\"{v}\""),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Null => write!(f, "null"),
            Token::Ident(v) => write!(f, "{v}"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::EqualEqual => write!(f, "=="),
            Token::BangEqual => write!(f, "!="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Eof => write!(f, "<eof>"),
        }
    }
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    token_start: Position,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            token_start: Position::new(1, 1),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        self.pos += 1;
        if let Some(c) = ch {
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn begin_token(&mut self) {
        self.token_start = Position::new(self.line, self.col);
    }

    fn read_string(&mut self) -> Result<Token, String> {
        let mut s = String::new();

        loop {
            match self.advance() {
                None => return Err("Unterminated string literal".to_string()),
                Some('"') => break,
                Some('\\') => match self.advance() {
                    None => return Err("Unterminated string escape".to_string()),
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(c) => {
                        s.push('\\');
                        s.push(c);
                    }
                },
                Some(ch) => s.push(ch),
            }
        }

        Ok(Token::String(s))
    }

    fn read_number(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);
        let mut is_float = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.advance();
            } else if ch == '.' {
                is_float = true;
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if s.ends_with('.') {
            is_float = true;
        }

        if is_float {
            let v: f64 = s.parse().unwrap_or(0.0);
            Token::Float(v)
        } else {
            let v: i64 = s.parse().unwrap_or(0);
            Token::Int(v)
        }
    }

    fn read_ident(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);

        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        match s.as_str() {
            "true" => Token::True,
            "false" => Token::False,
            "null" => Token::Null,
            _ => Token::Ident(s),
        }
    }

    pub fn last_token_start(&self) -> Position {
        self.token_start
    }

    pub fn next_token(&mut self) -> Result<(Token, Position), String> {
        self.skip_whitespace();
        self.begin_token();

        let Some(ch) = self.advance() else {
            return Ok((Token::Eof, self.token_start));
        };

        let token = match ch {
            '"' => self.read_string(),
            '0'..='9' => Ok(self.read_number(ch)),
            'a'..='z' | 'A'..='Z' | '_' => Ok(self.read_ident(ch)),

            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash),
            '%' => Ok(Token::Percent),

            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::EqualEqual)
                } else {
                    Err("Unexpected '=', did you mean '=='?".to_string())
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::BangEqual)
                } else {
                    Ok(Token::Bang)
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::GreaterEqual)
                } else {
                    Ok(Token::Greater)
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::LessEqual)
                } else {
                    Ok(Token::Less)
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    Ok(Token::AndAnd)
                } else {
                    Err("Unexpected '&', did you mean '&&'?".to_string())
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    Ok(Token::OrOr)
                } else {
                    Err("Unexpected '|', did you mean '||'?".to_string())
                }
            }

            '(' => Ok(Token::LeftParen),
            ')' => Ok(Token::RightParen),
            ',' => Ok(Token::Comma),

            other => Err(format!("Unexpected character '{other}'")),
        };

        token.map(|t| (t, self.token_start))
    }
}
