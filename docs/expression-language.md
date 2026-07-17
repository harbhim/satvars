# Expression Language

The expression language is used in `filter` and `set_field` stages within YAML pipeline configs. It is parsed by the `satva-parser` crate into an AST that the `satva-expr` evaluator resolves against each record.

## Literals

| Syntax | Type | Example |
|---|---|---|
| `123` | Int64 | `42`, `-5`, `0` |
| `3.14` | Float64 | `2.5`, `-1.0` |
| `"hello"` | String | `"hello world"`, `""` |
| `true` / `false` | Boolean | `true`, `false` |
| `null` | Null | `null` |

String escape sequences: `\n`, `\t`, `\"`, `\\`.

## Field References

Any unquoted identifier is treated as a field name:

```
salary
first_name
department
```

## Operators

Listed in precedence order (highest to lowest):

| Precedence | Operators | Associativity | Description |
|---|---|---|---|
| 1 (highest) | `!` `-` | Right | Logical not, numeric negate |
| 2 | `*` `/` `%` | Left | Multiply, divide, modulo |
| 3 | `+` `-` | Left | Add, subtract, string concat |
| 4 | `==` `!=` `>` `>=` `<` `<=` | Left | Comparison |
| 5 | `&&` | Left | Logical AND (short-circuit) |
| 6 (lowest) | `\|\|` | Left | Logical OR (short-circuit) |

### Short-circuit Evaluation

`&&` and `||` use short-circuit evaluation:
- `false && ...` → returns `false` without evaluating the right side
- `true || ...` → returns `true` without evaluating the right side

This allows safe null checks:

```
is_not_null(salary) && salary >= 70000
```

If salary is null, `is_not_null(salary)` is `false`, and `salary >= 70000` is never evaluated — avoiding a type error.

### Null in Comparisons

Comparisons involving `Null` return `false`:

```
null > 5       → false
null == null   → true
null == 5      → false
null != 5      → true
```

## Functions

### String Functions

| Function | Example | Description |
|---|---|---|
| `upper(s)` | `upper(name)` | Convert to uppercase |
| `lower(s)` | `lower(name)` | Convert to lowercase |
| `trim(s)` | `trim(name)` | Strip leading/trailing whitespace |
| `length(s)` | `length(name)` | Character count |
| `concat(a, b, ...)` | `concat(first, " ", last)` | String concatenation |

### Null Functions

| Function | Example | Description |
|---|---|---|
| `coalesce(a, b, ...)` | `coalesce(middle_name, "N/A")` | First non-null value |
| `is_null(v)` | `is_null(email)` | True if value is null |
| `is_not_null(v)` | `is_not_null(salary)` | True if value is not null |

### Cast Functions

| Function | Example | Description |
|---|---|---|
| `cast_int(v)` | `cast_int(salary)` | Cast to Int64 |
| `cast_float(v)` | `cast_float(age)` | Cast to Float64 |
| `cast_bool(v)` | `cast_bool(active)` | Cast to Boolean |
| `cast_string(v)` | `cast_string(id)` | Cast to String |

## Examples

```
# Comparison
age >= 18
department == "Engineering"
salary > 50000 && department == "HR"

# Null-safe filter
is_not_null(salary) && is_not_null(active) && active == true && salary >= 70000

# Arithmetic
salary * 0.15
(price - discount) * 1.1 + tax

# String manipulation
upper(first_name) + " " + upper(last_name)
trim(upper(concat(first_name, " ", last_name)))

# Functions
coalesce(bonus, 0.0)
length(trim(name)) > 0
cast_int(salary) > 100000

# Parentheses
(active == true || active == null) && salary >= 50000
```
