use satva_expr::{BinaryOperator, Expression, Function, UnaryOperator};
use satva_types::Value;

use crate::error::ParseError;
use crate::lexer::{Lexer, Position, Token};

pub struct Parser {
    lexer: Lexer,
    current: Token,
    current_pos: Position,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let (current, current_pos) = lexer
            .next_token()
            .unwrap_or((Token::Eof, Position::new(1, 1)));
        Self {
            lexer,
            current,
            current_pos,
        }
    }

    fn pos(&self) -> Position {
        self.current_pos
    }

    pub fn parse(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_or()?;
        if self.current != Token::Eof {
            return Err(ParseError::unexpected(
                &self.current,
                "end of expression",
                Some(self.pos()),
            ));
        }
        Ok(expr)
    }

    fn advance(&mut self) -> Result<(), ParseError> {
        match self.lexer.next_token() {
            Ok((token, pos)) => {
                self.current = token;
                self.current_pos = pos;
                Ok(())
            }
            Err(msg) => {
                let pos = Some(self.lexer.last_token_start());
                self.current = Token::Eof;
                Err(ParseError::lexical(msg, pos))
            }
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        if self.current == *expected {
            self.advance()
        } else {
            Err(ParseError::expected(
                expected,
                &self.current,
                Some(self.pos()),
            ))
        }
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;
        while self.current == Token::OrOr {
            self.advance()?;
            let right = self.parse_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;
        while self.current == Token::AndAnd {
            self.advance()?;
            let right = self.parse_comparison()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_term()?;

        let op = match &self.current {
            Token::EqualEqual => BinaryOperator::Equal,
            Token::BangEqual => BinaryOperator::NotEqual,
            Token::Greater => BinaryOperator::GreaterThan,
            Token::GreaterEqual => BinaryOperator::GreaterThanOrEqual,
            Token::Less => BinaryOperator::LessThan,
            Token::LessEqual => BinaryOperator::LessThanOrEqual,
            _ => return Ok(left),
        };

        self.advance()?;
        let right = self.parse_term()?;

        Ok(Expression::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_factor()?;

        while matches!(&self.current, Token::Plus | Token::Minus) {
            let op = match &self.current {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Subtract,
                _ => unreachable!(),
            };
            self.advance()?;
            let right = self.parse_factor()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;

        while matches!(&self.current, Token::Star | Token::Slash | Token::Percent) {
            let op = match &self.current {
                Token::Star => BinaryOperator::Multiply,
                Token::Slash => BinaryOperator::Divide,
                Token::Percent => BinaryOperator::Modulo,
                _ => unreachable!(),
            };
            self.advance()?;
            let right = self.parse_unary()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if self.current == Token::Bang {
            self.advance()?;
            let expr = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOperator::Not,
                expr: Box::new(expr),
            });
        }

        if self.current == Token::Minus {
            self.advance()?;
            let expr = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOperator::Negate,
                expr: Box::new(expr),
            });
        }

        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expression, ParseError> {
        let primary = self.parse_primary()?;

        if self.current == Token::LeftParen {
            let name = match &primary {
                Expression::Field(name) => name.clone(),
                _ => {
                    return Err(ParseError::expected(
                        &Token::LeftParen,
                        &self.current,
                        Some(self.pos()),
                    ));
                }
            };

            self.advance()?;

            let mut arguments = Vec::new();

            if self.current != Token::RightParen {
                arguments.push(self.parse_or()?);
                while self.current == Token::Comma {
                    self.advance()?;
                    arguments.push(self.parse_or()?);
                }
            }

            self.expect(&Token::RightParen)?;

            let function = match name.to_lowercase().as_str() {
                "upper" => Function::Upper,
                "lower" => Function::Lower,
                "trim" => Function::Trim,
                "length" => Function::Length,
                "concat" => Function::Concat,
                "coalesce" => Function::Coalesce,
                "is_null" => Function::IsNull,
                "is_not_null" => Function::IsNotNull,
                "cast_int" => Function::CastInt,
                "cast_float" => Function::CastFloat,
                "cast_bool" => Function::CastBool,
                "cast_string" => Function::CastString,
                _ => {
                    return Err(ParseError::unknown_function(name, Some(self.pos())));
                }
            };

            return Ok(Expression::Function {
                function,
                arguments,
            });
        }

        Ok(primary)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let token = self.current.clone();
        match token {
            Token::Int(v) => {
                self.advance()?;
                Ok(Expression::Literal(Value::Int64(v)))
            }
            Token::Float(v) => {
                self.advance()?;
                Ok(Expression::Literal(Value::Float64(v)))
            }
            Token::String(s) => {
                self.advance()?;
                Ok(Expression::Literal(Value::String(s)))
            }
            Token::True => {
                self.advance()?;
                Ok(Expression::Literal(Value::Boolean(true)))
            }
            Token::False => {
                self.advance()?;
                Ok(Expression::Literal(Value::Boolean(false)))
            }
            Token::Null => {
                self.advance()?;
                Ok(Expression::Literal(Value::Null))
            }
            Token::Ident(name) => {
                self.advance()?;
                Ok(Expression::Field(name))
            }
            Token::LeftParen => {
                self.advance()?;
                let expr = self.parse_or()?;
                self.expect(&Token::RightParen)?;
                Ok(expr)
            }
            other => Err(ParseError::expected(
                &Token::Ident("expression".to_string()),
                &other,
                Some(self.pos()),
            )),
        }
    }
}

pub fn parse_expression(input: &str) -> Result<Expression, ParseError> {
    let mut parser = Parser::new(input);
    parser.parse()
}
