#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    // String
    Upper,
    Lower,
    Trim,
    Length,
    Concat,

    // Null
    Coalesce,
    IsNull,
    IsNotNull,

    // Casts
    CastInt,
    CastFloat,
    CastBool,
    CastString,
}
