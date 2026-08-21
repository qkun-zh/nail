use agdb::DbKeyValue;

use crate::error::Error;
use crate::kinds::NodeKind;
use crate::value::{Value, try_from_db_value};

/// Typed view of one node's data. Implemented by domain structs in `back`;
/// the crate stays generic over row content.
pub trait Row: Sized {
    const KIND: NodeKind;

    /// Domain identifier (`UUIDv7` string); becomes the alias
    /// `"{kind}:{business_id}"` under which the node is resolvable.
    fn business_id(&self) -> &str;

    /// Present fields only; keys absent here are cleared on upsert.
    fn to_row(&self) -> Vec<(String, Value)>;

    /// Rebuilds a row from a stored node's values.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] if a required key is missing or has the
    /// wrong value kind.
    fn from_lookup(lookup: &dyn ValueLookup) -> Result<Self, Error>;
}

/// Read accessor handed to [`Row::from_lookup`].
pub trait ValueLookup {
    /// Returns the stored value for `key`, if present.
    fn get(&self, key: &str) -> Option<Value>;

    /// Reads a required text key.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] if the key is absent or not text.
    fn required_text(&self, key: &str) -> Result<String, Error> {
        match self.get(key) {
            Some(Value::Text(text)) => Ok(text),
            Some(_) => Err(Error::Invalid(format!("key {key} is not text"))),
            None => Err(Error::Invalid(format!("missing key {key}"))),
        }
    }

    /// Reads an optional text key.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] if the key is present but not text.
    fn optional_text(&self, key: &str) -> Result<Option<String>, Error> {
        match self.get(key) {
            Some(Value::Text(text)) => Ok(Some(text)),
            Some(_) => Err(Error::Invalid(format!("key {key} is not text"))),
            None => Ok(None),
        }
    }

    /// Reads a required int key.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] if the key is absent or not an int.
    fn required_int(&self, key: &str) -> Result<i64, Error> {
        match self.get(key) {
            Some(Value::Int(int)) => Ok(int),
            Some(_) => Err(Error::Invalid(format!("key {key} is not an int"))),
            None => Err(Error::Invalid(format!("missing key {key}"))),
        }
    }
}

pub(crate) struct ElementLookup<'a> {
    values: &'a [DbKeyValue],
}

impl<'a> ElementLookup<'a> {
    pub(crate) fn new(values: &'a [DbKeyValue]) -> Self {
        Self { values }
    }
}

impl ValueLookup for ElementLookup<'_> {
    fn get(&self, key: &str) -> Option<Value> {
        self.values
            .iter()
            .find(|pair| pair.key.string().is_ok_and(|name| name == key))
            .and_then(|pair| try_from_db_value(&pair.value))
    }
}
