//! SObject record representation.

use std::collections::HashMap;

/// A Salesforce SObject record as a bag of field→value pairs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SObjectRecord {
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

impl SObjectRecord {
    pub fn new() -> Self {
        Self { fields: HashMap::new() }
    }

    pub fn field(mut self, name: &str, value: impl Into<serde_json::Value>) -> Self {
        self.fields.insert(name.to_string(), value.into());
        self
    }

    pub fn id(&self) -> Option<&str> {
        self.fields.get("Id").or(self.fields.get("id")).and_then(|v| v.as_str())
    }
}

impl Default for SObjectRecord {
    fn default() -> Self {
        Self::new()
    }
}
