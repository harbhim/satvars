use satva_core::Value;

/// A logical expression tree.
///
/// Expressions are immutable. Builder methods create new expression
/// trees rather than modifying existing ones.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),

    Field(String),

    Unary {
        op: UnaryOperator,
        expr: Box<Expression>,
    },

    Binary {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,

    // Boolean
    And,
    Or,
}

/// Creates a field reference.
///
/// Example:
///
/// ```ignore
/// let expr = field("salary");
/// ```
pub fn field(name: impl Into<String>) -> Expression {
    Expression::Field(name.into())
}

/// Creates a literal value.
///
/// Example:
///
/// ```ignore
/// let expr = lit(42);
/// ```
pub fn lit<T>(value: T) -> Expression
where
    T: Into<Value>,
{
    Expression::Literal(value.into())
}

impl Expression {
    fn binary(self, op: BinaryOperator, rhs: Expression) -> Self {
        Self::Binary {
            left: Box::new(self),
            op,
            right: Box::new(rhs),
        }
    }

    fn unary(self, op: UnaryOperator) -> Self {
        Self::Unary {
            op,
            expr: Box::new(self),
        }
    }

    // Arithmetic

    pub fn add(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Add, rhs)
    }

    pub fn sub(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Subtract, rhs)
    }

    pub fn mul(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Multiply, rhs)
    }

    pub fn div(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Divide, rhs)
    }

    pub fn modulo(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Modulo, rhs)
    }

    // Comparison

    pub fn eq(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Equal, rhs)
    }

    pub fn ne(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::NotEqual, rhs)
    }

    pub fn gt(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::GreaterThan, rhs)
    }

    pub fn ge(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::GreaterThanOrEqual, rhs)
    }

    pub fn lt(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::LessThan, rhs)
    }

    pub fn le(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::LessThanOrEqual, rhs)
    }

    // Boolean

    pub fn and(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::And, rhs)
    }

    pub fn or(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Or, rhs)
    }

    // Unary

    pub fn not(self) -> Self {
        self.unary(UnaryOperator::Not)
    }

    pub fn neg(self) -> Self {
        self.unary(UnaryOperator::Negate)
    }
}
