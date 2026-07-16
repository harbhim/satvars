use crate::Value;
use indexmap::IndexMap;

use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct Record {
    pub fields: IndexMap<String, Value>,
}
impl Default for Record {
    fn default() -> Self {
        Self::new()
    }
}

impl Record {
    pub fn new() -> Self {
        Self {
            fields: IndexMap::new(),
        }
    }
    pub fn insert(&mut self, key: &str, value: Value) {
        self.fields.insert(key.to_string(), value);
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.fields.shift_remove(key)
    }
    pub fn require_string(&self, key: &str) -> Result<&str> {
        let value = self
            .get(key)
            .ok_or_else(|| anyhow!("Missing field: {key}"))?;

        value
            .as_string()
            .ok_or_else(|| anyhow!("Field '{key}' is not a string"))
    }
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
    pub fn len(&self) -> usize {
        self.fields.len()
    }
    pub fn contains_key(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }
    pub fn keys(&self) -> indexmap::map::Keys<'_, String, Value> {
        self.fields.keys()
    }
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Value) -> bool,
    {
        self.fields.retain(f);
    }
    pub fn iter(&self) -> indexmap::map::Iter<'_, String, Value> {
        self.fields.iter()
    }
}

impl<'a> IntoIterator for &'a Record {
    type Item = (&'a String, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.iter()
    }
}
