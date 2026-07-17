use satva_expr::{BinaryOperator, Expression, Function, UnaryOperator};
use satva_parser::parse_expression;
use satva_types::Value;

fn lit_i(v: i64) -> Expression {
    Expression::Literal(Value::Int64(v))
}

fn lit_f(v: f64) -> Expression {
    Expression::Literal(Value::Float64(v))
}

fn lit_s(v: &str) -> Expression {
    Expression::Literal(Value::String(v.to_string()))
}

fn lit_b(v: bool) -> Expression {
    Expression::Literal(Value::Boolean(v))
}

fn lit_null() -> Expression {
    Expression::Literal(Value::Null)
}

fn field(name: &str) -> Expression {
    Expression::Field(name.to_string())
}

fn bin(left: Expression, op: BinaryOperator, right: Expression) -> Expression {
    Expression::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

fn unary(op: UnaryOperator, expr: Expression) -> Expression {
    Expression::Unary {
        op,
        expr: Box::new(expr),
    }
}

fn func(function: Function, arguments: Vec<Expression>) -> Expression {
    Expression::Function {
        function,
        arguments,
    }
}

// --- Literals ---

#[test]
fn parse_int_literal() {
    assert_eq!(parse_expression("42").unwrap(), lit_i(42));
    assert_eq!(parse_expression("0").unwrap(), lit_i(0));
    assert_eq!(parse_expression("-5").unwrap(), unary(UnaryOperator::Negate, lit_i(5)));
}

#[test]
fn parse_float_literal() {
    assert_eq!(parse_expression("2.5").unwrap(), lit_f(2.5));
    assert_eq!(parse_expression("-2.5").unwrap(), unary(UnaryOperator::Negate, lit_f(2.5)));
}

#[test]
fn parse_string_literal() {
    assert_eq!(parse_expression(r#""hello""#).unwrap(), lit_s("hello"));
    assert_eq!(parse_expression(r#""""#).unwrap(), lit_s(""));
}

#[test]
fn parse_string_escape() {
    assert_eq!(
        parse_expression(r#""hello\nworld""#).unwrap(),
        lit_s("hello\nworld")
    );
    assert_eq!(
        parse_expression(r#""say \"hi\"""#).unwrap(),
        lit_s("say \"hi\"")
    );
}

#[test]
fn parse_boolean_literal() {
    assert_eq!(parse_expression("true").unwrap(), lit_b(true));
    assert_eq!(parse_expression("false").unwrap(), lit_b(false));
}

#[test]
fn parse_null() {
    assert_eq!(parse_expression("null").unwrap(), lit_null());
}

// --- Field references ---

#[test]
fn parse_field() {
    assert_eq!(parse_expression("salary").unwrap(), field("salary"));
    assert_eq!(parse_expression("first_name").unwrap(), field("first_name"));
    assert_eq!(parse_expression("_hidden").unwrap(), field("_hidden"));
}

// --- Binary operators: arithmetic ---

#[test]
fn parse_addition() {
    let expected = bin(field("a"), BinaryOperator::Add, lit_i(1));
    assert_eq!(parse_expression("a + 1").unwrap(), expected);
}

#[test]
fn parse_subtraction() {
    let expected = bin(field("x"), BinaryOperator::Subtract, lit_i(10));
    assert_eq!(parse_expression("x - 10").unwrap(), expected);
}

#[test]
fn parse_multiplication() {
    let expected = bin(field("price"), BinaryOperator::Multiply, lit_f(1.1));
    assert_eq!(parse_expression("price * 1.1").unwrap(), expected);
}

#[test]
fn parse_division() {
    let expected = bin(field("total"), BinaryOperator::Divide, lit_i(2));
    assert_eq!(parse_expression("total / 2").unwrap(), expected);
}

#[test]
fn parse_modulo() {
    let expected = bin(field("id"), BinaryOperator::Modulo, lit_i(10));
    assert_eq!(parse_expression("id % 10").unwrap(), expected);
}

// --- Operator precedence ---

#[test]
fn precedence_multiplication_over_addition() {
    let expected = bin(field("a"), BinaryOperator::Add, bin(field("b"), BinaryOperator::Multiply, lit_i(3)));
    assert_eq!(parse_expression("a + b * 3").unwrap(), expected);
}

#[test]
fn precedence_addition_over_comparison() {
    let expected = bin(
        bin(field("a"), BinaryOperator::Add, lit_i(1)),
        BinaryOperator::GreaterThan,
        lit_i(10),
    );
    assert_eq!(parse_expression("a + 1 > 10").unwrap(), expected);
}

#[test]
fn precedence_and_over_or() {
    let expected = bin(
        bin(field("a"), BinaryOperator::GreaterThan, lit_i(1)),
        BinaryOperator::Or,
        bin(field("b"), BinaryOperator::LessThan, lit_i(10)),
    );
    assert_eq!(parse_expression("a > 1 || b < 10").unwrap(), expected);
}

// --- Parentheses ---

#[test]
fn parentheses_override_precedence() {
    let expected = bin(
        bin(field("a"), BinaryOperator::Add, field("b")),
        BinaryOperator::Multiply,
        lit_i(3),
    );
    assert_eq!(parse_expression("(a + b) * 3").unwrap(), expected);
}

#[test]
fn nested_parentheses() {
    let expected = bin(
        bin(lit_i(1), BinaryOperator::Add, lit_i(2)),
        BinaryOperator::Multiply,
        bin(lit_i(3), BinaryOperator::Add, lit_i(4)),
    );
    assert_eq!(parse_expression("(1 + 2) * (3 + 4)").unwrap(), expected);
}

// --- Comparison operators ---

#[test]
fn parse_comparisons() {
    assert_eq!(
        parse_expression("a == 1").unwrap(),
        bin(field("a"), BinaryOperator::Equal, lit_i(1))
    );
    assert_eq!(
        parse_expression("a != 1").unwrap(),
        bin(field("a"), BinaryOperator::NotEqual, lit_i(1))
    );
    assert_eq!(
        parse_expression("a > 1").unwrap(),
        bin(field("a"), BinaryOperator::GreaterThan, lit_i(1))
    );
    assert_eq!(
        parse_expression("a >= 1").unwrap(),
        bin(field("a"), BinaryOperator::GreaterThanOrEqual, lit_i(1))
    );
    assert_eq!(
        parse_expression("a < 1").unwrap(),
        bin(field("a"), BinaryOperator::LessThan, lit_i(1))
    );
    assert_eq!(
        parse_expression("a <= 1").unwrap(),
        bin(field("a"), BinaryOperator::LessThanOrEqual, lit_i(1))
    );
}

// --- Logical operators ---

#[test]
fn parse_and_or() {
    let expected = bin(
        bin(field("a"), BinaryOperator::GreaterThan, lit_i(10)),
        BinaryOperator::And,
        bin(field("b"), BinaryOperator::Equal, lit_s("x")),
    );
    assert_eq!(parse_expression("a > 10 && b == \"x\"").unwrap(), expected);

    let expected = bin(
        field("flag"),
        BinaryOperator::Or,
        field("backup"),
    );
    assert_eq!(parse_expression("flag || backup").unwrap(), expected);
}

// --- Unary operators ---

#[test]
fn parse_logical_not() {
    assert_eq!(
        parse_expression("!flag").unwrap(),
        unary(UnaryOperator::Not, field("flag"))
    );
}

#[test]
fn parse_double_not() {
    assert_eq!(
        parse_expression("!!flag").unwrap(),
        unary(UnaryOperator::Not, unary(UnaryOperator::Not, field("flag")))
    );
}

#[test]
fn parse_negate() {
    assert_eq!(
        parse_expression("-x").unwrap(),
        unary(UnaryOperator::Negate, field("x"))
    );
}

// --- Functions ---

#[test]
fn parse_upper() {
    assert_eq!(
        parse_expression("upper(name)").unwrap(),
        func(Function::Upper, vec![field("name")])
    );
}

#[test]
fn parse_lower() {
    assert_eq!(
        parse_expression("lower(name)").unwrap(),
        func(Function::Lower, vec![field("name")])
    );
}

#[test]
fn parse_trim() {
    assert_eq!(
        parse_expression("trim(name)").unwrap(),
        func(Function::Trim, vec![field("name")])
    );
}

#[test]
fn parse_length() {
    assert_eq!(
        parse_expression("length(name)").unwrap(),
        func(Function::Length, vec![field("name")])
    );
}

#[test]
fn parse_concat() {
    assert_eq!(
        parse_expression("concat(first, \" \", last)").unwrap(),
        func(Function::Concat, vec![field("first"), lit_s(" "), field("last")])
    );
}

#[test]
fn parse_coalesce() {
    assert_eq!(
        parse_expression("coalesce(a, b, c)").unwrap(),
        func(Function::Coalesce, vec![field("a"), field("b"), field("c")])
    );
}

#[test]
fn parse_is_null() {
    assert_eq!(
        parse_expression("is_null(field)").unwrap(),
        func(Function::IsNull, vec![field("field")])
    );
}

#[test]
fn parse_is_not_null() {
    assert_eq!(
        parse_expression("is_not_null(field)").unwrap(),
        func(Function::IsNotNull, vec![field("field")])
    );
}

#[test]
fn parse_cast_functions() {
    assert_eq!(
        parse_expression("cast_int(x)").unwrap(),
        func(Function::CastInt, vec![field("x")])
    );
    assert_eq!(
        parse_expression("cast_float(x)").unwrap(),
        func(Function::CastFloat, vec![field("x")])
    );
    assert_eq!(
        parse_expression("cast_bool(x)").unwrap(),
        func(Function::CastBool, vec![field("x")])
    );
    assert_eq!(
        parse_expression("cast_string(x)").unwrap(),
        func(Function::CastString, vec![field("x")])
    );
}

// --- Complex expressions ---

#[test]
fn parse_complex_filter_expression() {
    let expr = parse_expression("salary > 50000 && department == \"HR\"").unwrap();
    let expected = bin(
        bin(field("salary"), BinaryOperator::GreaterThan, lit_i(50000)),
        BinaryOperator::And,
        bin(field("department"), BinaryOperator::Equal, lit_s("HR")),
    );
    assert_eq!(expr, expected);
}

#[test]
fn parse_expression_with_function_in_comparison() {
    let expr = parse_expression("length(name) > 0").unwrap();
    let expected = bin(
        func(Function::Length, vec![field("name")]),
        BinaryOperator::GreaterThan,
        lit_i(0),
    );
    assert_eq!(expr, expected);
}

#[test]
fn parse_nested_function_call() {
    let expr = parse_expression("trim(upper(name))").unwrap();
    let expected = func(
        Function::Trim,
        vec![func(Function::Upper, vec![field("name")])],
    );
    assert_eq!(expr, expected);
}

#[test]
fn parse_combined_arithmetic() {
    let expr = parse_expression("(price - discount) * 1.1").unwrap();
    let expected = bin(
        bin(field("price"), BinaryOperator::Subtract, field("discount")),
        BinaryOperator::Multiply,
        lit_f(1.1),
    );
    assert_eq!(expr, expected);
}

// --- Error cases ---

#[test]
fn error_unterminated_string() {
    assert!(parse_expression(r#""hello"#).is_err());
}

#[test]
fn error_unclosed_paren() {
    assert!(parse_expression("(1 + 2").is_err());
}

#[test]
fn error_unexpected_character() {
    assert!(parse_expression("1 @ 2").is_err());
}

#[test]
fn error_trailing_garbage() {
    assert!(parse_expression("1 + 2 )").is_err());
}

#[test]
fn error_unknown_function() {
    assert!(parse_expression("foo(x)").is_err());
}

#[test]
fn error_single_equals() {
    assert!(parse_expression("a = 1").is_err());
}

#[test]
fn error_single_ampersand() {
    assert!(parse_expression("a & b").is_err());
}

#[test]
fn error_single_pipe() {
    assert!(parse_expression("a | b").is_err());
}

// --- Position in errors ---

#[test]
fn error_includes_position() {
    // "1 + 2 )": trailing ')' at col 7 (1-indexed)
    let err = parse_expression("1 + 2 )").unwrap_err();
    assert!(err.position.is_some());
    let pos = err.position.unwrap();
    assert_eq!(pos.line, 1);
    assert_eq!(pos.col, 7);

    // "a = 1": '=' at col 3
    let err = parse_expression("a = 1").unwrap_err();
    assert!(err.position.is_some());
    let pos = err.position.unwrap();
    assert_eq!(pos.line, 1);
    assert_eq!(pos.col, 3);
}

#[test]
fn error_position_multiline() {
    // "a == 1\n+" - after '+', EOF at line 2, col 2
    let err = parse_expression("a == 1\n+").unwrap_err();
    assert!(err.position.is_some());
    let pos = err.position.unwrap();
    assert_eq!(pos.line, 2);
    assert_eq!(pos.col, 2);
}

#[test]
fn error_position_string_shows_message() {
    let err = parse_expression("a = 1").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("1:3"));
    assert!(msg.contains("expected"));
}

#[test]
fn error_position_on_unclosed_paren() {
    // "(1 + 2": missing ')' at end, error at EOF (len 6, col 7)
    let err = parse_expression("(1 + 2").unwrap_err();
    assert!(err.position.is_some());
    let pos = err.position.unwrap();
    assert_eq!(pos.line, 1);
    assert_eq!(pos.col, 7); // EOF at position 7
}

#[test]
fn error_position_on_unknown_function() {
    let err = parse_expression("foo(x)").unwrap_err();
    assert!(err.position.is_some());
    assert_eq!(err.position.unwrap().line, 1);
}
