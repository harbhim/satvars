use satva_types::Value;

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

    pub fn plus(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Add, rhs)
    }

    pub fn minus(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Subtract, rhs)
    }

    pub fn times(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Multiply, rhs)
    }

    pub fn divide_by(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Divide, rhs)
    }

    pub fn modulo(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Modulo, rhs)
    }

    // Comparison

    pub fn equal_to(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::Equal, rhs)
    }

    pub fn not_equal_to(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::NotEqual, rhs)
    }

    pub fn greater_than(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::GreaterThan, rhs)
    }

    pub fn greater_than_or_equal_to(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::GreaterThanOrEqual, rhs)
    }

    pub fn less_than(self, rhs: Expression) -> Self {
        self.binary(BinaryOperator::LessThan, rhs)
    }

    pub fn less_than_or_equal_to(self, rhs: Expression) -> Self {
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

    pub fn logical_not(self) -> Self {
        self.unary(UnaryOperator::Not)
    }

    pub fn negate(self) -> Self {
        self.unary(UnaryOperator::Negate)
    }
}
