use crate::Value;
use crate::record::Record;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Null,
    Int64,
    Float64,
    Boolean,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    name: String,
    data_type: DataType,
    nullable: bool,
}

impl Field {
    pub fn new(name: impl Into<String>, data_type: DataType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data_type(&self) -> DataType {
        self.data_type
    }

    pub fn is_nullable(&self) -> bool {
        self.nullable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    fields: Vec<Field>,
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|field| field.name() == name)
    }

    /// Infers the schema from a slice of records by inspecting their values.
    /// It counts type occurrences and selects the most common type for each field
    /// (majority voting), making it robust against dirty or corrupted cells.
    pub fn infer(records: &[Record]) -> Self {
        struct TypeStats {
            int_count: usize,
            float_count: usize,
            bool_count: usize,
            string_count: usize,
            nullable: bool,
        }

        let mut field_order = Vec::new();
        let mut field_stats: std::collections::HashMap<String, TypeStats> =
            std::collections::HashMap::new();

        for record in records {
            for (key, value) in &record.fields {
                if !field_stats.contains_key(key) {
                    field_order.push(key.clone());
                }

                let stats = field_stats.entry(key.clone()).or_insert(TypeStats {
                    int_count: 0,
                    float_count: 0,
                    bool_count: 0,
                    string_count: 0,
                    nullable: false,
                });

                match value {
                    Value::Null => stats.nullable = true,
                    Value::Int64(_) => stats.int_count += 1,
                    Value::Float64(_) => stats.float_count += 1,
                    Value::Boolean(_) => stats.bool_count += 1,
                    Value::String(s) => {
                        if s.is_empty() {
                            stats.nullable = true;
                        } else if s.parse::<i64>().is_ok() {
                            stats.int_count += 1;
                        } else if s.parse::<f64>().is_ok() {
                            stats.float_count += 1;
                        } else if s.parse::<bool>().is_ok() {
                            stats.bool_count += 1;
                        } else {
                            stats.string_count += 1;
                        }
                    }
                }
            }
        }

        // Detect missing fields in any records to determine nullability
        for record in records {
            for key in &field_order {
                #[expect(
                    clippy::collapsible_if,
                    reason = "avoid let_chains syntax which may not compile on older toolchains"
                )]
                if let Some(stats) = field_stats.get_mut(key) {
                    if !record.fields.contains_key(key) {
                        stats.nullable = true;
                    }
                }
            }
        }

        let mut fields = Vec::new();
        for name in field_order {
            if let Some(stats) = field_stats.remove(&name) {
                let total_non_null =
                    stats.int_count + stats.float_count + stats.bool_count + stats.string_count;
                let data_type = if total_non_null == 0 {
                    DataType::String
                } else if stats.int_count * 2 > total_non_null {
                    DataType::Int64
                } else if (stats.int_count + stats.float_count) * 2 > total_non_null {
                    DataType::Float64
                } else if stats.bool_count * 2 > total_non_null {
                    DataType::Boolean
                } else {
                    DataType::String
                };

                fields.push(Field::new(name, data_type, stats.nullable));
            }
        }

        fields.sort_by(|a, b| a.name().cmp(b.name()));
        Schema::new(fields)
    }
}
