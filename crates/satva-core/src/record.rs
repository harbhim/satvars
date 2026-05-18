use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Record {
    pub fields: HashMap<String, Value>,
}
impl Record {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }
    pub fn insert(&mut self, key: &str, value: Value) {
        self.fields.insert(key.to_string(), value);
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.fields.remove(key)
    }
}
