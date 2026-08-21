use std::convert::TryFrom;

use agdb::DbValue;

/// Storage-neutral value mirror. Only the value kinds nail uses cross the
/// seam; anything else is a storage-level concern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Text(String),
}

impl From<Value> for DbValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Int(int) => Self::I64(int),
            Value::Text(text) => Self::String(text),
        }
    }
}

pub(crate) fn try_from_db_value(value: &DbValue) -> Option<Value> {
    match value {
        DbValue::I64(int) => Some(Value::Int(*int)),
        DbValue::String(text) => Some(Value::Text(text.clone())),
        _ => None,
    }
}

impl TryFrom<Value> for i64 {
    type Error = Value;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int(int) => Ok(int),
            Value::Text(text) => Err(Value::Text(text)),
        }
    }
}

impl TryFrom<Value> for String {
    type Error = Value;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Text(text) => Ok(text),
            Value::Int(int) => Err(Value::Int(int)),
        }
    }
}
