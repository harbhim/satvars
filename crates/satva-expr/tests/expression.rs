use satva_expr::{BinaryOperator, Expression, UnaryOperator, field, lit};
use satva_types::Value;

#[test]
fn literal_expression() {
    let expr = lit(42);

    assert_eq!(expr, Expression::Literal(Value::Int64(42)));
}

#[test]
fn field_expression() {
    let expr = field("age");

    assert_eq!(expr, Expression::Field("age".to_string()));
}

#[test]
fn binary_expression() {
    let expr = field("age").greater_than(lit(18));

    assert_eq!(
        expr,
        Expression::Binary {
            left: Box::new(Expression::Field("age".to_string())),
            op: BinaryOperator::GreaterThan,
            right: Box::new(Expression::Literal(Value::Int64(18))),
        }
    );
}

#[test]
fn nested_expression() {
    let expr = field("salary").times(lit(2)).plus(lit(1000));

    assert_eq!(
        expr,
        Expression::Binary {
            left: Box::new(Expression::Binary {
                left: Box::new(Expression::Field("salary".to_string())),
                op: BinaryOperator::Multiply,
                right: Box::new(Expression::Literal(Value::Int64(2))),
            }),
            op: BinaryOperator::Add,
            right: Box::new(Expression::Literal(Value::Int64(1000))),
        }
    );
}

#[test]
fn unary_not_expression() {
    let expr = field("active").logical_not();

    assert_eq!(
        expr,
        Expression::Unary {
            op: UnaryOperator::Not,
            expr: Box::new(Expression::Field("active".to_string())),
        }
    );
}

#[test]
fn unary_negate_expression() {
    let expr = field("salary").negate();

    assert_eq!(
        expr,
        Expression::Unary {
            op: UnaryOperator::Negate,
            expr: Box::new(Expression::Field("salary".to_string())),
        }
    );
}

#[test]
fn boolean_expression_tree() {
    let expr = field("age")
        .greater_than(lit(18))
        .and(field("active").equal_to(lit(true)));

    assert_eq!(
        expr,
        Expression::Binary {
            left: Box::new(Expression::Binary {
                left: Box::new(Expression::Field("age".to_string())),
                op: BinaryOperator::GreaterThan,
                right: Box::new(Expression::Literal(Value::Int64(18))),
            }),
            op: BinaryOperator::And,
            right: Box::new(Expression::Binary {
                left: Box::new(Expression::Field("active".to_string())),
                op: BinaryOperator::Equal,
                right: Box::new(Expression::Literal(Value::Boolean(true))),
            }),
        }
    );
}
